//! Orchestrator pattern trait and result types

use crate::error::Result;
use crate::Agent;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Output from an orchestrator pattern execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorResult {
    /// Final synthesized output
    pub content: String,
    /// Individual agent outputs
    pub agent_outputs: HashMap<String, AgentOutput>,
    /// Pattern-specific metadata
    pub metadata: OrchestratorMetadata,
}

/// Individual agent output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// Agent name
    pub agent_name: String,
    /// Agent's response content
    pub content: String,
    /// Number of ReAct loops executed
    pub loops_executed: usize,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Pattern execution metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorMetadata {
    /// Pattern type used
    pub pattern_type: String,
    /// Total execution time
    pub total_time_ms: u64,
    /// Number of agents involved
    pub agent_count: usize,
    /// Number of handoffs performed
    pub handoff_count: usize,
    /// Pattern-specific data
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl OrchestratorResult {
    /// Create a new result
    pub fn new(content: impl Into<String>, pattern_type: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            agent_outputs: HashMap::new(),
            metadata: OrchestratorMetadata {
                pattern_type: pattern_type.into(),
                total_time_ms: 0,
                agent_count: 0,
                handoff_count: 0,
                extra: HashMap::new(),
            },
        }
    }

    /// Add an agent output
    pub fn with_agent_output(mut self, output: AgentOutput) -> Self {
        self.agent_outputs.insert(output.agent_name.clone(), output);
        self.metadata.agent_count = self.agent_outputs.len();
        self
    }

    /// Set execution time
    pub fn with_time(mut self, time_ms: u64) -> Self {
        self.metadata.total_time_ms = time_ms;
        self
    }

    /// Set handoff count
    pub fn with_handoffs(mut self, count: usize) -> Self {
        self.metadata.handoff_count = count;
        self
    }

    /// Add extra metadata
    pub fn with_extra(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.extra.insert(key.into(), value);
        self
    }
}

/// Trait for orchestrator patterns
#[async_trait]
pub trait OrchestratorPattern: Send + Sync {
    /// Execute the pattern with given input
    async fn execute(&self, input: &str) -> Result<OrchestratorResult>;

    /// Get the pattern type name
    fn pattern_type(&self) -> &str;

    /// Get the number of agents in this pattern
    fn agent_count(&self) -> usize;
}

/// Builder for orchestrator patterns
pub struct OrchestratorBuilder {
    agents: Vec<Agent>,
}

impl OrchestratorBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self { agents: Vec::new() }
    }

    /// Add an agent
    pub fn agent(mut self, agent: Agent) -> Self {
        self.agents.push(agent);
        self
    }

    /// Add multiple agents
    pub fn agents(mut self, agents: Vec<Agent>) -> Self {
        self.agents.extend(agents);
        self
    }

    /// Get agents
    pub fn into_agents(self) -> Vec<Agent> {
        self.agents
    }
}

impl Default for OrchestratorBuilder {
    fn default() -> Self {
        Self::new()
    }
}
