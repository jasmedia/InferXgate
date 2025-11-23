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
    ChatCompletionRequest, ChatCompletionResponse, Choice, ContentPart, Message, MessageContent,
    Usage,
};

#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    client: Arc<Client>,
}

impl AnthropicProvider {
    pub fn new() -> Self {
        info!("🔧 Initializing AnthropicProvider with connection pooling");

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
            // Note: Removed http2_prior_knowledge() - let client auto-negotiate
            // Build the client
            .build()
            .expect("Failed to create HTTP client for AnthropicProvider");

        info!("✅ AnthropicProvider HTTP client configured with connection pooling");

        Self {
            client: Arc::new(client),
        }
    }

    fn convert_message(&self, msg: &Message) -> AnthropicMessage {
        let content = match &msg.content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Parts(parts) => {
                // For multimodal, we'd need to handle this properly
                // For now, just extract text parts
                parts
                    .iter()
                    .filter_map(|part| match part {
                        ContentPart::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        };

        AnthropicMessage {
            role: if msg.role == "assistant" {
                "assistant".to_string()
            } else {
                "user".to_string()
            },
            content,
        }
    }
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<AnthropicContent>,
    model: String,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: i32,
    output_tokens: i32,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    delta: Option<AnthropicDelta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    delta_type: String,
    text: Option<String>,
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    async fn complete(
        &self,
        request: ChatCompletionRequest,
        api_key: &str,
    ) -> ApiResult<ChatCompletionResponse> {
        debug!("Anthropic completion request for model: {}", request.model);

        // Extract system message if present
        let mut system_message = None;
        let mut messages = Vec::new();

        for msg in &request.messages {
            if msg.role == "system" {
                system_message = Some(match &msg.content {
                    MessageContent::Text(text) => text.clone(),
                    MessageContent::Parts(_) => continue,
                });
            } else {
                messages.push(self.convert_message(msg));
            }
        }

        let anthropic_request = AnthropicRequest {
            model: request.model.clone(),
            messages,
            max_tokens: request.max_tokens.unwrap_or(1024),
            temperature: request.temperature,
            top_p: request.top_p,
            stop_sequences: request.stop,
            stream: Some(false),
        };

        let mut req = self
            .client
            .post(provider_config::anthropic::API_URL)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&anthropic_request);

        if system_message.is_some() {
            req = req.header("anthropic-beta", "messages-2023-12-15");
            // In a real implementation, we'd include the system message in the request body
        }

        let response = req
            .send()
            .await
            .map_err(|e| ApiError::ProviderError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Anthropic API error: {} - {}", status, error_text);
            return Err(ApiError::ProviderError(format!(
                "Anthropic API error: {} - {}",
                status, error_text
            )));
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ProviderError(format!("Failed to parse response: {}", e)))?;

        // Convert to OpenAI format
        let content = anthropic_response
            .content
            .into_iter()
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(ChatCompletionResponse {
            id: anthropic_response.id,
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: anthropic_response.model,
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: MessageContent::Text(content),
                    name: None,
                },
                finish_reason: anthropic_response
                    .stop_reason
                    .unwrap_or_else(|| "stop".to_string()),
            }],
            usage: Usage {
                prompt_tokens: anthropic_response.usage.input_tokens,
                completion_tokens: anthropic_response.usage.output_tokens,
                total_tokens: anthropic_response.usage.input_tokens
                    + anthropic_response.usage.output_tokens,
            },
        })
    }

    async fn stream_completion(
        &self,
        request: ChatCompletionRequest,
        api_key: &str,
    ) -> ApiResult<Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>> {
        debug!("Anthropic streaming request for model: {}", request.model);

        let mut messages = Vec::new();
        for msg in &request.messages {
            if msg.role != "system" {
                messages.push(self.convert_message(msg));
            }
        }

        let anthropic_request = AnthropicRequest {
            model: request.model.clone(),
            messages,
            max_tokens: request.max_tokens.unwrap_or(1024),
            temperature: request.temperature,
            top_p: request.top_p,
            stop_sequences: request.stop,
            stream: Some(true),
        };

        let response = self
            .client
            .post(provider_config::anthropic::API_URL)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| ApiError::ProviderError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ApiError::ProviderError(format!(
                "Anthropic API error: {} - {}",
                status, error_text
            )));
        }

        // Convert the response stream to SSE format
        let stream = response.bytes_stream().map(move |chunk| {
            match chunk {
                Ok(bytes) => {
                    // Parse the SSE data and convert to OpenAI format
                    let data = String::from_utf8_lossy(&bytes);

                    // This is a simplified version - in production, you'd properly parse SSE events
                    let openai_event = serde_json::json!({
                        "id": "chatcmpl-123",
                        "object": "chat.completion.chunk",
                        "created": chrono::Utc::now().timestamp(),
                        "model": request.model.clone(),
                        "choices": [{
                            "index": 0,
                            "delta": {
                                "content": data.trim_start_matches("data: ")
                            },
                            "finish_reason": null
                        }]
                    });

                    let sse_data = format!(
                        "data: {}\n\n",
                        serde_json::to_string(&openai_event).unwrap()
                    );
                    Ok(Bytes::from(sse_data))
                }
                Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
            }
        });

        Ok(Box::pin(stream))
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    fn supported_models(&self) -> Vec<String> {
        provider_config::get_supported_models("anthropic")
    }
}
