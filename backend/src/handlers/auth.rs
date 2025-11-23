use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Json,
};
use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    auth::{
        create_lookup_hash, generate_token, generate_virtual_key, get_key_prefix, hash_password,
        hash_token, hash_virtual_key, validate_master_key_format, verify_password, AuthUser,
        GitHubOAuthProvider, OAuthProvider,
    },
    error::{ApiError, ApiResult},
    models::{
        CreateVirtualKeyRequest, OAuthAccount, Session, User, VirtualKey, VirtualKeyResponse,
    },
    AppState,
};

// ============================================================================
// Registration and Login
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub username: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub username: Option<String>,
    pub role: String,
}

/// Register a new user with email and password
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<Json<AuthResponse>> {
    let pool = state
        .database
        .get_pool()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    // Validate email domain if configured
    if let Some(allowed_domains) = &state.config.allowed_email_domains {
        let email_domain = request
            .email
            .split('@')
            .nth(1)
            .ok_or_else(|| ApiError::BadRequest("Invalid email format".to_string()))?;

        if !allowed_domains.iter().any(|d| d == email_domain) {
            return Err(ApiError::BadRequest(format!(
                "Email domain '{}' is not allowed",
                email_domain
            )));
        }
    }

    // Hash password
    let password_hash = hash_password(&request.password)?;

    // Create user
    let user = User::create(
        pool,
        request.email,
        request.username,
        Some(password_hash),
        "user".to_string(),
    )
    .await?;

    // Generate JWT token
    let token = generate_token(
        user.id,
        user.email.clone(),
        user.role.clone(),
        &state.config.jwt_secret,
        state.config.jwt_expiry_hours,
    )?;

    // Create session
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + chrono::Duration::hours(state.config.jwt_expiry_hours);
    Session::create(pool, user.id, token_hash, expires_at).await?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse {
            id: user.id,
            email: user.email,
            username: user.username,
            role: user.role,
        },
    }))
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Login with email and password
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<AuthResponse>> {
    let pool = state
        .database
        .get_pool()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    // Find user by email
    let user = User::find_by_email(pool, &request.email)
        .await?
        .ok_or_else(|| ApiError::AuthenticationFailed)?;

    // Verify password
    let password_hash = user
        .password_hash
        .as_ref()
        .ok_or_else(|| ApiError::AuthenticationFailed)?;

    if !verify_password(&request.password, password_hash)? {
        return Err(ApiError::AuthenticationFailed);
    }

    // Generate JWT token
    let token = generate_token(
        user.id,
        user.email.clone(),
        user.role.clone(),
        &state.config.jwt_secret,
        state.config.jwt_expiry_hours,
    )?;

    // Create session
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + chrono::Duration::hours(state.config.jwt_expiry_hours);
    Session::create(pool, user.id, token_hash, expires_at).await?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse {
            id: user.id,
            email: user.email,
            username: user.username,
            role: user.role,
        },
    }))
}

