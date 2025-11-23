use axum::{
    extract::{Json, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tower_http::cors::CorsLayer;
use tracing::info;

mod auth;
mod cache;
mod config;
mod cost;
mod database;
mod error;
mod handlers;
mod load_balancer;
mod metrics;
mod models;
mod provider_config;
mod providers;
mod rate_limiter;

use cache::CacheManager;
use config::AppConfig;
use cost::CostCalculator;
use database::DatabaseManager;
use error::{ApiError, ApiResult};
use load_balancer::{LoadBalancer, LoadBalancingStrategy};
use metrics::MetricsCollector;
use providers::{
    anthropic::AnthropicProvider, azure::AzureProvider, gemini::GeminiProvider,
    openai::OpenAIProvider, LLMProvider,
};
use rate_limiter::RateLimiter;

// OpenAI-compatible request/response structures
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlContent },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ImageUrlContent {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: i32,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
}

// Model routing configuration
#[derive(Debug, Clone)]
pub struct ModelRoute {
    pub provider: String,
    pub target_model: String,
    pub api_key: String,
}

pub struct AppState {
    pub config: AppConfig,
    pub model_routes: DashMap<String, ModelRoute>, // Lock-free concurrent HashMap
    pub providers: HashMap<String, Box<dyn LLMProvider>>,
    pub cache: CacheManager,
    pub database: DatabaseManager,
    pub cost_calculator: CostCalculator,
    pub load_balancer: LoadBalancer,
    pub redis: Option<redis::aio::ConnectionManager>,
    pub rate_limiter: RateLimiter,
}

// Implement middleware traits for AppState
impl auth::HasMasterKey for AppState {
    fn get_master_key(&self) -> &str {
        self.config.master_key.as_deref().unwrap_or("") // Return empty if not set
    }
}

impl auth::HasJwtSecret for AppState {
    fn get_jwt_secret(&self) -> &str {
        &self.config.jwt_secret
    }
}

impl auth::HasDatabase for AppState {
    fn get_database_pool(&self) -> Option<&sqlx::Pool<sqlx::Postgres>> {
        self.database.get_pool()
    }
}

impl auth::HasRedis for AppState {
    fn get_redis_connection(&self) -> Option<&redis::aio::ConnectionManager> {
        self.redis.as_ref()
    }
}

impl auth::HasRateLimiter for AppState {
    fn get_rate_limiter(&self) -> Option<&RateLimiter> {
        Some(&self.rate_limiter)
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = AppConfig::load().expect("Failed to load configuration");

    // Initialize cache
    let cache = CacheManager::new(
        config.redis_url.clone(),
        config.cache_ttl_seconds,
        config.enable_caching,
    )
    .await;
    info!("Cache initialized: {}", cache.is_enabled());

    // Initialize database
    let database = DatabaseManager::new(config.database_url.clone()).await;
    info!("Database initialized: {}", database.is_enabled());

    // Initialize Redis connection for auth caching
    let redis = if let Some(redis_url) = &config.redis_url {
        match redis::Client::open(redis_url.as_str()) {
            Ok(client) => match client.get_connection_manager().await {
                Ok(manager) => {
                    info!("Redis connection for auth caching established");
                    Some(manager)
                }
                Err(e) => {
                    tracing::warn!("Failed to create Redis connection manager: {}", e);
                    None
                }
            },
            Err(e) => {
                tracing::warn!("Failed to create Redis client: {}", e);
                None
            }
        }
    } else {
        info!("Redis URL not provided, auth caching disabled");
        None
    };

    // Initialize cost calculator
    let cost_calculator = CostCalculator::new();
    info!("Cost calculator initialized");

    // Initialize load balancer (use RoundRobin by default, can be configurable)
    let load_balancer = LoadBalancer::new(LoadBalancingStrategy::RoundRobin);
    info!("Load balancer initialized with RoundRobin strategy");

    // Initialize rate limiter
    let rate_limiter = RateLimiter::new(redis.clone());
    info!("Rate limiter initialized");

    // Initialize providers
    let mut providers: HashMap<String, Box<dyn LLMProvider>> = HashMap::new();
    providers.insert("anthropic".to_string(), Box::new(AnthropicProvider::new()));
    providers.insert("gemini".to_string(), Box::new(GeminiProvider::new()));
    providers.insert("openai".to_string(), Box::new(OpenAIProvider::new()));

    // Initialize Azure provider (resource name is passed via api_key as "resource:key")
    providers.insert("azure".to_string(), Box::new(AzureProvider::new()));

    // Load provider keys from database (takes precedence over env vars)
    let db_provider_keys = if database.is_enabled() {
        match database.load_all_provider_keys().await {
            Ok(keys) => {
                info!("✅ Loaded {} provider keys from database", keys.len());
                keys.into_iter().collect::<HashMap<_, _>>()
            }
            Err(e) => {
                tracing::warn!("⚠️ Failed to load provider keys from database: {}", e);
                HashMap::new()
            }
        }
    } else {
        HashMap::new()
    };

    // Initialize model routes
    let mut model_routes = HashMap::new();

    // Anthropic models
    for model in provider_config::anthropic::PRIMARY_MODELS {
        // Prefer database key over env var
        let api_key = db_provider_keys
            .get("anthropic")
            .or(config.anthropic_api_key.as_ref());

        if let Some(api_key) = api_key {
            model_routes.insert(
                model.to_string(),
                ModelRoute {
                    provider: "anthropic".to_string(),
                    target_model: model.to_string(),
                    api_key: api_key.clone(),
                },
            );
        }
    }

    // Gemini models (updated to 2.x family - 1.x deprecated)
    for model in provider_config::gemini::PRIMARY_MODELS {
        // Prefer database key over env var
        let api_key = db_provider_keys
            .get("gemini")
            .or(config.gemini_api_key.as_ref());

        if let Some(api_key) = api_key {
            model_routes.insert(
                model.to_string(),
                ModelRoute {
                    provider: "gemini".to_string(),
                    target_model: model.to_string(),
                    api_key: api_key.clone(),
                },
            );
        }
    }

    // OpenAI models
    for model in provider_config::openai::PRIMARY_MODELS {
        // Prefer database key over env var
        let api_key = db_provider_keys
            .get("openai")
            .or(config.openai_api_key.as_ref());

        if let Some(api_key) = api_key {
            model_routes.insert(
                model.to_string(),
                ModelRoute {
                    provider: "openai".to_string(),
                    target_model: model.to_string(),
                    api_key: api_key.clone(),
                },
            );
        }
    }

    // Azure OpenAI models
    for model in provider_config::azure::PRIMARY_MODELS {
        // Prefer database key over env var
        let api_key = db_provider_keys
            .get("azure")
            .or(config.azure_api_key.as_ref());

        if let Some(api_key) = api_key {
            model_routes.insert(
                model.to_string(),
                ModelRoute {
                    provider: "azure".to_string(),
                    target_model: model.to_string(),
                    api_key: api_key.clone(),
                },
            );
        }
    }

    // Convert HashMap to DashMap for lock-free concurrent access
    let model_routes_dashmap = DashMap::new();
    for (key, value) in model_routes {
        model_routes_dashmap.insert(key, value);
    }
    info!("✅ Model routes initialized with lock-free DashMap");

    let app_state = Arc::new(AppState {
        config: config.clone(),
        model_routes: model_routes_dashmap,
        providers,
        cache,
        database,
        cost_calculator,
        load_balancer,
        redis,
        rate_limiter,
    });

    // Build authentication routes (public)
    let auth_routes = Router::new()
        .route("/auth/register", post(handlers::register))
        .route("/auth/login", post(handlers::login))
        .route("/auth/oauth/github", get(handlers::github_oauth_start))
        .route("/auth/oauth/callback", get(handlers::oauth_callback));

    // User routes (require JWT)
    let user_routes = Router::new()
        .route("/auth/me", get(handlers::get_current_user))
        .route("/auth/logout", post(handlers::logout))
        .route("/auth/keys", get(handlers::get_user_keys))
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::require_jwt,
        ));

    // Key management routes (require auth - master key OR JWT)
    let key_routes = Router::new()
        .route("/auth/key/generate", post(handlers::generate_key))
        .route("/auth/key/info", get(handlers::get_key_info))
        .route("/auth/key/update", post(handlers::update_key))
        .route("/auth/key/delete", post(handlers::delete_key))
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::require_auth,
        ));

    // Provider configuration routes (require auth - master key OR JWT)
    let provider_routes = Router::new()
        .route(
            "/v1/providers/configure",
            post(handlers::update_provider_key),
        )
        .route("/v1/providers/delete", post(handlers::delete_provider_key))
        .route_layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth::require_auth,
        ));

    // API routes (conditionally protected)
    let api_routes = if config.require_auth {
        Router::new()
            .route("/v1/chat/completions", post(chat_completions))
            .route("/v1/models", post(list_models))
            .route_layer(middleware::from_fn_with_state(
                app_state.clone(),
                auth::enforce_rate_limit,
            ))
            .route_layer(middleware::from_fn_with_state(
                app_state.clone(),
                auth::require_auth,
            ))
    } else {
        Router::new()
            .route("/v1/chat/completions", post(chat_completions))
            .route("/v1/models", post(list_models))
    };

    // Public routes (health and metrics)
    let public_routes = Router::new()
        .route("/health", post(health_check))
        .route("/metrics", get(metrics_handler))
        .route("/stats", get(stats_handler))
        .route("/v1/providers", get(list_providers));

    // Combine all routes
    let app = Router::new()
        .merge(auth_routes)
        .merge(user_routes)
        .merge(key_routes)
        .merge(provider_routes)
        .merge(api_routes)
        .merge(public_routes)
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let addr = format!("{}:{}", config.host, config.port);
    info!("LLM Gateway server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app).await.unwrap();
}

