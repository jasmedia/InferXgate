use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use url::Url;

use crate::error::{ApiError, ApiResult};

use super::{OAuthProvider, OAuthTokens, OAuthUserInfo};

const GITHUB_AUTH_URL: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_USER_API_URL: &str = "https://api.github.com/user";

#[derive(Clone)]
pub struct GitHubOAuthProvider {
    client_id: String,
    client_secret: String,
    http_client: Arc<Client>,
}

#[derive(Debug, Serialize)]
struct TokenRequest {
    client_id: String,
    client_secret: String,
    code: String,
    redirect_uri: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct GitHubUser {
    id: i64,
    login: String,
    email: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

impl GitHubOAuthProvider {
    pub fn new(client_id: String, client_secret: String) -> Self {
        let client = Client::builder()
            // Connection pool settings
            .pool_max_idle_per_host(5) // OAuth calls are less frequent, keep 5 connections
            .pool_idle_timeout(Duration::from_secs(60))
            // Timeout settings
            .timeout(Duration::from_secs(30)) // OAuth calls should be faster
            .connect_timeout(Duration::from_secs(10))
            // TCP settings
            .tcp_keepalive(Duration::from_secs(60))
            .tcp_nodelay(true)
            // Build the client
            .build()
            .expect("Failed to create HTTP client for GitHubOAuthProvider");

        Self {
            client_id,
            client_secret,
            http_client: Arc::new(client),
        }
    }
}

#[async_trait]
impl OAuthProvider for GitHubOAuthProvider {
    fn name(&self) -> &str {
        "github"
    }

    fn authorize_url(&self, state: &str, redirect_uri: &str) -> String {
        let mut url = Url::parse(GITHUB_AUTH_URL).unwrap();
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("scope", "read:user user:email")
            .append_pair("state", state);

        url.to_string()
    }

    async fn exchange_code(&self, code: &str, redirect_uri: &str) -> ApiResult<OAuthTokens> {
        let request_body = TokenRequest {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            code: code.to_string(),
            redirect_uri: redirect_uri.to_string(),
        };

        let response = self
            .http_client
            .post(GITHUB_TOKEN_URL)
            .header("Accept", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| ApiError::ExternalApiError(format!("Failed to exchange code: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ApiError::ExternalApiError(format!(
                "GitHub token exchange failed: {}",
                error_text
            )));
        }

        let token_response: TokenResponse = response.json().await.map_err(|e| {
            ApiError::ExternalApiError(format!("Failed to parse token response: {}", e))
        })?;

        Ok(OAuthTokens {
            access_token: token_response.access_token,
            refresh_token: None, // GitHub doesn't provide refresh tokens
            expires_in: None,    // GitHub tokens don't expire
        })
    }

    async fn get_user_info(&self, access_token: &str) -> ApiResult<OAuthUserInfo> {
        // Get user profile
        let user_response = self
            .http_client
            .get(GITHUB_USER_API_URL)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "llm-gateway")
            .send()
            .await
            .map_err(|e| ApiError::ExternalApiError(format!("Failed to get user info: {}", e)))?;

        if !user_response.status().is_success() {
            let error_text = user_response.text().await.unwrap_or_default();
            return Err(ApiError::ExternalApiError(format!(
                "GitHub user info failed: {}",
                error_text
            )));
        }

        let github_user: GitHubUser = user_response
            .json()
            .await
            .map_err(|e| ApiError::ExternalApiError(format!("Failed to parse user info: {}", e)))?;

        // Get user email if not in profile
        let email = if let Some(email) = github_user.email {
            email
        } else {
            // Fetch emails from emails endpoint
            let emails_response = self
                .http_client
                .get("https://api.github.com/user/emails")
                .header("Authorization", format!("Bearer {}", access_token))
                .header("User-Agent", "llm-gateway")
                .send()
                .await
                .map_err(|e| {
                    ApiError::ExternalApiError(format!("Failed to get user emails: {}", e))
                })?;

            if !emails_response.status().is_success() {
                return Err(ApiError::ExternalApiError(
                    "Failed to get user email from GitHub".to_string(),
                ));
            }

            let emails: Vec<GitHubEmail> = emails_response.json().await.map_err(|e| {
                ApiError::ExternalApiError(format!("Failed to parse emails: {}", e))
            })?;

            // Find primary verified email
            emails
                .into_iter()
                .find(|e| e.primary && e.verified)
                .map(|e| e.email)
                .ok_or_else(|| {
                    ApiError::ExternalApiError(
                        "No verified email found in GitHub account".to_string(),
                    )
                })?
        };

        Ok(OAuthUserInfo {
            provider_user_id: github_user.id.to_string(),
            email,
            username: Some(github_user.login),
            avatar_url: github_user.avatar_url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorize_url() {
        let provider =
            GitHubOAuthProvider::new("test_client_id".to_string(), "test_secret".to_string());

        let url = provider.authorize_url("random_state", "http://localhost:3000/callback");

        assert!(url.contains("client_id=test_client_id"));
        assert!(url.contains("state=random_state"));
        assert!(url.contains("redirect_uri=http"));
        assert!(url.contains("scope=read:user%20user:email"));
    }
}
