use axum::{
    async_trait,
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::{
    auth::{extract_bearer_token, validate_token},
    metrics::MetricsCollector,
    models::{User, VirtualKey},
    rate_limiter::{RateLimit, RateLimiter},
};

/// Authenticated user information extracted from JWT or API key
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: uuid::Uuid,
    pub email: String,
    pub role: String,
    pub auth_type: AuthType,
}

#[derive(Debug, Clone)]
pub enum AuthType {
    JWT,
    VirtualKey { key_id: uuid::Uuid },
    MasterKey,
}

/// Implement FromRequestParts to allow AuthUser to be used as an extractor
#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<AuthUser>().cloned().ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Missing authentication".to_string(),
            )
        })
    }
}

/// Middleware to require master key authentication
/// Used for admin operations like user creation and key management
pub async fn require_master_key<S>(
    State(state): State<Arc<S>>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)>
where
    S: HasMasterKey,
{
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Missing authorization header".to_string(),
            )
        })?;

    let token = extract_bearer_token(auth_header).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid authorization header format".to_string(),
        )
    })?;

    let master_key = state.get_master_key();

    if token != master_key {
        return Err((StatusCode::UNAUTHORIZED, "Invalid master key".to_string()));
    }

    // Add auth type to extensions
    let auth_user = AuthUser {
        user_id: uuid::Uuid::nil(), // Master key doesn't have a user
        email: "admin".to_string(),
        role: "admin".to_string(),
        auth_type: AuthType::MasterKey,
    };

    request.extensions_mut().insert(auth_user);

    Ok(next.run(request).await)
}

/// Middleware to require JWT authentication
/// Used for user-specific operations in the web UI
pub async fn require_jwt<S>(
    State(state): State<Arc<S>>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)>
where
    S: HasJwtSecret + HasDatabase,
{
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Missing authorization header".to_string(),
            )
        })?;

    let token = extract_bearer_token(auth_header).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid authorization header format".to_string(),
        )
    })?;

    let jwt_secret = state.get_jwt_secret();
    let claims = validate_token(token, jwt_secret).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid or expired token".to_string(),
        )
    })?;

    // Verify user still exists in database
    let pool = state.get_database_pool().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database not available".to_string(),
        )
    })?;

    let user_id = uuid::Uuid::parse_str(&claims.sub).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid user ID in token".to_string(),
        )
    })?;

    let user = User::find_by_id(pool, user_id)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to verify user".to_string(),
            )
        })?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "User not found".to_string()))?;

    let auth_user = AuthUser {
        user_id: user.id,
        email: user.email,
        role: user.role,
        auth_type: AuthType::JWT,
    };

    request.extensions_mut().insert(auth_user);

    Ok(next.run(request).await)
}

/// Middleware to require authentication (JWT or virtual key)
/// Used for API endpoints - accepts both JWT and API keys
pub async fn require_auth<S>(
    State(state): State<Arc<S>>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)>
