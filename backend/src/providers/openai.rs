use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::StreamExt;
use tracing::{debug, error, info};

use crate::{
    error::{ApiError, ApiResult},
    provider_config,
    providers::LLMProvider,
    ChatCompletionRequest, ChatCompletionResponse,
};

#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    client: Arc<Client>,
}

impl OpenAIProvider {
    pub fn new() -> Self {
        info!("🔧 Initializing OpenAIProvider with connection pooling");

        let client = Client::builder()
            // Connection pool settings
            .pool_max_idle_per_host(10) // Keep up to 10 idle connections per host
            .pool_idle_timeout(Duration::from_secs(90)) // Keep connections alive for 90s
            // Timeout settings
            .timeout(Duration::from_secs(120)) // Total request timeout (LLM calls can be long)
            .connect_timeout(Duration::from_secs(10)) // Connection establishment timeout
            // TCP settings
            .tcp_keepalive(Duration::from_secs(60)) // Send TCP keepalive every 60s
            .tcp_nodelay(true) // Disable Nagle's algorithm for lower latency
            // Build the client
            .build()
            .expect("Failed to create HTTP client for OpenAIProvider");

        info!("✅ OpenAIProvider HTTP client configured with connection pooling");

        Self {
            client: Arc::new(client),
        }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn complete(
        &self,
        request: ChatCompletionRequest,
        api_key: &str,
    ) -> ApiResult<ChatCompletionResponse> {
        debug!("OpenAI completion request for model: {}", request.model);

        // OpenAI API is already OpenAI-compatible, so we can pass through directly
        let response = self
            .client
            .post(provider_config::openai::API_URL)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ApiError::ProviderError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("OpenAI API error: {} - {}", status, error_text);
            return Err(ApiError::ProviderError(format!(
                "OpenAI API error: {} - {}",
                status, error_text
            )));
        }

        let openai_response: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ProviderError(format!("Failed to parse response: {}", e)))?;

        Ok(openai_response)
    }

    async fn stream_completion(
        &self,
        request: ChatCompletionRequest,
        api_key: &str,
    ) -> ApiResult<Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>> {
        debug!("OpenAI streaming request for model: {}", request.model);

        // Create a new request with stream enabled
        let mut streaming_request = request.clone();
        streaming_request.stream = Some(true);

        let response = self
            .client
            .post(provider_config::openai::API_URL)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&streaming_request)
            .send()
            .await
            .map_err(|e| ApiError::ProviderError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ApiError::ProviderError(format!(
                "OpenAI API error: {} - {}",
                status, error_text
            )));
        }

        // OpenAI already returns SSE format, so we can pass through directly
        let stream = response.bytes_stream().map(|chunk| match chunk {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        });

        Ok(Box::pin(stream))
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn supported_models(&self) -> Vec<String> {
        provider_config::get_supported_models("openai")
    }
}
