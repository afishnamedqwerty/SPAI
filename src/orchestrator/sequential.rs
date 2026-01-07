//! Sequential orchestrator pattern
//!
//! Agents execute in order, with the output of each agent becoming
//! the input for the next agent in the sequence.

use crate::error::Result;
use crate::Agent;
use crate::orchestrator::pattern::{OrchestratorPattern, OrchestratorResult, AgentOutput};
use async_trait::async_trait;
use std::time::Instant;

/// Sequential orchestrator - agents execute in order
pub struct SequentialOrchestrator {
    agents: Vec<Agent>,
}

impl SequentialOrchestrator {
    /// Create a new sequential orchestrator with given agents
    pub fn new(agents: Vec<Agent>) -> Self {
        Self { agents }
    }

    /// Create from a single agent (for simple chains)
    pub fn single(agent: Agent) -> Self {
        Self { agents: vec![agent] }
    }
}

#[async_trait]
impl OrchestratorPattern for SequentialOrchestrator {
    async fn execute(&self, input: &str) -> Result<OrchestratorResult> {
        let start = Instant::now();
        let mut result = OrchestratorResult::new("", "sequential");
        let mut current_input = input.to_string();

        for agent in &self.agents {
            let agent_start = Instant::now();
            
            let output = agent.react_loop(&current_input).await?;
            
            let agent_output = AgentOutput {
                agent_name: agent.name.clone(),
                content: output.content.clone(),
                loops_executed: output.trace.iteration_count(),
                execution_time_ms: agent_start.elapsed().as_millis() as u64,
            };
            
            result = result.with_agent_output(agent_output);
            current_input = output.content;
        }

        result.content = current_input;
        result = result.with_time(start.elapsed().as_millis() as u64);
        
        Ok(result)
    }

    fn pattern_type(&self) -> &str {
        "sequential"
    }

    fn agent_count(&self) -> usize {
        self.agents.len()
    }
}