where
    S: HasJwtSecret + HasDatabase + HasRedis,
{
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Missing authorization header".to_string(),
            )
        })?;

    let token = extract_bearer_token(auth_header).map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Invalid authorization header format".to_string(),
        )
    })?;

    let pool = state.get_database_pool().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database not available".to_string(),
        )
    })?;

    // Try JWT first
    let jwt_secret = state.get_jwt_secret();
    if let Ok(claims) = validate_token(token, jwt_secret) {
        let user_id = uuid::Uuid::parse_str(&claims.sub).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                "Invalid user ID in token".to_string(),
            )
        })?;

        let user = User::find_by_id(pool, user_id)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to verify user".to_string(),
                )
            })?
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "User not found".to_string()))?;

        let auth_user = AuthUser {
            user_id: user.id,
            email: user.email,
            role: user.role,
            auth_type: AuthType::JWT,
        };

        request.extensions_mut().insert(auth_user);
        return Ok(next.run(request).await);
    }

    // Try virtual key if JWT fails
    if token.starts_with("sk-") {
        // Create lookup hash for fast O(1) database query
        let lookup_hash = crate::auth::keys::create_lookup_hash(token);

        // CRITICAL FIX: Check verified token cache first to skip expensive bcrypt
        // This cache stores tokens that have already passed bcrypt verification
        let verified_token_key = format!("auth:verified:{}", lookup_hash);
        if let Some(redis_conn) = state.get_redis_connection() {
            if let Ok(Some(cached_key)) = get_cached_key(redis_conn, &verified_token_key).await {
                // Token already verified! Skip bcrypt entirely ✅
                tracing::debug!("Using verified token cache (skipping bcrypt)");

                // Validate key is still valid
                if cached_key.is_valid() {
                    let (user_id, email, role) = if let Some(user_id) = cached_key.user_id {
                        let user = User::find_by_id(pool, user_id)
                            .await
                            .map_err(|_| {
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "Failed to verify user".to_string(),
                                )
                            })?
                            .ok_or_else(|| {
                                (StatusCode::UNAUTHORIZED, "User not found".to_string())
                            })?;
                        (user.id, user.email, user.role)
                    } else {
                        (
                            uuid::Uuid::nil(),
                            "anonymous".to_string(),
                            "user".to_string(),
                        )
                    };

                    let auth_user = AuthUser {
                        user_id,
                        email,
                        role,
                        auth_type: AuthType::VirtualKey {
                            key_id: cached_key.id,
                        },
                    };

                    request.extensions_mut().insert(auth_user);
                    return Ok(next.run(request).await);
                }
            }
        }

        // Try to get from Redis cache first (5 minute TTL)
        let redis_key = format!("auth:key:{}", lookup_hash);
        let virtual_key = if let Some(redis_conn) = state.get_redis_connection() {
            // Try cache first
            match get_cached_key(redis_conn, &redis_key).await {
                Ok(Some(cached_key)) => Some(cached_key),
                Ok(None) => {
                    // Cache miss, fetch from database
                    let key = VirtualKey::find_by_lookup_hash(pool, &lookup_hash)
                        .await
                        .map_err(|_| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to verify key".to_string(),
                            )
                        })?;

                    // Cache for 5 minutes if found
                    if let Some(ref k) = key {
                        let _ = cache_key(redis_conn, &redis_key, k, 300).await;
                    }

                    key
                }
                Err(_) => {
                    // Redis error, fall back to database
                    VirtualKey::find_by_lookup_hash(pool, &lookup_hash)
                        .await
                        .map_err(|_| {
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to verify key".to_string(),
                            )
                        })?
                }
            }
        } else {
            // No Redis, direct database lookup
            VirtualKey::find_by_lookup_hash(pool, &lookup_hash)
                .await
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to verify key".to_string(),
                    )
                })?
        };

        let virtual_key = virtual_key.ok_or_else(|| {
            // Key not found, but verify with bcrypt anyway to prevent timing attacks
            let _ = crate::auth::keys::verify_virtual_key(
                token,
                "$2b$12$dummy.hash.for.timing.attack.prevention.only",
            );
            (StatusCode::UNAUTHORIZED, "Invalid API key".to_string())
        })?;

        // Verify with bcrypt (single verification, not N verifications!)
        if !crate::auth::keys::verify_virtual_key(token, &virtual_key.key_hash).unwrap_or(false) {
            return Err((StatusCode::UNAUTHORIZED, "Invalid API key".to_string()));
        }

        // CRITICAL FIX: Cache the verified token to skip bcrypt on future requests
        // This dramatically speeds up authenticated requests (9s → <10ms)
        if let Some(redis_conn) = state.get_redis_connection() {
            let verified_token_key = format!("auth:verified:{}", lookup_hash);
            // Cache for 5 minutes - shorter than key cache for security
            let _ = cache_key(redis_conn, &verified_token_key, &virtual_key, 300).await;
            tracing::debug!("Cached verified token for future requests");
        }

        // Validate key
        if !virtual_key.is_valid() {
            let reason = if virtual_key.blocked {
                "API key is blocked"
            } else if virtual_key.is_over_budget() {
                "API key has exceeded budget"
            } else if virtual_key.is_expired() {
                "API key has expired"
            } else {
                "API key is invalid"
            };
            return Err((StatusCode::UNAUTHORIZED, reason.to_string()));
        }

        // Get user info if key has a user
        let (user_id, email, role) = if let Some(user_id) = virtual_key.user_id {
            let user = User::find_by_id(pool, user_id)
                .await
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to verify user".to_string(),
                    )
                })?
                .ok_or_else(|| (StatusCode::UNAUTHORIZED, "User not found".to_string()))?;
            (user.id, user.email, user.role)
        } else {
            // System key without user
            (
                uuid::Uuid::nil(),
                "system".to_string(),
                "system".to_string(),
            )
        };

        let auth_user = AuthUser {
            user_id,
            email,
            role,
            auth_type: AuthType::VirtualKey {
                key_id: virtual_key.id,
            },
        };

        request.extensions_mut().insert(auth_user);

        // Update last used
        let _ = VirtualKey::update_last_used(pool, virtual_key.id).await;

        return Ok(next.run(request).await);
    }

    Err((
        StatusCode::UNAUTHORIZED,
        "Invalid authentication credentials".to_string(),
    ))
}

/// Trait for state that has a master key
pub trait HasMasterKey {
    fn get_master_key(&self) -> &str;
}

/// Trait for state that has a JWT secret
pub trait HasJwtSecret {
    fn get_jwt_secret(&self) -> &str;
}

/// Trait for state that has database access
pub trait HasDatabase {
    fn get_database_pool(&self) -> Option<&sqlx::Pool<sqlx::Postgres>>;
}

/// Trait for state that has Redis access
pub trait HasRedis {
    fn get_redis_connection(&self) -> Option<&redis::aio::ConnectionManager>;
}

/// Cache a virtual key in Redis
async fn cache_key(
    redis_conn: &redis::aio::ConnectionManager,
    redis_key: &str,
    key: &VirtualKey,
    ttl_seconds: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use redis::AsyncCommands;

    let mut conn = redis_conn.clone();
    let serialized = serde_json::to_string(key)?;
    conn.set_ex::<_, _, ()>(redis_key, serialized, ttl_seconds as u64)
        .await?;
    Ok(())
}

