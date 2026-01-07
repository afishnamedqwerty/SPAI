//! Hierarchical orchestrator pattern
//!
//! A lead agent decomposes tasks and delegates to subagents,
//! then synthesizes their outputs into a final result.

use crate::error::Result;
use crate::Agent;
use crate::handoffs::{Handoff, HandoffContext};
use crate::orchestrator::pattern::{OrchestratorPattern, OrchestratorResult, AgentOutput};
use crate::types::AgentId;
use async_trait::async_trait;
use std::time::Instant;
use futures::future::join_all;

/// Hierarchical orchestrator - lead agent with subagent delegation
pub struct HierarchicalOrchestrator {
    lead_agent: Agent,
    subagents: Vec<Agent>,
}

impl HierarchicalOrchestrator {
    /// Create a new hierarchical orchestrator
    pub fn new(lead_agent: Agent, subagents: Vec<Agent>) -> Self {
        Self { lead_agent, subagents }
    }

    /// Create handoff to a subagent
    fn create_handoff(&self, subagent: &Agent, subtask: &str, context: &str) -> Handoff {
        Handoff::new(
            AgentId::new(),
            AgentId::new(),
            format!("Delegating subtask: {}", subtask),
            HandoffContext::new(context),
        )
    }

    /// Parse subtasks from lead agent's decomposition
    fn parse_subtasks(&self, decomposition: &str) -> Vec<String> {
        // Look for numbered lists or bullet points
        decomposition
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("- ") ||
                trimmed.starts_with("* ") ||
                trimmed.starts_with("1.") ||
                trimmed.starts_with("2.") ||
                trimmed.starts_with("3.") ||
                trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
            })
            .map(|line| {
                line.trim()
                    .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == '-' || c == '*' || c == ' ')
                    .trim()
                    .to_string()
            })
            .filter(|s| !s.is_empty())
            .collect()
    }
}

#[async_trait]
impl OrchestratorPattern for HierarchicalOrchestrator {
    async fn execute(&self, input: &str) -> Result<OrchestratorResult> {
        let start = Instant::now();
        let mut result = OrchestratorResult::new("", "hierarchical");
        let mut handoff_count = 0;

        // Phase 1: Lead agent decomposes the task
        let decomposition_prompt = format!(
            "You are the lead coordinator. Decompose this task into {} subtasks for your subagents:\n\n{}",
            self.subagents.len(),
            input
        );

        let lead_start = Instant::now();
        let lead_output = self.lead_agent.react_loop(&decomposition_prompt).await?;
        
        result = result.with_agent_output(AgentOutput {
            agent_name: format!("{} (decomposition)", self.lead_agent.name),
            content: lead_output.content.clone(),
            loops_executed: lead_output.trace.iteration_count(),
            execution_time_ms: lead_start.elapsed().as_millis() as u64,
        });

        // Phase 2: Parse subtasks and delegate to subagents
        let subtasks = self.parse_subtasks(&lead_output.content);
        
        let futures: Vec<_> = self.subagents.iter()
            .zip(subtasks.iter().cycle()) // Cycle if fewer subtasks than subagents
            .map(|(subagent, subtask)| {
                let _handoff = self.create_handoff(subagent, subtask, input);
                handoff_count += 1;
                
                let subtask_prompt = format!(
                    "Original task: {}\n\nYour assigned subtask: {}\n\nProvide your analysis:",
                    input, subtask
                );
                
                async move {
                    let agent_start = Instant::now();
                    let result = subagent.react_loop(&subtask_prompt).await;
                    (subagent.name.clone(), result, agent_start.elapsed().as_millis() as u64)
                }
            })
            .collect();

        let subagent_results = join_all(futures).await;
        
        let mut subagent_outputs = Vec::new();
        for (name, output_result, time_ms) in subagent_results {
            if let Ok(output) = output_result {
                let agent_output = AgentOutput {
                    agent_name: name,
                    content: output.content.clone(),
                    loops_executed: output.trace.iteration_count(),
                    execution_time_ms: time_ms,
                };
                subagent_outputs.push(agent_output.clone());
                result = result.with_agent_output(agent_output);
            }
        }

        // Phase 3: Lead agent synthesizes results
        let synthesis_prompt = format!(
            "Original task: {}\n\nSubagent outputs:\n{}\n\nSynthesize these into a comprehensive final answer:",
            input,
            subagent_outputs.iter()
                .map(|o| format!("### {}\n{}", o.agent_name, o.content))
                .collect::<Vec<_>>()
                .join("\n\n")
        );

        let synthesis_start = Instant::now();
        let synthesis_output = self.lead_agent.react_loop(&synthesis_prompt).await?;
        
        result = result.with_agent_output(AgentOutput {
            agent_name: format!("{} (synthesis)", self.lead_agent.name),
            content: synthesis_output.content.clone(),
            loops_executed: synthesis_output.trace.iteration_count(),
            execution_time_ms: synthesis_start.elapsed().as_millis() as u64,
        });

        result.content = synthesis_output.content;
        result = result
            .with_time(start.elapsed().as_millis() as u64)
            .with_handoffs(handoff_count)
            .with_extra("subtasks", serde_json::json!(subtasks));

        Ok(result)
    }

    fn pattern_type(&self) -> &str {
        "hierarchical"
    }

    fn agent_count(&self) -> usize {
        1 + self.subagents.len()
    }
}
