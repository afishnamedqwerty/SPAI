//! Concurrent orchestrator pattern
//!
//! All agents execute in parallel, with results aggregated according
//! to the specified strategy.

use crate::error::Result;
use crate::Agent;
use crate::orchestrator::config::AggregationStrategy;
use crate::orchestrator::pattern::{OrchestratorPattern, OrchestratorResult, AgentOutput};
use async_trait::async_trait;
use std::time::Instant;
use futures::future::join_all;

/// Concurrent orchestrator - parallel execution with aggregation
pub struct ConcurrentOrchestrator {
    agents: Vec<Agent>,
    aggregation: AggregationStrategy,
}

impl ConcurrentOrchestrator {
    /// Create a new concurrent orchestrator
    pub fn new(agents: Vec<Agent>) -> Self {
        Self {
            agents,
            aggregation: AggregationStrategy::Concatenate,
        }
    }

    /// Set aggregation strategy
    pub fn with_aggregation(mut self, strategy: AggregationStrategy) -> Self {
        self.aggregation = strategy;
        self
    }

    /// Aggregate outputs based on strategy
    fn aggregate(&self, outputs: &[AgentOutput]) -> String {
        match &self.aggregation {
            AggregationStrategy::Concatenate => {
                outputs.iter()
                    .map(|o| format!("## {}\n\n{}", o.agent_name, o.content))
                    .collect::<Vec<_>>()
                    .join("\n\n---\n\n")
            }
            AggregationStrategy::First => {
                outputs.first().map(|o| o.content.clone()).unwrap_or_default()
            }
            AggregationStrategy::Longest => {
                outputs.iter()
                    .max_by_key(|o| o.content.len())
                    .map(|o| o.content.clone())
                    .unwrap_or_default()
            }
            AggregationStrategy::Merge => {
                // Simple merge - deduplicate lines
                let mut seen = std::collections::HashSet::new();
                outputs.iter()
                    .flat_map(|o| o.content.lines())
                    .filter(|line| seen.insert(line.to_string()))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            AggregationStrategy::Synthesize => {
                // For synthesis, we just concatenate - actual synthesis
                // would require another agent call
                outputs.iter()
                    .map(|o| o.content.clone())
                    .collect::<Vec<_>>()
                    .join("\n\n")
            }
        }
    }
}

#[async_trait]
impl OrchestratorPattern for ConcurrentOrchestrator {
    async fn execute(&self, input: &str) -> Result<OrchestratorResult> {
        let start = Instant::now();
        
        // Create futures for all agents
        let futures: Vec<_> = self.agents.iter()
            .map(|agent| {
                let input = input.to_string();
                async move {
                    let agent_start = Instant::now();
                    let result = agent.react_loop(&input).await;
                    (agent.name.clone(), result, agent_start.elapsed().as_millis() as u64)
                }
            })
            .collect();

        // Execute all in parallel
        let results = join_all(futures).await;

        // Collect outputs
        let mut agent_outputs = Vec::new();
        let mut result = OrchestratorResult::new("", "concurrent");

        for (name, output_result, time_ms) in results {
            match output_result {
                Ok(output) => {
                    let agent_output = AgentOutput {
                        agent_name: name,
                        content: output.content,
                        loops_executed: output.trace.iteration_count(),
                        execution_time_ms: time_ms,
                    };
                    agent_outputs.push(agent_output.clone());
                    result = result.with_agent_output(agent_output);
                }
                Err(e) => {
                    tracing::warn!("Agent {} failed: {}", name, e);
                }
            }
        }

        // Aggregate results
        result.content = self.aggregate(&agent_outputs);
        result = result
            .with_time(start.elapsed().as_millis() as u64)
            .with_extra("aggregation", serde_json::json!(format!("{:?}", self.aggregation)));

        Ok(result)
    }

    fn pattern_type(&self) -> &str {
        "concurrent"
    }

    fn agent_count(&self) -> usize {
        self.agents.len()
    }
}
