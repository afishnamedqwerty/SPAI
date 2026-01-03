//! Workflow patterns for multi-agent orchestration

use crate::error::Result;
pub use crate::tracing_ext::PatternConfig;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Output from a workflow pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternOutput {
    /// Final output content
    pub content: String,
    /// Pattern-specific metadata
    pub metadata: serde_json::Value,
}

/// Workflow pattern trait
#[async_trait]
pub trait WorkflowPattern: Send + Sync {
    /// Execute the pattern with given agents and input
    async fn execute(
        &self,
        agents: &[String],
        input: &str,
        config: &PatternConfig,
    ) -> Result<PatternOutput>;

    /// Validate pattern configuration
    fn validate_config(&self, config: &PatternConfig) -> Result<()>;
}
