//! Error types for the ATHPTTGH framework

use thiserror::Error;

/// Result type alias for ATHPTTGH operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for the ATHPTTGH framework
#[derive(Debug, Error)]
pub enum Error {
    /// Error from the OpenRouter API
    #[error("OpenRouter API error: {0}")]
    OpenRouter(String),

    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Agent error
    #[error("Agent error: {0}")]
    Agent(String),

    /// Tool execution error
    #[error("Tool execution error: {tool}: {message}")]
    ToolExecution { tool: String, message: String },

    /// Handoff error
    #[error("Handoff error: {0}")]
    Handoff(String),

    /// Guardrail violation
    #[error("Guardrail violation: {guardrail}: {reason}")]
    GuardrailViolation { guardrail: String, reason: String },

    /// Approval denied
    #[error("Approval denied: {0}")]
    ApprovalDenied(String),

    /// Approval timeout
    #[error("Approval timeout: {0}")]
    ApprovalTimeout(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Context window exceeded
    #[error("Context window exceeded: {current} tokens (max: {max})")]
    ContextWindowExceeded { current: u64, max: u64 },

    /// Maximum loops exceeded
    #[error("Maximum loops exceeded: {0}")]
    MaxLoopsExceeded(u32),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Tracing error
    #[error("Tracing error: {0}")]
    Tracing(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON Schema validation error
    #[error("JSON Schema validation error: {0}")]
    JsonSchema(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Create an OpenRouter error
    pub fn openrouter(msg: impl Into<String>) -> Self {
        Self::OpenRouter(msg.into())
    }

    /// Create an agent error
    pub fn agent(msg: impl Into<String>) -> Self {
        Self::Agent(msg.into())
    }

    /// Create a tool execution error
    pub fn tool_execution(tool: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ToolExecution {
            tool: tool.into(),
            message: message.into(),
        }
    }

    /// Create a handoff error
    pub fn handoff(msg: impl Into<String>) -> Self {
        Self::Handoff(msg.into())
    }

    /// Create a guardrail violation error
    pub fn guardrail_violation(guardrail: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::GuardrailViolation {
            guardrail: guardrail.into(),
            reason: reason.into(),
        }
    }

    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a storage error
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }

    /// Create an other error
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}