/// Helper function to add rate limit headers to a response
fn add_rate_limit_headers(
    mut response: Response,
    status: &rate_limiter::RateLimitStatus,
) -> Response {
    let headers = response.headers_mut();

    if let Some(requests_remaining) = status.requests_remaining {
        headers.insert(
            "X-RateLimit-Limit-Requests",
            axum::http::HeaderValue::from_str(&requests_remaining.to_string())
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("0")),
        );
    }

    if let Some(tokens_remaining) = status.tokens_remaining {
        headers.insert(
            "X-RateLimit-Limit-Tokens",
            axum::http::HeaderValue::from_str(&tokens_remaining.to_string())
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("0")),
        );
    }

    if let Some(reset_at) = status.reset_at {
        headers.insert(
            "X-RateLimit-Reset",
            axum::http::HeaderValue::from_str(&reset_at.to_string())
                .unwrap_or_else(|_| axum::http::HeaderValue::from_static("0")),
        );
    }

    response
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    key_info: Option<auth::VirtualKeyInfo>,
    Json(request): Json<ChatCompletionRequest>,
) -> ApiResult<Response> {
    let start_time = std::time::Instant::now();
    tracing::info!("🚀 Request started for model: {}", request.model);

    // Get current rate limit status for headers
    let rate_limit_status = if let Some(ref info) = key_info {
        if info.rate_limit_rpm.is_some() || info.rate_limit_tpm.is_some() {
            state
                .rate_limiter
                .get_status(
                    &info.key_id.to_string(),
                    &rate_limiter::RateLimit {
                        requests_per_minute: info.rate_limit_rpm,
                        tokens_per_minute: info.rate_limit_tpm,
                    },
                )
                .await
                .ok()
        } else {
            None
        }
    } else {
        None
    };

    // Get model route (lock-free with DashMap)
    let route_lookup_start = std::time::Instant::now();
    let route = state
        .model_routes
        .get(&request.model)
        .ok_or_else(|| ApiError::ModelNotFound(request.model.clone()))?
        .clone();
    tracing::info!(
        "📋 Route lookup (lock-free): {:?}",
        route_lookup_start.elapsed()
    );

    // Check cache for non-streaming requests
    let is_streaming = request.stream.unwrap_or(false);

    if !is_streaming && state.cache.is_enabled() {
        let cache_check_start = std::time::Instant::now();
        let cache_key = state.cache.generate_cache_key(
            &request.model,
            &serde_json::to_string(&request.messages).unwrap_or_default(),
        );

        if let Ok(Some(cached_response)) =
            state.cache.get::<ChatCompletionResponse>(&cache_key).await
        {
            tracing::info!("💾 Cache HIT: {:?}", cache_check_start.elapsed());
            MetricsCollector::record_cache_hit();
            MetricsCollector::record_request(&request.model, &route.provider, true);

            // Record cached usage
            if state.database.is_enabled() {
                let _ = state
                    .database
                    .record_usage(
                        &request.model,
                        &route.provider,
                        cached_response.usage.prompt_tokens,
                        cached_response.usage.completion_tokens,
                        cached_response.usage.total_tokens,
                        0.0, // No cost for cached requests
                        start_time.elapsed().as_millis() as i64,
                        request.user.clone(),
                        true,
                        None,
                    )
                    .await;
            }

            tracing::info!("✅ Total time (cached): {:?}", start_time.elapsed());
            let mut response = Json(cached_response).into_response();
            if let Some(ref status) = rate_limit_status {
                response = add_rate_limit_headers(response, status);
            }
            return Ok(response);
        }

        tracing::info!("❌ Cache MISS: {:?}", cache_check_start.elapsed());
        MetricsCollector::record_cache_miss();
    }

    // Get provider
    let provider = state
        .providers
        .get(&route.provider)
        .ok_or_else(|| ApiError::ProviderNotFound(route.provider.clone()))?;

    // Record active request
    MetricsCollector::inc_active_requests(&route.provider);

    let result = if is_streaming {
        // Handle streaming response
        match provider
            .stream_completion(request.clone(), &route.api_key)
            .await
        {
            Ok(stream) => {
                MetricsCollector::dec_active_requests(&route.provider);
                MetricsCollector::record_request(&request.model, &route.provider, true);
                state
                    .load_balancer
                    .record_success(
                        &route.provider,
                        &route.target_model,
                        start_time.elapsed().as_millis() as u64,
                    )
                    .await;

                let mut response_builder = Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/event-stream")
                    .header("Cache-Control", "no-cache")
                    .header("Connection", "keep-alive");

                // Add rate limit headers to streaming response
                if let Some(ref status) = rate_limit_status {
                    if let Some(requests_remaining) = status.requests_remaining {
                        response_builder = response_builder
                            .header("X-RateLimit-Limit-Requests", requests_remaining.to_string());
                    }
                    if let Some(tokens_remaining) = status.tokens_remaining {
                        response_builder = response_builder
                            .header("X-RateLimit-Limit-Tokens", tokens_remaining.to_string());
                    }
                    if let Some(reset_at) = status.reset_at {
                        response_builder =
                            response_builder.header("X-RateLimit-Reset", reset_at.to_string());
                    }
                }

                Ok(response_builder
                    .body(axum::body::Body::from_stream(stream))
                    .unwrap())
            }
            Err(e) => {
                MetricsCollector::dec_active_requests(&route.provider);
                MetricsCollector::record_request(&request.model, &route.provider, false);
                state
                    .load_balancer
                    .record_error(&route.provider, &route.target_model)
                    .await;
                Err(e)
            }
        }
    } else {
        // Handle regular response
        let provider_call_start = std::time::Instant::now();
        tracing::info!("🌐 Calling provider: {}", route.provider);
        match provider.complete(request.clone(), &route.api_key).await {
            Ok(response) => {
                tracing::info!(
                    "🌐 Provider call completed: {:?}",
                    provider_call_start.elapsed()
                );
                let latency_ms = start_time.elapsed().as_millis() as i64;
                let latency_secs = latency_ms as f64 / 1000.0;

                // Calculate cost
                let cost = state.cost_calculator.calculate_cost(
                    &request.model,
                    response.usage.prompt_tokens,
                    response.usage.completion_tokens,
                );

                // Record metrics
                MetricsCollector::dec_active_requests(&route.provider);
                MetricsCollector::record_request(&request.model, &route.provider, true);
                MetricsCollector::record_tokens(
                    &request.model,
                    &route.provider,
                    response.usage.prompt_tokens,
                    response.usage.completion_tokens,
                );
                MetricsCollector::record_cost(&request.model, &route.provider, cost);
                MetricsCollector::record_latency(&request.model, &route.provider, latency_secs);

                // Record in database
                if state.database.is_enabled() {
                    let _ = state
                        .database
                        .record_usage(
                            &request.model,
                            &route.provider,
                            response.usage.prompt_tokens,
                            response.usage.completion_tokens,
                            response.usage.total_tokens,
                            cost,
                            latency_ms,
                            request.user.clone(),
                            false,
                            None,
                        )
                        .await;
                }

                // Update load balancer
                state
                    .load_balancer
                    .record_success(&route.provider, &route.target_model, latency_ms as u64)
                    .await;

                // Cache the response
                if state.cache.is_enabled() {
                    let cache_store_start = std::time::Instant::now();
                    let cache_key = state.cache.generate_cache_key(
                        &request.model,
                        &serde_json::to_string(&request.messages).unwrap_or_default(),
                    );
                    let _ = state.cache.set(&cache_key, &response).await;
                    tracing::info!("💾 Cache store: {:?}", cache_store_start.elapsed());
                }

                tracing::info!("✅ Total time: {:?}", start_time.elapsed());
                let mut final_response = Json(response).into_response();
                if let Some(ref status) = rate_limit_status {
                    final_response = add_rate_limit_headers(final_response, status);
                }
                Ok(final_response)
            }
            Err(e) => {
                let latency_ms = start_time.elapsed().as_millis() as i64;

                MetricsCollector::dec_active_requests(&route.provider);
                MetricsCollector::record_request(&request.model, &route.provider, false);
                state
                    .load_balancer
                    .record_error(&route.provider, &route.target_model)
                    .await;

                // Record error in database
                if state.database.is_enabled() {
                    let _ = state
                        .database
                        .record_usage(
                            &request.model,
                            &route.provider,
                            0,
                            0,
                            0,
                            0.0,
                            latency_ms,
                            request.user.clone(),
                            false,
                            Some(e.to_string()),
                        )
                        .await;
                }

                Err(e)
            }
        }
    };

    result
}

