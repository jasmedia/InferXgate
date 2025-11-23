use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,      // User ID
    pub email: String,    // User email
    pub role: String,     // User role
    pub exp: i64,         // Expiration time
    pub iat: i64,         // Issued at
}

/// Generate a JWT token for a user
pub fn generate_token(
    user_id: Uuid,
    email: String,
    role: String,
    secret: &str,
    expiry_hours: i64,
) -> ApiResult<String> {
    let now = Utc::now();
    let expiration = now + Duration::hours(expiry_hours);

    let claims = Claims {
        sub: user_id.to_string(),
        email,
        role,
        exp: expiration.timestamp(),
        iat: now.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ApiError::InternalError(format!("Failed to generate token: {}", e)))
}

/// Validate and decode a JWT token
pub fn validate_token(token: &str, secret: &str) -> ApiResult<Claims> {
    let validation = Validation::default();

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|e| ApiError::AuthenticationFailed)
}

/// Extract token from Authorization header
pub fn extract_bearer_token(auth_header: &str) -> ApiResult<&str> {
    if !auth_header.starts_with("Bearer ") {
        return Err(ApiError::AuthenticationFailed);
    }

    Ok(&auth_header[7..]) // Skip "Bearer "
}

/// Hash a token for storage (for session tracking and invalidation)
pub fn hash_token(token: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_generate_and_validate_token() {
        let secret = "test_secret_key";
        let user_id = Uuid::new_v4();
        let email = "test@example.com".to_string();
        let role = "user".to_string();

        let token = generate_token(user_id, email.clone(), role.clone(), secret, 24).unwrap();
        let claims = validate_token(&token, secret).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, email);
        assert_eq!(claims.role, role);
    }

    #[test]
    fn test_extract_bearer_token() {
        let auth_header = "Bearer my_token_123";
        let token = extract_bearer_token(auth_header).unwrap();
        assert_eq!(token, "my_token_123");

        let invalid_header = "NotBearer token";
        assert!(extract_bearer_token(invalid_header).is_err());
    }

    #[test]
    fn test_invalid_token() {
        let secret = "test_secret_key";
        let invalid_token = "invalid.token.here";
        assert!(validate_token(invalid_token, secret).is_err());
    }
}
