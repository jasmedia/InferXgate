use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VirtualKey {
    pub id: Uuid,
    #[serde(skip_serializing)]
    pub key_hash: String,
    #[serde(skip_serializing)]
    pub key_lookup_hash: Option<String>, // SHA256 hash for fast lookup
    pub key_prefix: String, // Show "sk-..." to users
    pub user_id: Option<Uuid>,
    pub name: Option<String>,
    pub max_budget: Option<f64>,
    pub current_spend: f64,
    pub rate_limit_rpm: Option<i32>,
    pub rate_limit_tpm: Option<i32>,
    pub allowed_models: Option<Vec<String>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub blocked: bool,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateVirtualKeyRequest {
    pub name: Option<String>,
    pub max_budget: Option<f64>,
    pub rate_limit_rpm: Option<i32>,
    pub rate_limit_tpm: Option<i32>,
    pub allowed_models: Option<Vec<String>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VirtualKeyResponse {
    pub id: Uuid,
    pub key: String, // Full key returned only on creation
    pub key_prefix: String,
    pub name: Option<String>,
    pub max_budget: Option<f64>,
    pub current_spend: f64,
    pub rate_limit_rpm: Option<i32>,
    pub rate_limit_tpm: Option<i32>,
    pub allowed_models: Option<Vec<String>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub blocked: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateVirtualKeyRequest {
    pub name: Option<String>,
    pub max_budget: Option<f64>,
    pub rate_limit_rpm: Option<i32>,
    pub rate_limit_tpm: Option<i32>,
    pub allowed_models: Option<Vec<String>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub blocked: Option<bool>,
}

impl VirtualKey {
    /// Create a new virtual key
    pub async fn create(
        pool: &Pool<Postgres>,
        key_hash: String,
        key_lookup_hash: String,
        key_prefix: String,
        user_id: Option<Uuid>,
        name: Option<String>,
        max_budget: Option<f64>,
        rate_limit_rpm: Option<i32>,
        rate_limit_tpm: Option<i32>,
        allowed_models: Option<Vec<String>>,
        expires_at: Option<DateTime<Utc>>,
    ) -> ApiResult<Self> {
        let key: VirtualKey = sqlx::query_as(
            r#"
            INSERT INTO virtual_keys
            (key_hash, key_lookup_hash, key_prefix, user_id, name, max_budget, rate_limit_rpm,
             rate_limit_tpm, allowed_models, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, key_hash, key_lookup_hash, key_prefix, user_id, name, max_budget, current_spend,
                      rate_limit_rpm, rate_limit_tpm, allowed_models, expires_at, blocked,
                      created_at, last_used_at
            "#,
        )
        .bind(&key_hash)
        .bind(&key_lookup_hash)
        .bind(&key_prefix)
        .bind(user_id)
        .bind(&name)
        .bind(max_budget)
        .bind(rate_limit_rpm)
        .bind(rate_limit_tpm)
        .bind(&allowed_models)
        .bind(expires_at)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(key)
    }

    /// Find virtual key by lookup hash (fast, for authentication)
    pub async fn find_by_lookup_hash(
        pool: &Pool<Postgres>,
        lookup_hash: &str,
    ) -> ApiResult<Option<Self>> {
        let key = sqlx::query_as::<_, VirtualKey>(
            r#"
            SELECT id, key_hash, key_lookup_hash, key_prefix, user_id, name, max_budget, current_spend,
                   rate_limit_rpm, rate_limit_tpm, allowed_models, expires_at, blocked,
                   created_at, last_used_at
            FROM virtual_keys
            WHERE key_lookup_hash = $1
            "#,
        )
        .bind(lookup_hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(key)
    }

    /// Find virtual key by key hash
    pub async fn find_by_hash(pool: &Pool<Postgres>, key_hash: &str) -> ApiResult<Option<Self>> {
        let key = sqlx::query_as::<_, VirtualKey>(
            r#"
            SELECT id, key_hash, key_lookup_hash, key_prefix, user_id, name, max_budget, current_spend,
                   rate_limit_rpm, rate_limit_tpm, allowed_models, expires_at, blocked,
                   created_at, last_used_at
            FROM virtual_keys
            WHERE key_hash = $1
            "#,
        )
        .bind(key_hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(key)
    }

    /// Get all virtual keys (for bcrypt verification - use with caution)
    pub async fn find_all(pool: &Pool<Postgres>) -> ApiResult<Vec<Self>> {
        let keys = sqlx::query_as::<_, VirtualKey>(
            r#"
            SELECT id, key_hash, key_lookup_hash, key_prefix, user_id, name, max_budget, current_spend,
                   rate_limit_rpm, rate_limit_tpm, allowed_models, expires_at, blocked,
                   created_at, last_used_at
            FROM virtual_keys
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(keys)
    }

    /// Find virtual key by ID
    pub async fn find_by_id(pool: &Pool<Postgres>, key_id: Uuid) -> ApiResult<Option<Self>> {
        let key = sqlx::query_as::<_, VirtualKey>(
            r#"
            SELECT id, key_hash, key_lookup_hash, key_prefix, user_id, name, max_budget, current_spend,
                   rate_limit_rpm, rate_limit_tpm, allowed_models, expires_at, blocked,
                   created_at, last_used_at
            FROM virtual_keys
            WHERE id = $1
            "#,
        )
        .bind(key_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(key)
    }

    /// Get all keys for a user
    pub async fn find_by_user(pool: &Pool<Postgres>, user_id: Uuid) -> ApiResult<Vec<Self>> {
        let keys = sqlx::query_as::<_, VirtualKey>(
            r#"
            SELECT id, key_hash, key_lookup_hash, key_prefix, user_id, name, max_budget, current_spend,
                   rate_limit_rpm, rate_limit_tpm, allowed_models, expires_at, blocked,
                   created_at, last_used_at
            FROM virtual_keys
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(keys)
    }

    /// Update virtual key
    pub async fn update(
        pool: &Pool<Postgres>,
        key_id: Uuid,
        name: Option<String>,
        max_budget: Option<f64>,
        rate_limit_rpm: Option<i32>,
        rate_limit_tpm: Option<i32>,
        allowed_models: Option<Vec<String>>,
        expires_at: Option<DateTime<Utc>>,
        blocked: Option<bool>,
    ) -> ApiResult<Self> {
        let key: VirtualKey = sqlx::query_as(
            r#"
            UPDATE virtual_keys
            SET
                name = COALESCE($2, name),
                max_budget = COALESCE($3, max_budget),
                rate_limit_rpm = COALESCE($4, rate_limit_rpm),
                rate_limit_tpm = COALESCE($5, rate_limit_tpm),
                allowed_models = COALESCE($6, allowed_models),
                expires_at = COALESCE($7, expires_at),
                blocked = COALESCE($8, blocked)
            WHERE id = $1
            RETURNING id, key_hash, key_prefix, user_id, name, max_budget, current_spend,
                      rate_limit_rpm, rate_limit_tpm, allowed_models, expires_at, blocked,
                      created_at, last_used_at
            "#,
        )
        .bind(key_id)
        .bind(name)
        .bind(max_budget)
        .bind(rate_limit_rpm)
        .bind(rate_limit_tpm)
        .bind(&allowed_models)
        .bind(expires_at)
        .bind(blocked)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(key)
    }

    /// Block/unblock a key
    pub async fn set_blocked(pool: &Pool<Postgres>, key_id: Uuid, blocked: bool) -> ApiResult<()> {
        sqlx::query(
            r#"
            UPDATE virtual_keys
            SET blocked = $2
            WHERE id = $1
            "#,
        )
        .bind(key_id)
        .bind(blocked)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Increment spend for a key
    pub async fn increment_spend(
        pool: &Pool<Postgres>,
        key_id: Uuid,
        amount: f64,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            UPDATE virtual_keys
            SET current_spend = current_spend + $2,
                last_used_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(key_id)
        .bind(amount)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Update last used timestamp
    pub async fn update_last_used(pool: &Pool<Postgres>, key_id: Uuid) -> ApiResult<()> {
        sqlx::query(
            r#"
            UPDATE virtual_keys
            SET last_used_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(key_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Delete a virtual key
    pub async fn delete(pool: &Pool<Postgres>, key_id: Uuid) -> ApiResult<()> {
        sqlx::query(
            r#"
            DELETE FROM virtual_keys
            WHERE id = $1
            "#,
        )
        .bind(key_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Check if key has exceeded budget
    pub fn is_over_budget(&self) -> bool {
        if let Some(max_budget) = self.max_budget {
            self.current_spend >= max_budget
        } else {
            false
        }
    }

    /// Check if key is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }

    /// Check if key is valid for use
    pub fn is_valid(&self) -> bool {
        !self.blocked && !self.is_over_budget() && !self.is_expired()
    }

    /// Check if key can access a specific model
    pub fn can_access_model(&self, model: &str) -> bool {
        if let Some(allowed_models) = &self.allowed_models {
            allowed_models.iter().any(|m| m == model)
        } else {
            true // No restrictions
        }
    }
}
