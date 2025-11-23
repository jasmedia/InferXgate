use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Forbidden")]
    Forbidden,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("External API error: {0}")]
    ExternalApiError(String),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Internal server error")]
    InternalServerError,

    #[error("Request timeout")]
    RequestTimeout,

    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Rate limit error: {0}")]
    RateLimitError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message, error_type) = match &self {
            ApiError::ModelNotFound(msg) => (StatusCode::NOT_FOUND, msg.clone(), "ModelNotFound"),
            ApiError::ProviderNotFound(msg) => {
                (StatusCode::NOT_FOUND, msg.clone(), "ProviderNotFound")
            }
            ApiError::ProviderError(msg) => (StatusCode::BAD_GATEWAY, msg.clone(), "ProviderError"),
            ApiError::InvalidRequest(msg) => {
                (StatusCode::BAD_REQUEST, msg.clone(), "InvalidRequest")
            }
            ApiError::AuthenticationFailed => (
                StatusCode::UNAUTHORIZED,
                "Authentication failed".to_string(),
                "AuthenticationFailed",
            ),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_string(), "Forbidden"),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone(), "NotFound"),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone(), "BadRequest"),
            ApiError::ExternalApiError(msg) => {
                (StatusCode::BAD_GATEWAY, msg.clone(), "ExternalApiError")
            }
            ApiError::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg.clone(),
                "InternalError",
            ),
            ApiError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate limit exceeded".to_string(),
                "RateLimitExceeded",
            ),
            ApiError::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
                "InternalServerError",
            ),
            ApiError::RequestTimeout => (
                StatusCode::REQUEST_TIMEOUT,
                "Request timeout".to_string(),
                "RequestTimeout",
            ),
            ApiError::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Service unavailable".to_string(),
                "ServiceUnavailable",
            ),
            ApiError::DatabaseError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg.clone(),
                "DatabaseError",
            ),
            ApiError::CacheError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone(), "CacheError")
            }
            ApiError::RateLimitError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                msg.clone(),
                "RateLimitError",
            ),
        };

        let body = Json(json!({
            "error": {
                "message": error_message,
                "type": error_type,
                "code": status.as_u16(),
            }
        }));

        (status, body).into_response()
    }
}