async fn list_models(State(state): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    // DashMap provides lock-free iteration
    let models: Vec<serde_json::Value> = state
        .model_routes
        .iter()
        .map(|entry| {
            serde_json::json!({
                "id": entry.key(),
                "object": "model",
                "owned_by": "llm-gateway",
                "permission": []
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "object": "list",
        "data": models
    })))
}

async fn list_providers(State(state): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    // Group models by provider
    let mut provider_map: HashMap<String, Vec<String>> = HashMap::new();

    for entry in state.model_routes.iter() {
        let model_name = entry.key().clone();
        let provider_name = entry.value().provider.clone();

        provider_map
            .entry(provider_name)
            .or_insert_with(Vec::new)
            .push(model_name);
    }

    // Build provider objects with metadata (include ALL providers)
    let providers: Vec<serde_json::Value> = state
        .providers
        .keys()
        .map(|provider_id| {
            let configured_models = provider_map.get(provider_id).cloned().unwrap_or_default();
            let is_configured = !configured_models.is_empty();

            // Get provider metadata from centralized config
            let endpoint = provider_config::get_endpoint(provider_id);
            let default_models = provider_config::get_primary_models(provider_id);

            // Use configured models if available, otherwise show default models
            let models: Vec<String> = if is_configured {
                configured_models
            } else {
                default_models.iter().map(|s| s.to_string()).collect()
            };

            let status = if is_configured { "active" } else { "inactive" };

            serde_json::json!({
                "id": provider_id,
                "name": capitalize_provider_name(provider_id),
                "status": status,
                "models": models,
                "endpoint": endpoint,
                "api_key_configured": is_configured,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "object": "list",
        "data": providers
    })))
}

fn capitalize_provider_name(provider_id: &str) -> String {
    match provider_id {
        "anthropic" => "Anthropic".to_string(),
        "gemini" => "Google Gemini".to_string(),
        "openai" => "OpenAI".to_string(),
        "azure" => "Azure OpenAI".to_string(),
        _ => provider_id.to_string(),
    }
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn metrics_handler() -> impl IntoResponse {
    match MetricsCollector::export_metrics() {
        Ok(metrics) => (StatusCode::OK, metrics).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to export metrics: {}", e),
        )
            .into_response(),
    }
}

async fn stats_handler(State(state): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    if !state.database.is_enabled() {
        return Ok(Json(serde_json::json!({
            "error": "Database not enabled, stats unavailable"
        })));
    }

    let stats = state.database.get_usage_stats(7).await?;
    let recent_usage = state.database.get_recent_usage(10).await?;
    let health_stats = state.load_balancer.get_all_health_stats().await;

    Ok(Json(serde_json::json!({
        "usage_stats": stats,
        "recent_requests": recent_usage,
        "provider_health": health_stats,
        "cache_enabled": state.cache.is_enabled(),
        "database_enabled": state.database.is_enabled(),
    })))
}
