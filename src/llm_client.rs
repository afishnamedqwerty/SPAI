//! Unified LLM client trait for both remote (OpenRouter) and local (vLLM/SGLang) models

use crate::error::Result;
use crate::openrouter::{CompletionRequest, CompletionResponse, CompletionStream};
use async_trait::async_trait;

/// Unified trait for LLM clients (both remote and local)
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Send a completion request
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Stream a completion request
    async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream>;

    /// Get the client type for debugging/logging
    fn client_type(&self) -> &str;

    /// Get the base URL (for local models) or endpoint (for remote)
    fn endpoint(&self) -> &str;
}
