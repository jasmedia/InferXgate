use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::ApiResult;

/// OAuth user information returned by providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUserInfo {
    pub provider_user_id: String,
    pub email: String,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
}

/// OAuth tokens returned after authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
}

/// Trait for OAuth provider implementations
/// Allows easy addition of new OAuth providers (Google, Microsoft, etc.)
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    /// Provider name (e.g., "github", "google", "microsoft")
    fn name(&self) -> &str;

    /// Generate the authorization URL to redirect users to
    fn authorize_url(&self, state: &str, redirect_uri: &str) -> String;

    /// Exchange authorization code for access tokens
    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> ApiResult<OAuthTokens>;

    /// Get user information from the provider using access token
    async fn get_user_info(&self, access_token: &str) -> ApiResult<OAuthUserInfo>;
}
