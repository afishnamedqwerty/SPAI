//! vLLM local model client with OpenAI-compatible API support
//!
//! This module provides a client for locally-hosted models served via vLLM,
//! which exposes an OpenAI-compatible HTTP API.
//!
//! # Quick Start with OLMo-7B-Reasoning
//!
//! 1. Install vLLM:
//!    ```bash
//!    pip install vllm
//!    ```
//!
//! 2. Start the vLLM server:
//!    ```bash
//!    python -m vllm.entrypoints.openai.api_server \
//!        --model allenai/OLMo-7B-1124-Instruct \
//!        --host 0.0.0.0 \
//!        --port 8000 \
//!        --dtype auto \
//!        --max-model-len 4096
//!    ```
//!
//! 3. Use with SPAI:
//!    ```rust
//!    let client = VllmClient::new("http://localhost:8000")?;
//!    let agent = Agent::builder()
//!        .client(Arc::new(client))
//!        .build()?;
//!    ```

use crate::error::{Error, Result};
use crate::llm_client::LlmClient;
use crate::openrouter::{CompletionRequest, CompletionResponse, CompletionStream};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// vLLM client configuration
#[derive(Debug, Clone)]
pub struct VllmConfig {
    /// Base URL of the vLLM server (e.g., "http://localhost:8000")
    pub base_url: String,
    /// Request timeout
    pub timeout: Duration,
    /// Optional API key (for secured vLLM deployments)
    pub api_key: Option<String>,
}

impl VllmConfig {
    /// Create a new vLLM configuration
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            timeout: Duration::from_secs(300), // 5 minutes for long reasoning
            api_key: None,
        }
    }

    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let base_url = std::env::var("VLLM_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8000".to_string());
        let api_key = std::env::var("VLLM_API_KEY").ok();

        Ok(Self {
            base_url,
            timeout: Duration::from_secs(300),
            api_key,
        })
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }
}

/// vLLM client for local model inference
pub struct VllmClient {
    /// HTTP client
    client: Client,
    /// Configuration
    config: VllmConfig,
}

impl VllmClient {
    /// Create a new vLLM client from environment variables
    pub fn from_env() -> Result<Self> {
        let config = VllmConfig::from_env()?;
        Self::new(config)
    }

    /// Create a new vLLM client with the given configuration
    pub fn new(config: VllmConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(config.timeout)
            .build()?;

        Ok(Self { client, config })
    }

    /// Get the configuration
    pub fn config(&self) -> &VllmConfig {
        &self.config
    }

    /// Check if the vLLM server is healthy
    pub async fn health_check(&self) -> Result<VllmHealth> {
        let url = format!("{}/health", self.config.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(Error::config(format!(
                "vLLM health check failed: {}",
                response.status()
            )));
        }

        // vLLM /health endpoint returns empty body, just check status
        Ok(VllmHealth {
            status: "ok".to_string(),
        })
    }

    /// Get model information from the vLLM server
    pub async fn get_models(&self) -> Result<ModelsResponse> {
        let url = format!("{}/v1/models", self.config.base_url);

        let mut request = self.client.get(&url);
        if let Some(ref api_key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(Error::config(format!(
                "Failed to get models: {}",
                response.status()
            )));
        }

        Ok(response.json().await?)
    }
}

#[async_trait]
impl LlmClient for VllmClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let url = format!("{}/v1/chat/completions", self.config.base_url);

        let mut http_request = self.client.post(&url).json(&request);

        if let Some(ref api_key) = self.config.api_key {
            http_request = http_request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = http_request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::openrouter(format!(
                "vLLM request failed with status {}: {}",
                status, error_text
            )));
        }

        let completion: CompletionResponse = response.json().await?;
        Ok(completion)
    }

    async fn stream(&self, request: CompletionRequest) -> Result<CompletionStream> {
        let url = format!("{}/v1/chat/completions", self.config.base_url);

        let mut request_with_stream = request;
        request_with_stream.stream = true;

        let mut http_request = self.client.post(&url).json(&request_with_stream);

        if let Some(ref api_key) = self.config.api_key {
            http_request = http_request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = http_request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::openrouter(format!(
                "vLLM stream request failed with status {}: {}",
                status, error_text
            )));
        }

        Ok(CompletionStream::new(response.bytes_stream()))
    }

    fn client_type(&self) -> &str {
        "vllm"
    }

    fn endpoint(&self) -> &str {
        &self.config.base_url
    }
}

/// vLLM health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VllmHealth {
    /// Server status
    pub status: String,
}

/// Models response from vLLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    /// Object type (always "list")
    pub object: String,
    /// List of available models
    pub data: Vec<ModelInfo>,
}

/// Information about a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model ID
    pub id: String,
    /// Object type (always "model")
    pub object: String,
    /// Creation timestamp
    pub created: u64,
    /// Owner organization
    pub owned_by: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vllm_config_from_env() {
        std::env::set_var("VLLM_BASE_URL", "http://localhost:8000");
        let config = VllmConfig::from_env().unwrap();
        assert_eq!(config.base_url, "http://localhost:8000");
    }

    #[test]
    fn test_vllm_config_builder() {
        let config = VllmConfig::new("http://localhost:9000")
            .with_timeout(Duration::from_secs(60))
            .with_api_key("test-key");

        assert_eq!(config.base_url, "http://localhost:9000");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.api_key, Some("test-key".to_string()));
    }
}
