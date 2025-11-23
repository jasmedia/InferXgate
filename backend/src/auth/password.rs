use bcrypt::{hash, verify, DEFAULT_COST};

use crate::error::{ApiError, ApiResult};

/// Hash a password using bcrypt
pub fn hash_password(password: &str) -> ApiResult<String> {
    hash(password, DEFAULT_COST).map_err(|e| ApiError::InternalError(format!("Failed to hash password: {}", e)))
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> ApiResult<bool> {
    verify(password, hash).map_err(|e| ApiError::InternalError(format!("Failed to verify password: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }
}
