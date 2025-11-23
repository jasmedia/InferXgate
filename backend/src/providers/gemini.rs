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
pub struct GeminiProvider {
    client: Arc<Client>,
}

impl GeminiProvider {
    pub fn new() -> Self {
        info!("🔧 Initializing GeminiProvider with connection pooling");

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
            .expect("Failed to create HTTP client for GeminiProvider");

        info!("✅ GeminiProvider HTTP client configured with connection pooling");

        Self {
            client: Arc::new(client),
        }
    }

    fn convert_messages(&self, messages: &[Message]) -> Vec<GeminiContent> {
        let mut contents = Vec::new();

        for msg in messages {
            let role = if msg.role == "assistant" {
                "model"
            } else {
                "user"
            };

            let parts = match &msg.content {
                MessageContent::Text(text) => vec![GeminiPart::Text { text: text.clone() }],
                MessageContent::Parts(parts) => {
                    parts
                        .iter()
                        .map(|part| match part {
                            ContentPart::Text { text } => GeminiPart::Text { text: text.clone() },
                            ContentPart::ImageUrl { image_url } => {
                                // In a real implementation, we'd need to handle base64 images
                                GeminiPart::Text {
                                    text: format!("[Image: {}]", image_url.url),
                                }
                            }
                        })
                        .collect()
                }
            };

            contents.push(GeminiContent {
                role: role.to_string(),
                parts,
            });
        }

        contents
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_settings: Option<Vec<SafetySetting>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum GeminiPart {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SafetySetting {
    category: String,
    threshold: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: GeminiContent,
    finish_reason: Option<String>,
    index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_ratings: Option<Vec<SafetyRating>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SafetyRating {
    category: String,
    probability: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMetadata {
    prompt_token_count: i32,
    candidates_token_count: i32,
    total_token_count: i32,
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    async fn complete(
        &self,
        request: ChatCompletionRequest,
        api_key: &str,
    ) -> ApiResult<ChatCompletionResponse> {
        debug!("Gemini completion request for model: {}", request.model);

        let contents = self.convert_messages(&request.messages);

        let generation_config = Some(GenerationConfig {
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: None,
            max_output_tokens: request.max_tokens,
            stop_sequences: request.stop,
        });

        let gemini_request = GeminiRequest {
            contents,
            generation_config,
            safety_settings: Some(vec![SafetySetting {
                category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                threshold: "BLOCK_ONLY_HIGH".to_string(),
            }]),
        };

        let url = format!(
            "{}:generateContent?key={}",
            Self::get_model_endpoint(&request.model),
            api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&gemini_request)
            .send()
            .await
            .map_err(|e| ApiError::ProviderError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            error!("Gemini API error: {} - {}", status, error_text);
            return Err(ApiError::ProviderError(format!(
                "Gemini API error: {} - {}",
                status, error_text
            )));
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .map_err(|e| ApiError::ProviderError(format!("Failed to parse response: {}", e)))?;

        // Convert to OpenAI format
        let candidate = gemini_response
            .candidates
            .first()
            .ok_or_else(|| ApiError::ProviderError("No candidates in response".to_string()))?;

        let content = candidate
            .content
            .parts
            .iter()
            .filter_map(|part| match part {
                GeminiPart::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        let usage = gemini_response.usage_metadata.unwrap_or(UsageMetadata {
            prompt_token_count: 0,
            candidates_token_count: 0,
            total_token_count: 0,
        });

        Ok(ChatCompletionResponse {
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model: request.model.clone(),
            choices: vec![Choice {
                index: 0,
                message: Message {
                    role: "assistant".to_string(),
                    content: MessageContent::Text(content),
                    name: None,
                },
                finish_reason: candidate
                    .finish_reason
                    .clone()
                    .unwrap_or_else(|| "stop".to_string()),
            }],
            usage: Usage {
                prompt_tokens: usage.prompt_token_count,
                completion_tokens: usage.candidates_token_count,
                total_tokens: usage.total_token_count,
            },
        })
    }

    async fn stream_completion(
        &self,
        request: ChatCompletionRequest,
        api_key: &str,
    ) -> ApiResult<Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>> {
        debug!("Gemini streaming request for model: {}", request.model);

        let contents = self.convert_messages(&request.messages);

        let generation_config = Some(GenerationConfig {
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: None,
            max_output_tokens: request.max_tokens,
            stop_sequences: request.stop,
        });

        let gemini_request = GeminiRequest {
            contents,
            generation_config,
            safety_settings: None,
        };

        let url = format!(
            "{}:streamGenerateContent?key={}",
            Self::get_model_endpoint(&request.model),
            api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&gemini_request)
            .send()
            .await
            .map_err(|e| ApiError::ProviderError(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(ApiError::ProviderError(format!(
                "Gemini API error: {} - {}",
                status, error_text
            )));
        }

        // Convert the response stream to SSE format
        let model = request.model.clone();
        let stream = response.bytes_stream().map(move |chunk| {
            match chunk {
                Ok(bytes) => {
                    // Parse the response and convert to OpenAI format
                    let data = String::from_utf8_lossy(&bytes);

                    // This is a simplified version - in production, you'd properly parse the stream
                    let openai_event = serde_json::json!({
                        "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                        "object": "chat.completion.chunk",
                        "created": chrono::Utc::now().timestamp(),
                        "model": model.clone(),
                        "choices": [{
                            "index": 0,
                            "delta": {
                                "content": data.trim()
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
        "gemini"
    }

    fn supported_models(&self) -> Vec<String> {
        provider_config::get_supported_models("gemini")
    }
}

impl GeminiProvider {
    fn get_model_endpoint(model: &str) -> String {
        format!("{}/{}", provider_config::gemini::API_URL, model)
    }
}
