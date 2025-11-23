use async_trait::async_trait;
use bytes::Bytes;
use futures::Stream;
use reqwest::Client;
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
pub struct AzureProvider {
    client: Arc<Client>,
}

impl AzureProvider {
    pub fn new() -> Self {
        info!("🔧 Initializing AzureProvider with connection pooling");

        let client = Client::builder()
            // Connection pool settings
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90))
            // Timeout settings
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            // TCP settings
            .tcp_keepalive(Duration::from_secs(60))
            .tcp_nodelay(true)
            .build()
            .expect("Failed to create HTTP client for AzureProvider");

        info!("✅ AzureProvider HTTP client configured with connection pooling");

        Self {
            client: Arc::new(client),
        }
    }

    /// Parse the API key which contains "resource_name:actual_api_key"
    fn parse_api_key(api_key: &str) -> Result<(&str, &str), ApiError> {
        api_key.split_once(':').ok_or_else(|| {
            ApiError::ProviderError(
                "Azure API key must be in format 'resource_name:api_key'. Please reconfigure the Azure provider.".to_string(),
            )
        })
    }

    /// Build the Azure OpenAI API URL for a specific deployment
    fn build_url(resource_name: &str, model: &str) -> String {
        let deployment_name = provider_config::azure::get_deployment_name(model);
        format!(
            "https://{}.openai.azure.com/openai/deployments/{}/chat/completions?api-version={}",
            resource_name,
            deployment_name,
            provider_config::azure::API_VERSION
        )
    }
}

#[async_trait]
impl LLMProvider for AzureProvider {
    async fn complete(
        &self,
        request: ChatCompletionRequest,
        api_key: &str,
    ) -> ApiResult<ChatCompletionResponse> {
        debug!(
            "Azure OpenAI completion request for model: {}",
            request.model
        );

        let (resource_name, actual_api_key) = Self::parse_api_key(api_key)?;
        let url = Self::build_url(resource_name, &request.model);

        // Azure OpenAI uses the same request format as OpenAI
        let response = self
            .client
            .post(&url)
            .header("api-key", actual_api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ApiError::ProviderError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Azure OpenAI API error: {} - {}", status, error_text);
            return Err(ApiError::ProviderError(format!(
                "Azure OpenAI API error: {} - {}",
                status, error_text
            )));
        }

        let azure_response: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ProviderError(format!("Failed to parse response: {}", e)))?;

        Ok(azure_response)
    }

    async fn stream_completion(
        &self,
        request: ChatCompletionRequest,
        api_key: &str,
    ) -> ApiResult<Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>> {
        debug!(
            "Azure OpenAI streaming request for model: {}",
            request.model
        );

        let (resource_name, actual_api_key) = Self::parse_api_key(api_key)?;
        let url = Self::build_url(resource_name, &request.model);

        // Create a new request with stream enabled
        let mut streaming_request = request.clone();
        streaming_request.stream = Some(true);

        let response = self
            .client
            .post(&url)
            .header("api-key", actual_api_key)
            .header("Content-Type", "application/json")
            .json(&streaming_request)
            .send()
            .await
            .map_err(|e| ApiError::ProviderError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ApiError::ProviderError(format!(
                "Azure OpenAI API error: {} - {}",
                status, error_text
            )));
        }

        // Azure OpenAI returns SSE format compatible with OpenAI
        let stream = response.bytes_stream().map(|chunk| match chunk {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        });

        Ok(Box::pin(stream))
    }

    fn name(&self) -> &str {
        "azure"
    }

    fn supported_models(&self) -> Vec<String> {
        provider_config::get_supported_models("azure")
    }
}
