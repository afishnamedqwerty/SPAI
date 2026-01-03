//! Handoff protocol and inter-agent delegation

use crate::react::{Observation, ReActTrace};
use crate::types::AgentId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Handoff request from one agent to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Handoff {
    /// Source agent initiating the handoff
    pub source: AgentId,
    /// Target agent receiving control
    pub target: AgentId,
    /// Reason for handoff (for tracing and debugging)
    pub reason: String,
    /// Context to transfer to target agent
    pub context: HandoffContext,
    /// Whether to return control after target completes
    pub return_control: bool,
}

impl Handoff {
    /// Create a new handoff
    pub fn new(
        source: AgentId,
        target: AgentId,
        reason: impl Into<String>,
        context: HandoffContext,
    ) -> Self {
        Self {
            source,
            target,
            reason: reason.into(),
            context,
            return_control: true,
        }
    }

    /// Set whether to return control
    pub fn with_return_control(mut self, return_control: bool) -> Self {
        self.return_control = return_control;
        self
    }
}

/// Context to transfer during handoff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffContext {
    /// Original user query
    pub original_query: String,
    /// Accumulated observations from source agent
    pub observations: Vec<Observation>,
    /// Partial reasoning trace
    pub trace: ReActTrace,
    /// Custom metadata for the handoff
    pub metadata: HashMap<String, serde_json::Value>,
}

impl HandoffContext {
    /// Create a new handoff context
    pub fn new(original_query: impl Into<String>) -> Self {
        Self {
            original_query: original_query.into(),
            observations: Vec::new(),
            trace: ReActTrace::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add an observation
    pub fn with_observation(mut self, observation: Observation) -> Self {
        self.observations.push(observation);
        self
    }

    /// Set the trace
    pub fn with_trace(mut self, trace: ReActTrace) -> Self {
        self.trace = trace;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Handoff strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HandoffStrategy {
    /// Direct transfer - target takes full control
    Direct,
    /// Collaborative - both agents work together
    Collaborative,
    /// Supervised - source monitors target's progress
    Supervised {
        /// Check interval
        check_interval: Duration,
    },
    /// Cascading - target may further delegate
    Cascading {
        /// Maximum delegation depth
        max_depth: u32,
    },
}

impl Default for HandoffStrategy {
    fn default() -> Self {
        Self::Direct
    }
}