/// Logout (invalidate session)
pub async fn logout(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> ApiResult<StatusCode> {
    let pool = state
        .database
        .get_pool()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    // Delete all sessions for the user
    Session::delete_by_user(pool, auth_user.user_id).await?;

    Ok(StatusCode::OK)
}

/// Get current user info
pub async fn get_current_user(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> ApiResult<Json<UserResponse>> {
    let pool = state
        .database
        .get_pool()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let user = User::find_by_id(pool, auth_user.user_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    Ok(Json(UserResponse {
        id: user.id,
        email: user.email,
        username: user.username,
        role: user.role,
    }))
}

// ============================================================================
// OAuth (GitHub)
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: String,
}

/// Initiate GitHub OAuth flow
pub async fn github_oauth_start(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<OAuthStartResponse>> {
    let github_client_id = state
        .config
        .github_client_id
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("GitHub OAuth not configured".to_string()))?;

    let github_client_secret = state
        .config
        .github_client_secret
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("GitHub OAuth not configured".to_string()))?;

    let provider = GitHubOAuthProvider::new(github_client_id.clone(), github_client_secret.clone());

    // Generate random state for CSRF protection
    let state_token = generate_virtual_key(); // Reuse key generation for random string

    let auth_url = provider.authorize_url(&state_token, &state.config.oauth_redirect_url);

    Ok(Json(OAuthStartResponse {
        auth_url,
        state: state_token,
    }))
}

#[derive(Debug, Serialize)]
pub struct OAuthStartResponse {
    pub auth_url: String,
    pub state: String,
}

/// Handle OAuth callback (all providers)
pub async fn oauth_callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<OAuthCallbackQuery>,
) -> ApiResult<impl IntoResponse> {
    let pool = state
        .database
        .get_pool()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    // For now, assume GitHub (can be extended with provider parameter)
    let github_client_id = state
        .config
        .github_client_id
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("GitHub OAuth not configured".to_string()))?;

    let github_client_secret = state
        .config
        .github_client_secret
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("GitHub OAuth not configured".to_string()))?;

    let provider = GitHubOAuthProvider::new(github_client_id.clone(), github_client_secret.clone());

    // Exchange code for tokens
    let tokens = provider
        .exchange_code(&query.code, &state.config.oauth_redirect_url)
        .await?;

    // Get user info from provider
    let oauth_user_info = provider.get_user_info(&tokens.access_token).await?;

    // Validate email domain if configured
    if let Some(allowed_domains) = &state.config.allowed_email_domains {
        let email_domain = oauth_user_info
            .email
            .split('@')
            .nth(1)
            .ok_or_else(|| ApiError::BadRequest("Invalid email format".to_string()))?;

        if !allowed_domains.iter().any(|d| d == email_domain) {
            return Err(ApiError::BadRequest(format!(
                "Email domain '{}' is not allowed",
                email_domain
            )));
        }
    }

    // Check if OAuth account exists
    let oauth_account =
        OAuthAccount::find_by_provider(pool, provider.name(), &oauth_user_info.provider_user_id)
            .await?;

    let user = if let Some(account) = oauth_account {
        // Existing user - update OAuth account
        OAuthAccount::upsert(
            pool,
            account.user_id,
            provider.name().to_string(),
            oauth_user_info.provider_user_id.clone(),
            oauth_user_info.username.clone(),
            Some(tokens.access_token.clone()),
            tokens.refresh_token.clone(),
            None,
        )
        .await?;

        User::find_by_id(pool, account.user_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?
    } else {
        // New user - create user and OAuth account
        let user = User::create(
            pool,
            oauth_user_info.email.clone(),
            oauth_user_info.username.clone(),
            None, // No password for OAuth users
            "user".to_string(),
        )
        .await?;

        OAuthAccount::upsert(
            pool,
            user.id,
            provider.name().to_string(),
            oauth_user_info.provider_user_id,
            oauth_user_info.username,
            Some(tokens.access_token),
            tokens.refresh_token,
            None,
        )
        .await?;

        user
    };

    // Generate JWT token
    let token = generate_token(
        user.id,
        user.email.clone(),
        user.role.clone(),
        &state.config.jwt_secret,
        state.config.jwt_expiry_hours,
    )?;

    // Create session
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + chrono::Duration::hours(state.config.jwt_expiry_hours);
    Session::create(pool, user.id, token_hash, expires_at).await?;

    // Encode user data as JSON for passing to frontend
    let user_data = serde_json::to_string(&UserResponse {
        id: user.id,
        email: user.email,
        username: user.username,
        role: user.role,
    })
    .map_err(|e| ApiError::InternalError(format!("Failed to serialize user data: {}", e)))?;

    // Redirect to frontend with token and user data as URL parameters
    // Using base64 encoding for cleaner URLs and to avoid URL encoding issues
    let user_data_encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&user_data);

    let redirect_url = format!(
        "http://localhost:5173/auth/oauth/callback?token={}&user={}",
        token, user_data_encoded
    );

    Ok(Redirect::to(&redirect_url))
}

// ============================================================================
// Virtual Keys (API Keys)
// ============================================================================

