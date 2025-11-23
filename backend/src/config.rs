use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub anthropic_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub azure_api_key: Option<String>,
    pub azure_resource_name: Option<String>,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub aws_region: Option<String>,
    pub cohere_api_key: Option<String>,
    pub log_level: String,
    pub redis_url: Option<String>,
    pub database_url: Option<String>,
    pub enable_caching: bool,
    pub cache_ttl_seconds: u64,

    // Authentication configuration
    pub master_key: Option<String>,
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub require_auth: bool,

    // OAuth configuration
    pub github_client_id: Option<String>,
    pub github_client_secret: Option<String>,
    pub google_client_id: Option<String>,
    pub google_client_secret: Option<String>,
    pub oauth_redirect_url: String,
    pub frontend_url: String,

    // Security configuration
    pub allowed_email_domains: Option<Vec<String>>,
    pub proxy_admin_id: Option<String>,
}

impl AppConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();

        // Generate a default JWT secret if not provided (for development only)
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
            eprintln!("WARNING: JWT_SECRET not set, using default (INSECURE for production!)");
            "default-jwt-secret-change-me-in-production".to_string()
        });

        // Parse allowed email domains
        let allowed_email_domains = env::var("ALLOWED_EMAIL_DOMAINS").ok().map(|domains| {
            domains
                .split(',')
                .map(|d| d.trim().to_string())
                .filter(|d| !d.is_empty())
                .collect()
        });

        Ok(AppConfig {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()?,
            anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok(),
            gemini_api_key: env::var("GEMINI_API_KEY").ok(),
            openai_api_key: env::var("OPENAI_API_KEY").ok(),
            azure_api_key: env::var("AZURE_API_KEY").ok(),
            azure_resource_name: env::var("AZURE_RESOURCE_NAME").ok(),
            aws_access_key_id: env::var("AWS_ACCESS_KEY_ID").ok(),
            aws_secret_access_key: env::var("AWS_SECRET_ACCESS_KEY").ok(),
            aws_region: env::var("AWS_REGION").ok(),
            cohere_api_key: env::var("COHERE_API_KEY").ok(),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            redis_url: env::var("REDIS_URL").ok(),
            database_url: env::var("DATABASE_URL").ok(),
            enable_caching: env::var("ENABLE_CACHING")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            cache_ttl_seconds: env::var("CACHE_TTL_SECONDS")
                .unwrap_or_else(|_| "3600".to_string())
                .parse()
                .unwrap_or(3600),

            // Authentication configuration
            master_key: env::var("INFERXGATE_MASTER_KEY").ok(),
            jwt_secret,
            jwt_expiry_hours: env::var("JWT_EXPIRY_HOURS")
                .unwrap_or_else(|_| "168".to_string()) // 7 days default
                .parse()
                .unwrap_or(168),
            require_auth: env::var("REQUIRE_AUTH")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),

            // OAuth configuration
            github_client_id: env::var("GITHUB_CLIENT_ID").ok(),
            github_client_secret: env::var("GITHUB_CLIENT_SECRET").ok(),
            google_client_id: env::var("GOOGLE_CLIENT_ID").ok(),
            google_client_secret: env::var("GOOGLE_CLIENT_SECRET").ok(),
            oauth_redirect_url: env::var("OAUTH_REDIRECT_URL")
                .unwrap_or_else(|_| "http://localhost:3000/auth/oauth/callback".to_string()),
            frontend_url: env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),

            // Security configuration
            allowed_email_domains,
            proxy_admin_id: env::var("PROXY_ADMIN_ID").ok(),
        })
    }
}
