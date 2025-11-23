use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: Option<String>,
    #[serde(skip_serializing)]
    pub password_hash: Option<String>,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OAuthAccount {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub provider_username: Option<String>,
    #[serde(skip_serializing)]
    pub access_token_encrypted: Option<String>,
    #[serde(skip_serializing)]
    pub refresh_token_encrypted: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    #[serde(skip_serializing)]
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl User {
    /// Create a new user with email and password
    pub async fn create(
        pool: &Pool<Postgres>,
        email: String,
        username: Option<String>,
        password_hash: Option<String>,
        role: String,
    ) -> ApiResult<Self> {
        let user: User = sqlx::query_as(
            r#"
            INSERT INTO users (email, username, password_hash, role)
            VALUES ($1, $2, $3, $4)
            RETURNING id, email, username, password_hash, role, created_at, updated_at
            "#,
        )
        .bind(&email)
        .bind(&username)
        .bind(&password_hash)
        .bind(&role)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                ApiError::BadRequest("User with this email already exists".to_string())
            } else {
                ApiError::DatabaseError(e.to_string())
            }
        })?;

        Ok(user)
    }

    /// Find user by email
    pub async fn find_by_email(pool: &Pool<Postgres>, email: &str) -> ApiResult<Option<Self>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, username, password_hash, role, created_at, updated_at
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(user)
    }

    /// Find user by ID
    pub async fn find_by_id(pool: &Pool<Postgres>, user_id: Uuid) -> ApiResult<Option<Self>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, username, password_hash, role, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(user)
    }

    /// Update user password
    pub async fn update_password(
        pool: &Pool<Postgres>,
        user_id: Uuid,
        new_password_hash: String,
    ) -> ApiResult<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET password_hash = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(new_password_hash)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Update user role
    pub async fn update_role(pool: &Pool<Postgres>, user_id: Uuid, role: String) -> ApiResult<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET role = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(role)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

impl OAuthAccount {
    /// Create or update OAuth account
    pub async fn upsert(
        pool: &Pool<Postgres>,
        user_id: Uuid,
        provider: String,
        provider_user_id: String,
        provider_username: Option<String>,
        access_token: Option<String>,
        refresh_token: Option<String>,
        expires_at: Option<DateTime<Utc>>,
    ) -> ApiResult<Self> {
        let account: OAuthAccount = sqlx::query_as(
            r#"
            INSERT INTO oauth_accounts
            (user_id, provider, provider_user_id, provider_username,
             access_token_encrypted, refresh_token_encrypted, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (provider, provider_user_id)
            DO UPDATE SET
                user_id = EXCLUDED.user_id,
                provider_username = EXCLUDED.provider_username,
                access_token_encrypted = EXCLUDED.access_token_encrypted,
                refresh_token_encrypted = EXCLUDED.refresh_token_encrypted,
                expires_at = EXCLUDED.expires_at
            RETURNING id, user_id, provider, provider_user_id, provider_username,
                      access_token_encrypted, refresh_token_encrypted, expires_at, created_at
            "#,
        )
        .bind(user_id)
        .bind(&provider)
        .bind(&provider_user_id)
        .bind(&provider_username)
        .bind(&access_token)
        .bind(&refresh_token)
        .bind(expires_at)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(account)
    }

    /// Find OAuth account by provider and provider user ID
    pub async fn find_by_provider(
        pool: &Pool<Postgres>,
        provider: &str,
        provider_user_id: &str,
    ) -> ApiResult<Option<Self>> {
        let account = sqlx::query_as::<_, OAuthAccount>(
            r#"
            SELECT id, user_id, provider, provider_user_id, provider_username,
                   access_token_encrypted, refresh_token_encrypted, expires_at, created_at
            FROM oauth_accounts
            WHERE provider = $1 AND provider_user_id = $2
            "#,
        )
        .bind(provider)
        .bind(provider_user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(account)
    }

    /// Get all OAuth accounts for a user
    pub async fn find_by_user(pool: &Pool<Postgres>, user_id: Uuid) -> ApiResult<Vec<Self>> {
        let accounts = sqlx::query_as::<_, OAuthAccount>(
            r#"
            SELECT id, user_id, provider, provider_user_id, provider_username,
                   access_token_encrypted, refresh_token_encrypted, expires_at, created_at
            FROM oauth_accounts
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(accounts)
    }
}

impl Session {
    /// Create a new session
    pub async fn create(
        pool: &Pool<Postgres>,
        user_id: Uuid,
        token_hash: String,
        expires_at: DateTime<Utc>,
    ) -> ApiResult<Self> {
        let session: Session = sqlx::query_as(
            r#"
            INSERT INTO sessions (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, token_hash, expires_at, created_at
            "#,
        )
        .bind(user_id)
        .bind(&token_hash)
        .bind(expires_at)
        .fetch_one(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(session)
    }

    /// Find session by token hash
    pub async fn find_by_token(
        pool: &Pool<Postgres>,
        token_hash: &str,
    ) -> ApiResult<Option<Self>> {
        let session = sqlx::query_as::<_, Session>(
            r#"
            SELECT id, user_id, token_hash, expires_at, created_at
            FROM sessions
            WHERE token_hash = $1 AND expires_at > NOW()
            "#,
        )
        .bind(token_hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(session)
    }

    /// Delete session (logout)
    pub async fn delete(pool: &Pool<Postgres>, token_hash: &str) -> ApiResult<()> {
        sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE token_hash = $1
            "#,
        )
        .bind(token_hash)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Delete all expired sessions (cleanup)
    pub async fn cleanup_expired(pool: &Pool<Postgres>) -> ApiResult<()> {
        sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE expires_at <= NOW()
            "#,
        )
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Delete all sessions for a user
    pub async fn delete_by_user(pool: &Pool<Postgres>, user_id: Uuid) -> ApiResult<()> {
        sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