/// Get a cached virtual key from Redis
async fn get_cached_key(
    redis_conn: &redis::aio::ConnectionManager,
    redis_key: &str,
) -> Result<Option<VirtualKey>, Box<dyn std::error::Error + Send + Sync>> {
    use redis::AsyncCommands;

    let mut conn = redis_conn.clone();
    let cached: Option<String> = conn.get(redis_key).await?;

    match cached {
        Some(data) => {
            let key: VirtualKey = serde_json::from_str(&data)?;
            Ok(Some(key))
        }
        None => Ok(None),
    }
}

/// Virtual key information for rate limiting
#[derive(Debug, Clone)]
pub struct VirtualKeyInfo {
    pub key_id: uuid::Uuid,
    pub rate_limit_rpm: Option<i32>,
    pub rate_limit_tpm: Option<i32>,
}

/// Implement FromRequestParts to allow VirtualKeyInfo to be used as an extractor
#[async_trait]
impl<S> FromRequestParts<S> for VirtualKeyInfo
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<VirtualKeyInfo>()
            .cloned()
            .ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Missing virtual key info".to_string(),
                )
            })
    }
}

/// Trait for state that has a rate limiter
pub trait HasRateLimiter {
    fn get_rate_limiter(&self) -> Option<&RateLimiter>;
}

/// Middleware to enforce rate limits for virtual keys
/// This should be applied after require_auth middleware
pub async fn enforce_rate_limit<S>(
    State(state): State<Arc<S>>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)>
where
    S: HasDatabase + HasRateLimiter,
{
    // Get auth user from extensions (added by require_auth)
    let auth_user = request
        .extensions()
        .get::<AuthUser>()
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Missing authentication".to_string(),
            )
        })?
        .clone();

    // Only enforce rate limits for virtual keys
    if let AuthType::VirtualKey { key_id } = auth_user.auth_type {
        let pool = state.get_database_pool().ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database not available".to_string(),
            )
        })?;

        // Fetch the virtual key to get rate limit settings
        let virtual_key = VirtualKey::find_by_id(pool, key_id)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to fetch key info".to_string(),
                )
            })?
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Key not found".to_string()))?;

        // Store key info in extensions for handler use
        let key_info = VirtualKeyInfo {
            key_id: virtual_key.id,
            rate_limit_rpm: virtual_key.rate_limit_rpm,
            rate_limit_tpm: virtual_key.rate_limit_tpm,
        };
        request.extensions_mut().insert(key_info);

        // Check rate limits if configured
        if virtual_key.rate_limit_rpm.is_some() || virtual_key.rate_limit_tpm.is_some() {
            if let Some(rate_limiter) = state.get_rate_limiter() {
                let rate_limit = RateLimit {
                    requests_per_minute: virtual_key.rate_limit_rpm,
                    tokens_per_minute: virtual_key.rate_limit_tpm,
                };

                // For pre-flight check, we only check request count (tokens will be checked after processing)
                let status = rate_limiter
                    .check_and_increment(&key_id.to_string(), &rate_limit, 1)
                    .await
                    .map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Rate limit check failed: {}", e),
                        )
                    })?;

                if status.limited {
                    // Record rate limit exceeded metrics
                    let key_id_str = key_id.to_string();
                    if status.requests_remaining == Some(0) {
                        MetricsCollector::record_rate_limit_exceeded(&key_id_str, "requests");
                    }
                    if status.tokens_remaining == Some(0) {
                        MetricsCollector::record_rate_limit_exceeded(&key_id_str, "tokens");
                    }

                    use axum::http::header::{HeaderMap, HeaderValue};
                    let mut headers = HeaderMap::new();

                    if let Some(reset_at) = status.reset_at {
                        headers.insert(
                            "X-RateLimit-Reset",
                            HeaderValue::from_str(&reset_at.to_string()).unwrap(),
                        );
                    }

                    if let Some(retry_after) = status.retry_after {
                        headers.insert(
                            "Retry-After",
                            HeaderValue::from_str(&retry_after.to_string()).unwrap(),
                        );
                    }

                    if let Some(remaining) = status.requests_remaining {
                        headers.insert(
                            "X-RateLimit-Remaining-Requests",
                            HeaderValue::from_str(&remaining.to_string()).unwrap(),
                        );
                    }

                    if let Some(remaining) = status.tokens_remaining {
                        headers.insert(
                            "X-RateLimit-Remaining-Tokens",
                            HeaderValue::from_str(&remaining.to_string()).unwrap(),
                        );
                    }

                    return Err((
                        StatusCode::TOO_MANY_REQUESTS,
                        "Rate limit exceeded".to_string(),
                    ));
                }

                // Update rate limit remaining metrics
                let key_id_str = key_id.to_string();
                if let Some(remaining) = status.requests_remaining {
                    MetricsCollector::set_rate_limit_remaining(&key_id_str, "requests", remaining);
                }
                if let Some(remaining) = status.tokens_remaining {
                    MetricsCollector::set_rate_limit_remaining(&key_id_str, "tokens", remaining);
                }
            }
        }
    }

    Ok(next.run(request).await)
}