/// Generate a new virtual key (requires master key or JWT)
pub async fn generate_key(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(request): Json<CreateVirtualKeyRequest>,
) -> ApiResult<Json<VirtualKeyResponse>> {
    let pool = state
        .database
        .get_pool()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    // Generate new key
    let key = generate_virtual_key();
    let key_hash = hash_virtual_key(&key)?;
    let key_lookup_hash = create_lookup_hash(&key);
    let key_prefix = get_key_prefix(&key);

    // Determine user_id (master key creates system keys, JWT creates user keys)
    let user_id = match auth_user.auth_type {
        crate::auth::AuthType::MasterKey => None, // System key
        _ => Some(auth_user.user_id),
    };

    let virtual_key = VirtualKey::create(
        pool,
        key_hash,
        key_lookup_hash,
        key_prefix.clone(),
        user_id,
        request.name,
        request.max_budget,
        request.rate_limit_rpm,
        request.rate_limit_tpm,
        request.allowed_models,
        request.expires_at,
    )
    .await?;

    Ok(Json(VirtualKeyResponse {
        id: virtual_key.id,
        key, // Only returned on creation
        key_prefix,
        name: virtual_key.name,
        max_budget: virtual_key.max_budget,
        current_spend: virtual_key.current_spend,
        rate_limit_rpm: virtual_key.rate_limit_rpm,
        rate_limit_tpm: virtual_key.rate_limit_tpm,
        allowed_models: virtual_key.allowed_models,
        expires_at: virtual_key.expires_at,
        blocked: virtual_key.blocked,
        created_at: virtual_key.created_at,
    }))
}

/// Get user's virtual keys
pub async fn get_user_keys(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
) -> ApiResult<Json<Vec<VirtualKey>>> {
    let pool = state
        .database
        .get_pool()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let keys = VirtualKey::find_by_user(pool, auth_user.user_id).await?;

    Ok(Json(keys))
}

#[derive(Debug, Deserialize)]
pub struct KeyIdPath {
    pub key_id: Uuid,
}

/// Get key info (requires master key)
pub async fn get_key_info(
    State(state): State<Arc<AppState>>,
    Query(query): Query<KeyIdPath>,
) -> ApiResult<Json<VirtualKey>> {
    let pool = state
        .database
        .get_pool()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    let key = VirtualKey::find_by_id(pool, query.key_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Key not found".to_string()))?;

    Ok(Json(key))
}

#[derive(Debug, Deserialize)]
pub struct UpdateKeyRequest {
    pub key_id: Uuid,
    pub name: Option<String>,
    pub max_budget: Option<f64>,
    pub rate_limit_rpm: Option<i32>,
    pub rate_limit_tpm: Option<i32>,
    pub allowed_models: Option<Vec<String>>,
    pub blocked: Option<bool>,
}

/// Update virtual key (requires master key or key owner)
pub async fn update_key(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Json(request): Json<UpdateKeyRequest>,
) -> ApiResult<Json<VirtualKey>> {
    let pool = state
        .database
        .get_pool()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    // Verify ownership if not master key
    if !matches!(auth_user.auth_type, crate::auth::AuthType::MasterKey) {
        let key = VirtualKey::find_by_id(pool, request.key_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Key not found".to_string()))?;

        if key.user_id != Some(auth_user.user_id) {
            return Err(ApiError::Forbidden);
        }
    }

    let updated_key = VirtualKey::update(
        pool,
        request.key_id,
        request.name,
        request.max_budget,
        request.rate_limit_rpm,
        request.rate_limit_tpm,
        request.allowed_models,
        None,
        request.blocked,
    )
    .await?;

    Ok(Json(updated_key))
}

/// Delete virtual key (requires master key or key owner)
pub async fn delete_key(
    State(state): State<Arc<AppState>>,
    auth_user: AuthUser,
    Query(query): Query<KeyIdPath>,
) -> ApiResult<StatusCode> {
    let pool = state
        .database
        .get_pool()
        .ok_or_else(|| ApiError::DatabaseError("Database not available".to_string()))?;

    // Verify ownership if not master key
    if !matches!(auth_user.auth_type, crate::auth::AuthType::MasterKey) {
        let key = VirtualKey::find_by_id(pool, query.key_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Key not found".to_string()))?;

        if key.user_id != Some(auth_user.user_id) {
            return Err(ApiError::Forbidden);
        }
    }

    VirtualKey::delete(pool, query.key_id).await?;

    Ok(StatusCode::OK)
}
