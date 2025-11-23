use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::{error::ApiResult, ChatCompletionRequest, ChatCompletionResponse};

pub mod anthropic;
pub mod azure;
pub mod gemini;
pub mod openai;

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn complete(
        &self,
        request: ChatCompletionRequest,
        api_key: &str,
    ) -> ApiResult<ChatCompletionResponse>;

    async fn stream_completion(
        &self,
        request: ChatCompletionRequest,
        api_key: &str,
    ) -> ApiResult<Pin<Box<dyn Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send>>>;

    fn name(&self) -> &str;

    fn supported_models(&self) -> Vec<String>;
}
