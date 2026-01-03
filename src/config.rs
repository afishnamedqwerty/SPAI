//! Configuration types for the ATHPTTGH framework

use crate::error::{Error, Result};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use dotenvy::dotenv;
use std::time::Duration;
use url::Url;

/// Model configuration for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model identifier (e.g., "anthropic/claude-sonnet-4")
    pub model: String,
    /// Temperature for sampling (0.0-2.0)
    pub temperature: f32,
    /// Maximum tokens for completion
    pub max_tokens: Option<u32>,
    /// Top-p sampling parameter
    pub top_p: Option<f32>,
    /// Frequency penalty
    pub frequency_penalty: Option<f32>,
    /// Presence penalty
    pub presence_penalty: Option<f32>,
}

impl ModelConfig {
    /// Create a new model configuration
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            temperature: 0.7,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
        }
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set the maximum tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the top-p parameter
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }
}

/// Provider preferences for OpenRouter routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPreferences {
    /// Preferred providers in priority order
    pub preferred: Vec<String>,
    /// Providers to exclude
    pub excluded: Vec<String>,
    /// Optimization target
    pub optimization: OptimizationTarget,
}

impl Default for ProviderPreferences {
    fn default() -> Self {
        Self {
            preferred: vec![],
            excluded: vec![],
            optimization: OptimizationTarget::Balanced,
        }
    }
}

/// Optimization target for provider selection
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationTarget {
    /// Optimize for lower cost
    LowerCost,
    /// Balanced cost and performance
    Balanced,
    /// Optimize for performance
    Performance,
}

/// OpenRouter client configuration
#[derive(Clone)]
pub struct OpenRouterConfig {
    /// API key (loaded from environment variable)
    pub api_key: SecretString,
    /// Base URL for OpenRouter API
    pub base_url: Url,
    /// Default model for agents
    pub default_model: String,
    /// Provider preferences
    pub provider_preferences: ProviderPreferences,
    /// Fallback models if primary unavailable
    pub fallback_models: Vec<String>,
    /// Maximum retries on failure
    pub max_retries: u32,
    /// Request timeout
    pub timeout: Duration,
    /// App name for OpenRouter tracking
    pub app_name: String,
}

impl OpenRouterConfig {
    /// Create a new OpenRouter configuration from environment
    pub fn from_env() -> Result<Self> {
        // Load .env if present so local development picks up OPENROUTER_API_KEY
        let _ = dotenv();

        let api_key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| Error::config("OPENROUTER_API_KEY environment variable not set"))?;

        Ok(Self {
            api_key: SecretString::from(api_key),
            base_url: Url::parse("https://openrouter.ai/api/v1")
                .expect("valid OpenRouter URL"),
            default_model: presets::BALANCED.to_string(),
            provider_preferences: ProviderPreferences::default(),
            fallback_models: vec![
                presets::FAST.to_string(),
                presets::FREE_TIER.to_string(),
            ],
            max_retries: 3,
            timeout: Duration::from_secs(120),
            app_name: "ATHPTTGH Agent Harness".to_string(),
        })
    }

    /// Create a new OpenRouter configuration with a specific API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: SecretString::from(api_key.into()),
            base_url: Url::parse("https://openrouter.ai/api/v1")
                .expect("valid OpenRouter URL"),
            default_model: presets::BALANCED.to_string(),
            provider_preferences: ProviderPreferences::default(),
            fallback_models: vec![
                presets::FAST.to_string(),
                presets::FREE_TIER.to_string(),
            ],
            max_retries: 3,
            timeout: Duration::from_secs(120),
            app_name: "ATHPTTGH Agent Harness".to_string(),
        }
    }

    /// Set the base URL
    pub fn with_base_url(mut self, base_url: Url) -> Self {
        self.base_url = base_url;
        self
    }

    /// Set the default model
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Set provider preferences
    pub fn with_provider_preferences(mut self, preferences: ProviderPreferences) -> Self {
        self.provider_preferences = preferences;
        self
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the app name
    pub fn with_app_name(mut self, app_name: impl Into<String>) -> Self {
        self.app_name = app_name.into();
        self
    }

    /// Get the API key as a string
    pub fn api_key(&self) -> &str {
        self.api_key.expose_secret()
    }
}

impl std::fmt::Debug for OpenRouterConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenRouterConfig")
            .field("api_key", &"***REDACTED***")
            .field("base_url", &self.base_url)
            .field("default_model", &self.default_model)
            .field("provider_preferences", &self.provider_preferences)
            .field("fallback_models", &self.fallback_models)
            .field("max_retries", &self.max_retries)
            .field("timeout", &self.timeout)
            .field("app_name", &self.app_name)
            .finish()
    }
}

/// Recommended model configurations
pub mod presets {
    /// Recommended for complex reasoning tasks
    pub const REASONING: &str = "anthropic/claude-opus-4";

    /// Balanced performance and cost
    pub const BALANCED: &str = "anthropic/claude-sonnet-4";

    /// Fast responses, lower cost
    pub const FAST: &str = "anthropic/claude-haiku-4";

    /// Optimized for coding tasks
    pub const CODING: &str = "anthropic/claude-sonnet-4";

    /// Free tier model
    pub const FREE_TIER: &str = "meta-llama/llama-3.3-70b-instruct:free";

    /// GPT-4 Turbo
    pub const GPT4_TURBO: &str = "openai/gpt-4-turbo";

    /// GPT-4o
    pub const GPT4O: &str = "openai/gpt-4o";

    /// Gemini Pro
    pub const GEMINI_PRO: &str = "google/gemini-pro-1.5";
}
