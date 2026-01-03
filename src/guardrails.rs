//! Guardrails for input/output validation and safety

use crate::agent::AgentOutput;
use crate::error::Result;
use crate::types::AgentId;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context for guardrail checks
#[derive(Debug, Clone)]
pub struct GuardrailContext {
    /// Agent being checked
    pub agent_id: AgentId,
    /// Additional context data
    pub data: HashMap<String, serde_json::Value>,
}

impl GuardrailContext {
    /// Create a new guardrail context
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            data: HashMap::new(),
        }
    }
}

/// Result of a guardrail check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailResult {
    /// Whether the check passed
    pub passed: bool,
    /// Whether to halt execution (tripwire)
    pub tripwire_triggered: bool,
    /// Explanation of the result
    pub reasoning: String,
    /// Suggested modification (if applicable)
    pub suggested_modification: Option<String>,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
}

impl GuardrailResult {
    /// Create a passing result
    pub fn pass(reasoning: impl Into<String>) -> Self {
        Self {
            passed: true,
            tripwire_triggered: false,
            reasoning: reasoning.into(),
            suggested_modification: None,
            confidence: 1.0,
        }
    }

    /// Create a failing result
    pub fn fail(reasoning: impl Into<String>) -> Self {
        Self {
            passed: false,
            tripwire_triggered: false,
            reasoning: reasoning.into(),
            suggested_modification: None,
            confidence: 1.0,
        }
    }

    /// Create a failing result that triggers a tripwire
    pub fn tripwire(reasoning: impl Into<String>) -> Self {
        Self {
            passed: false,
            tripwire_triggered: true,
            reasoning: reasoning.into(),
            suggested_modification: None,
            confidence: 1.0,
        }
    }

    /// Set confidence score
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    /// Set suggested modification
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggested_modification = Some(suggestion.into());
        self
    }
}

/// Input guardrail trait
#[async_trait]
pub trait InputGuardrail: Send + Sync {
    /// Unique identifier
    fn id(&self) -> &str;

    /// Check input before agent processing
    async fn check(&self, input: &str, ctx: &GuardrailContext) -> Result<GuardrailResult>;
}

/// Output guardrail trait
#[async_trait]
pub trait OutputGuardrail: Send + Sync {
    /// Unique identifier
    fn id(&self) -> &str;

    /// Check output after agent processing
    async fn check(&self, output: &AgentOutput, ctx: &GuardrailContext) -> Result<GuardrailResult>;
}
