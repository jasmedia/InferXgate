use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

#[derive(Clone)]
pub struct DatabaseManager {
    pool: Option<Pool<Postgres>>,
    enabled: bool,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UsageRecord {
    pub id: Uuid,
    pub model: String,
    pub provider: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub cost_usd: f64,
    pub latency_ms: i64,
    pub user_id: Option<String>,
    pub cached: bool,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageStats {
    pub total_requests: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub average_latency_ms: f64,
    pub cache_hit_rate: f64,
    pub requests_by_model: Vec<ModelStats>,
    pub requests_by_provider: Vec<ProviderStats>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ModelStats {
    pub model: String,
    pub count: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProviderStats {
    pub provider: String,
    pub count: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
}

impl DatabaseManager {
    pub async fn new(database_url: Option<String>) -> Self {
        let pool = if let Some(url) = database_url {
            match PgPoolOptions::new().max_connections(10).connect(&url).await {
                Ok(pool) => {
                    info!("Database connection established");

                    // Run migrations
                    if let Err(e) = Self::run_migrations(&pool).await {
                        error!("Failed to run migrations: {}", e);
                        return Self {
                            pool: None,
                            enabled: false,
                        };
                    }

                    Some(pool)
                }
                Err(e) => {
                    error!("Failed to connect to database: {}", e);
                    None
                }
            }
        } else {
            debug!("Database URL not provided, usage tracking disabled");
            None
        };

        Self {
            enabled: pool.is_some(),
            pool,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_pool(&self) -> Option<&Pool<Postgres>> {
        self.pool.as_ref()
    }

    async fn run_migrations(pool: &Pool<Postgres>) -> ApiResult<()> {
        info!("Running database migrations...");

        // Create users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                email VARCHAR(255) UNIQUE NOT NULL,
                username VARCHAR(255),
                password_hash VARCHAR(255),
                role VARCHAR(50) NOT NULL DEFAULT 'user',
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Create oauth_accounts table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_accounts (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                provider VARCHAR(50) NOT NULL,
                provider_user_id VARCHAR(255) NOT NULL,
                provider_username VARCHAR(255),
                access_token_encrypted TEXT,
                refresh_token_encrypted TEXT,
                expires_at TIMESTAMP WITH TIME ZONE,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                UNIQUE(provider, provider_user_id)
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Create virtual_keys table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS virtual_keys (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                key_hash VARCHAR(255) UNIQUE NOT NULL,
                key_prefix VARCHAR(20) NOT NULL,
                user_id UUID REFERENCES users(id) ON DELETE CASCADE,
                name VARCHAR(255),
                max_budget DOUBLE PRECISION,
                current_spend DOUBLE PRECISION NOT NULL DEFAULT 0,
                rate_limit_rpm INTEGER,
                rate_limit_tpm INTEGER,
                allowed_models TEXT[],
                expires_at TIMESTAMP WITH TIME ZONE,
                blocked BOOLEAN NOT NULL DEFAULT false,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                last_used_at TIMESTAMP WITH TIME ZONE
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Add key_lookup_hash column if it doesn't exist (for fast key lookup)
        sqlx::query(
            r#"
            DO $$
            BEGIN
                IF NOT EXISTS (
                    SELECT 1 FROM information_schema.columns
                    WHERE table_name = 'virtual_keys' AND column_name = 'key_lookup_hash'
                ) THEN
                    ALTER TABLE virtual_keys ADD COLUMN key_lookup_hash VARCHAR(64);
                END IF;
            END $$;
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Create sessions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                token_hash VARCHAR(255) UNIQUE NOT NULL,
                expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Create usage_records table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS usage_records (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                model VARCHAR(255) NOT NULL,
                provider VARCHAR(100) NOT NULL,
                prompt_tokens INTEGER NOT NULL,
                completion_tokens INTEGER NOT NULL,
                total_tokens INTEGER NOT NULL,
                cost_usd DOUBLE PRECISION NOT NULL,
                latency_ms BIGINT NOT NULL,
                user_id VARCHAR(255),
                virtual_key_id UUID REFERENCES virtual_keys(id) ON DELETE SET NULL,
                cached BOOLEAN NOT NULL DEFAULT false,
                error TEXT,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Create indexes for users
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_users_email
            ON users(email)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Create indexes for oauth_accounts
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_oauth_accounts_user_id
            ON oauth_accounts(user_id)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_oauth_accounts_provider
            ON oauth_accounts(provider, provider_user_id)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Create indexes for virtual_keys
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_virtual_keys_user_id
            ON virtual_keys(user_id)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_virtual_keys_key_hash
            ON virtual_keys(key_hash)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Create index for key_lookup_hash for fast key authentication
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_virtual_keys_key_lookup_hash
            ON virtual_keys(key_lookup_hash)
            WHERE key_lookup_hash IS NOT NULL
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Create indexes for sessions
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id
            ON sessions(user_id)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sessions_token_hash
            ON sessions(token_hash)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sessions_expires_at
            ON sessions(expires_at)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Create indexes for usage_records
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_usage_records_created_at
            ON usage_records(created_at DESC)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_usage_records_model
            ON usage_records(model)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_usage_records_provider
            ON usage_records(provider)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_usage_records_virtual_key_id
            ON usage_records(virtual_key_id)
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Create provider_keys table for storing provider API keys
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS provider_keys (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                provider_id VARCHAR(100) UNIQUE NOT NULL,
                api_key_encrypted TEXT NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
            )
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        info!("Database migrations completed successfully");
        Ok(())
    }

    pub async fn record_usage(
        &self,
        model: &str,
        provider: &str,
        prompt_tokens: i32,
        completion_tokens: i32,
        total_tokens: i32,
        cost_usd: f64,
        latency_ms: i64,
        user_id: Option<String>,
        cached: bool,
        error: Option<String>,
    ) -> ApiResult<Uuid> {
        if !self.enabled {
            return Ok(Uuid::new_v4());
        }

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| ApiError::DatabaseError("Database pool not available".to_string()))?;

        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO usage_records
            (id, model, provider, prompt_tokens, completion_tokens, total_tokens,
             cost_usd, latency_ms, user_id, cached, error)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(id)
        .bind(model)
        .bind(provider)
        .bind(prompt_tokens)
        .bind(completion_tokens)
        .bind(total_tokens)
        .bind(cost_usd)
        .bind(latency_ms)
        .bind(user_id)
        .bind(cached)
        .bind(error)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        debug!(
            "Recorded usage for model: {}, tokens: {}",
            model, total_tokens
        );
        Ok(id)
    }

    pub async fn get_usage_stats(&self, days: i32) -> ApiResult<UsageStats> {
        if !self.enabled {
            return Err(ApiError::DatabaseError(
                "Database not available".to_string(),
            ));
        }

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| ApiError::DatabaseError("Database pool not available".to_string()))?;

        // Get total stats
        let total_stats: (i64, i64, f64, f64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) as total_requests,
                COALESCE(SUM(total_tokens), 0) as total_tokens,
                CAST(COALESCE(SUM(cost_usd), 0) AS DOUBLE PRECISION) as total_cost,
                CAST(COALESCE(AVG(latency_ms), 0) AS DOUBLE PRECISION) as average_latency_ms
            FROM usage_records
            WHERE created_at >= NOW() - INTERVAL '1 day' * $1
            "#,
        )
        .bind(days)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Get cache hit rate
        let cache_stats: (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE cached = true) as cached_requests,
                COUNT(*) as total_requests
            FROM usage_records
            WHERE created_at >= NOW() - INTERVAL '1 day' * $1
            "#,
        )
        .bind(days)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        let cache_hit_rate = if cache_stats.1 > 0 {
            (cache_stats.0 as f64 / cache_stats.1 as f64) * 100.0
        } else {
            0.0
        };

        // Get stats by model
        let model_stats: Vec<ModelStats> = sqlx::query_as(
            r#"
            SELECT
                model,
                COUNT(*) as count,
                COALESCE(SUM(total_tokens), 0) as total_tokens,
                CAST(COALESCE(SUM(cost_usd), 0) AS DOUBLE PRECISION) as total_cost
            FROM usage_records
            WHERE created_at >= NOW() - INTERVAL '1 day' * $1
            GROUP BY model
            ORDER BY count DESC
            "#,
        )
        .bind(days)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        // Get stats by provider
        let provider_stats: Vec<ProviderStats> = sqlx::query_as(
            r#"
            SELECT
                provider,
                COUNT(*) as count,
                COALESCE(SUM(total_tokens), 0) as total_tokens,
                CAST(COALESCE(SUM(cost_usd), 0) AS DOUBLE PRECISION) as total_cost
            FROM usage_records
            WHERE created_at >= NOW() - INTERVAL '1 day' * $1
            GROUP BY provider
            ORDER BY count DESC
            "#,
        )
        .bind(days)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(UsageStats {
            total_requests: total_stats.0,
            total_tokens: total_stats.1,
            total_cost: total_stats.2,
            average_latency_ms: total_stats.3,
            cache_hit_rate,
            requests_by_model: model_stats,
            requests_by_provider: provider_stats,
        })
    }

    pub async fn get_recent_usage(&self, limit: i64) -> ApiResult<Vec<UsageRecord>> {
        if !self.enabled {
            return Err(ApiError::DatabaseError(
                "Database not available".to_string(),
            ));
        }

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| ApiError::DatabaseError("Database pool not available".to_string()))?;

        let records: Vec<UsageRecord> = sqlx::query_as(
            r#"
            SELECT id, model, provider, prompt_tokens, completion_tokens,
                   total_tokens, cost_usd, latency_ms, user_id, cached, error, created_at
            FROM usage_records
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(records)
    }

    /// Store or update a provider API key
    pub async fn store_provider_key(&self, provider_id: &str, api_key: &str) -> ApiResult<()> {
        if !self.enabled {
            return Err(ApiError::DatabaseError(
                "Database not available".to_string(),
            ));
        }

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| ApiError::DatabaseError("Database pool not available".to_string()))?;

        // For now, we'll store the key as-is. In production, you should encrypt it.
        // TODO: Implement proper encryption using a secret key from environment
        sqlx::query(
            r#"
            INSERT INTO provider_keys (provider_id, api_key_encrypted, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (provider_id)
            DO UPDATE SET api_key_encrypted = $2, updated_at = NOW()
            "#,
        )
        .bind(provider_id)
        .bind(api_key)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Retrieve a provider API key
    pub async fn get_provider_key(&self, provider_id: &str) -> ApiResult<Option<String>> {
        if !self.enabled {
            return Ok(None);
        }

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| ApiError::DatabaseError("Database pool not available".to_string()))?;

        let result: Option<(String,)> = sqlx::query_as(
            r#"
            SELECT api_key_encrypted
            FROM provider_keys
            WHERE provider_id = $1
            "#,
        )
        .bind(provider_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(result.map(|r| r.0))
    }

    /// Delete a provider API key
    pub async fn delete_provider_key(&self, provider_id: &str) -> ApiResult<()> {
        if !self.enabled {
            return Err(ApiError::DatabaseError(
                "Database not available".to_string(),
            ));
        }

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| ApiError::DatabaseError("Database pool not available".to_string()))?;

        sqlx::query(
            r#"
            DELETE FROM provider_keys
            WHERE provider_id = $1
            "#,
        )
        .bind(provider_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Load all provider keys from database
    pub async fn load_all_provider_keys(&self) -> ApiResult<Vec<(String, String)>> {
        if !self.enabled {
            return Ok(vec![]);
        }

        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| ApiError::DatabaseError("Database pool not available".to_string()))?;

        let results: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT provider_id, api_key_encrypted
            FROM provider_keys
            ORDER BY provider_id
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(results)
    }
}
