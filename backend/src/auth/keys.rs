use base64::{engine::general_purpose, Engine as _};
use rand::Rng;
use sha2::{Digest, Sha256};

use crate::error::{ApiError, ApiResult};

/// Generate a new virtual key with "sk-" prefix (LiteLLM compatible)
pub fn generate_virtual_key() -> String {
    let random_bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen()).collect();
    let key = general_purpose::STANDARD.encode(&random_bytes);
    format!("sk-{}", key)
}

/// Hash a virtual key for storage
/// Uses bcrypt cost of 10 for balance between security and performance
/// Cost 10 provides ~100ms verification time (vs 9+ seconds with higher costs)
pub fn hash_virtual_key(key: &str) -> ApiResult<String> {
    use bcrypt::hash;
    const BCRYPT_COST: u32 = 10;
    hash(key, BCRYPT_COST)
        .map_err(|e| ApiError::InternalError(format!("Failed to hash key: {}", e)))
}

/// Verify a virtual key against a hash
pub fn verify_virtual_key(key: &str, hash: &str) -> ApiResult<bool> {
    use bcrypt::verify;
    verify(key, hash).map_err(|e| ApiError::InternalError(format!("Failed to verify key: {}", e)))
}

/// Create a SHA256 lookup hash for fast key authentication
/// This is used for O(1) database lookups, not for security (bcrypt is still used for that)
pub fn create_lookup_hash(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Get the prefix of a key for display (first 12 characters)
pub fn get_key_prefix(key: &str) -> String {
    if key.len() >= 12 {
        key[..12].to_string()
    } else {
        key.to_string()
    }
}

/// Validate master key format (must start with "sk-")
pub fn validate_master_key_format(key: &str) -> ApiResult<()> {
    if !key.starts_with("sk-") {
        return Err(ApiError::BadRequest(
            "Master key must start with 'sk-'".to_string(),
        ));
    }

    if key.len() < 10 {
        return Err(ApiError::BadRequest("Master key is too short".to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_virtual_key() {
        let key = generate_virtual_key();
        assert!(key.starts_with("sk-"));
        assert!(key.len() > 10);
    }

    #[test]
    fn test_hash_and_verify_key() {
        let key = generate_virtual_key();
        let hash = hash_virtual_key(&key).unwrap();

        assert!(verify_virtual_key(&key, &hash).unwrap());
        assert!(!verify_virtual_key("sk-wrong-key", &hash).unwrap());
    }

    #[test]
    fn test_get_key_prefix() {
        let key = "sk-1234567890abcdefgh";
        let prefix = get_key_prefix(&key);
        assert_eq!(prefix, "sk-123456789");
    }

    #[test]
    fn test_validate_master_key_format() {
        assert!(validate_master_key_format("sk-valid-key-123").is_ok());
        assert!(validate_master_key_format("invalid-key").is_err());
        assert!(validate_master_key_format("sk-").is_err());
    }
}
