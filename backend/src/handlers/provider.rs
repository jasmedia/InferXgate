use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{error::ApiResult, provider_config, AppState};

#[derive(Debug, Deserialize)]
pub struct UpdateProviderRequest {
    pub provider_id: String,
    pub api_key: String,
    /// Azure-specific: Resource name for endpoint construction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure_resource_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateProviderResponse {
    pub success: bool,
    pub message: String,
    pub provider_id: String,
    pub models_configured: usize,
}

/// Update API key for a provider
/// This requires admin access (master key or admin user)
pub async fn update_provider_key(
    State(state): State<Arc<AppState>>,
    Json(request): Json<UpdateProviderRequest>,
) -> ApiResult<impl IntoResponse> {
    tracing::info!("🔧 Updating API key for provider: {}", request.provider_id);

    // Validate provider exists
    if !state.providers.contains_key(&request.provider_id) {
        return Ok((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("Provider '{}' not found", request.provider_id)
            })),
        )
            .into_response());
    }

    // Validate API key is not empty
    if request.api_key.trim().is_empty() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "API key cannot be empty"
            })),
        )
            .into_response());
    }

    // For Azure, validate and combine resource name with API key
    let api_key_to_store = if request.provider_id == "azure" {
        let resource_name = request.azure_resource_name.as_ref().ok_or_else(|| {
            crate::error::ApiError::BadRequest(
                "Azure provider requires azure_resource_name".to_string(),
            )
        })?;
        if resource_name.trim().is_empty() {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Azure resource name cannot be empty"
                })),
            )
                .into_response());
        }
        // Store as "resource_name:api_key" format
        format!("{}:{}", resource_name.trim(), request.api_key.trim())
    } else {
        request.api_key.clone()
    };

    // Get the list of models for this provider from centralized config
    let models_to_configure = provider_config::get_primary_models(&request.provider_id);

    // Update or create model routes for this provider
    let mut configured_count = 0;
    for model in models_to_configure {
        let route = crate::ModelRoute {
            provider: request.provider_id.clone(),
            target_model: model.to_string(),
            api_key: api_key_to_store.clone(),
        };

        state.model_routes.insert(model.to_string(), route);
        configured_count += 1;
    }

    // Store the API key in the database if available
    if state.database.is_enabled() {
        match state
            .database
            .store_provider_key(&request.provider_id, &api_key_to_store)
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "✅ Provider API key stored in database: {}",
                    request.provider_id
                );
            }
            Err(e) => {
                tracing::warn!(
                    "⚠️ Failed to store provider key in database: {}. Models will be configured but won't persist on restart.",
                    e
                );
            }
        }
    }

    tracing::info!(
        "✅ Updated {} models for provider: {}",
        configured_count,
        request.provider_id
    );

    Ok((
        StatusCode::OK,
        Json(UpdateProviderResponse {
            success: true,
            message: format!(
                "Successfully configured {} models for {}",
                configured_count, request.provider_id
            ),
            provider_id: request.provider_id,
            models_configured: configured_count,
        }),
    )
        .into_response())
}

/// Delete API key for a provider (removes all model routes)
pub async fn delete_provider_key(
    State(state): State<Arc<AppState>>,
    Json(request): Json<serde_json::Value>,
) -> ApiResult<impl IntoResponse> {
    let provider_id = request["provider_id"]
        .as_str()
        .ok_or_else(|| crate::error::ApiError::BadRequest("provider_id required".to_string()))?;

    tracing::info!("🗑️ Removing API key for provider: {}", provider_id);

    // Find and remove all model routes for this provider
    let keys_to_remove: Vec<String> = state
        .model_routes
        .iter()
        .filter(|entry| entry.value().provider == provider_id)
        .map(|entry| entry.key().clone())
        .collect();

    for key in &keys_to_remove {
        state.model_routes.remove(key);
    }

    // Remove from database if available
    if state.database.is_enabled() {
        match state.database.delete_provider_key(provider_id).await {
            Ok(_) => {
                tracing::info!("✅ Provider API key deleted from database: {}", provider_id);
            }
            Err(e) => {
                tracing::warn!("⚠️ Failed to delete provider key from database: {}", e);
            }
        }
    }

    tracing::info!(
        "✅ Removed {} models for provider: {}",
        keys_to_remove.len(),
        provider_id
    );

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": format!("Successfully removed {} models for {}", keys_to_remove.len(), provider_id),
            "provider_id": provider_id,
            "models_removed": keys_to_remove.len()
        })),
    )
        .into_response())
}
