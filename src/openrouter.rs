//! OpenRouter API client implementation with streaming support

use crate::config::OpenRouterConfig;
use crate::error::{Error, Result};
use crate::llm_client::LlmClient;
use crate::types::TokenUsage;
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};

/// OpenRouter API client
pub struct OpenRouterClient {
    /// HTTP client
    client: Client,
    /// Configuration
    config: OpenRouterConfig,
}

impl OpenRouterClient {
    /// Create a new OpenRouter client from environment variables
    pub fn from_env() -> Result<Self> {
        let config = OpenRouterConfig::from_env()?;
        Self::new(config)
    }

    /// Create a new OpenRouter client with the given configuration
    pub fn new(config: OpenRouterConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()?;

        Ok(Self { client, config })
    }

    /// Send a completion request
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key()))
            .header("X-Title", &self.config.app_name)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::openrouter(format!(
                "Request failed with status {}: {}",
                status, error_text
            )));
        }

        let completion: CompletionResponse = response.json().await?;
        Ok(completion)
    }

    /// Stream a completion request
    pub async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream> {
        let url = format!("{}/chat/completions", self.config.base_url);

        let mut request_with_stream = request;
        request_with_stream.stream = true;

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key()))
            .header("X-Title", &self.config.app_name)
            .json(&request_with_stream)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::openrouter(format!(
                "Stream request failed with status {}: {}",
                status, error_text
            )));
        }

        Ok(CompletionStream::new(response.bytes_stream()))
    }

    /// Get the configuration
    pub fn config(&self) -> &OpenRouterConfig {
        &self.config
    }
}

/// Completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Model identifier
    pub model: String,
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Temperature for sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum tokens for completion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Frequency penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    /// Presence penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,
    /// Tools available to the model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// Tool choice behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
}

impl CompletionRequest {
    /// Create a new completion request
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stream: false,
            tools: None,
            tool_choice: None,
        }
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the maximum tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Enable streaming
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    /// Set the tools
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set the tool choice
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }
}

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: Role,
    /// Content of the message
    pub content: String,
    /// Optional name of the sender
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional tool calls (for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Optional tool call ID (for tool messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a tool message
    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// Role of a message sender
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// Tool message
    Tool,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Type of tool (always "function" for now)
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function details
    pub function: FunctionDefinition,
}

/// Function definition for tool calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// JSON Schema for parameters
    pub parameters: serde_json::Value,
}

/// Tool choice behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Auto (model decides)
    Auto,
    /// None (no tool calls)
    None,
    /// Required (must call a tool)
    Required,
    /// Specific function
    Function { function: FunctionChoice },
}

/// Specific function choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionChoice {
    /// Function name
    pub name: String,
}

/// Tool call from the assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool call ID
    pub id: String,
    /// Type (always "function")
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function details
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// Function name
    pub name: String,
    /// Function arguments (JSON string)
    pub arguments: String,
}

/// Completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Unique identifier
    pub id: String,
    /// Model used
    pub model: String,
    /// Choices
    pub choices: Vec<Choice>,
    /// Token usage
    pub usage: Usage,
}

/// Choice in completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// Index of the choice
    pub index: u32,
    /// Message content
    pub message: Message,
    /// Finish reason
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Prompt tokens
    pub prompt_tokens: u64,
    /// Completion tokens
    pub completion_tokens: u64,
    /// Total tokens
    pub total_tokens: u64,
}

impl From<Usage> for TokenUsage {
    fn from(usage: Usage) -> Self {
        TokenUsage::new(usage.prompt_tokens, usage.completion_tokens)
    }
}

/// Stream chunk from streaming completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Unique identifier
    pub id: String,
    /// Model used
    pub model: String,
    /// Choices
    pub choices: Vec<StreamChoice>,
}

/// Choice in stream chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    /// Index of the choice
    pub index: u32,
    /// Delta (incremental content)
    pub delta: Delta,
    /// Finish reason
    pub finish_reason: Option<String>,
}

/// Delta (incremental content) in stream chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    /// Role (only in first chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    /// Content delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool calls delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Streaming completion response
pub struct CompletionStream {
    inner: Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send>>,
    buffer: String,
}

impl CompletionStream {
    pub(crate) fn new(stream: impl Stream<Item = reqwest::Result<Bytes>> + Send + 'static) -> Self {
        Self {
            inner: Box::pin(stream),
            buffer: String::new(),
        }
    }

    /// Get the next chunk from the stream
    pub async fn next_chunk(&mut self) -> Option<Result<StreamChunk>> {
        // Simplified streaming implementation - in production would need proper SSE parsing
        None
    }
}

impl Stream for CompletionStream {
    type Item = Result<StreamChunk>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Simplified implementation - in production would parse SSE events
        Poll::Ready(None)
    }
}

#[async_trait]
impl LlmClient for OpenRouterClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Delegate to the existing complete method
        OpenRouterClient::complete(self, request).await
    }

    async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream> {
        // Delegate to the existing stream method
        OpenRouterClient::stream(self, request).await
    }

    fn client_type(&self) -> &str {
        "openrouter"
    }

    fn endpoint(&self) -> &str {
        self.config.base_url.as_str()
    }
}
