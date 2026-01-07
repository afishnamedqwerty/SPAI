//! Consensus orchestrator pattern
//!
//! Multiple agents vote/respond independently, and a majority
//! voting mechanism determines the final consensus.

use crate::error::Result;
use crate::Agent;
use crate::orchestrator::pattern::{OrchestratorPattern, OrchestratorResult, AgentOutput};
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;
use futures::future::join_all;

/// Consensus orchestrator - majority voting
pub struct ConsensusOrchestrator {
    agents: Vec<Agent>,
    threshold: f64,
}

impl ConsensusOrchestrator {
    /// Create a new consensus orchestrator
    pub fn new(agents: Vec<Agent>) -> Self {
        Self {
            agents,
            threshold: 0.66, // 2/3 majority by default
        }
    }

    /// Set consensus threshold (0.0 to 1.0)
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Majority vote handoff function
    fn majority_vote(&self, responses: &[String]) -> (String, f64) {
        if responses.is_empty() {
            return (String::new(), 0.0);
        }

        // Simple heuristic: Extract key decisions/answers
        // Look for patterns like "Yes", "No", "Approve", "Reject", etc.
        let mut vote_counts: HashMap<String, usize> = HashMap::new();
        
        let decision_keywords = [
            ("yes", "yes"), ("approve", "yes"), ("agree", "yes"), ("support", "yes"),
            ("no", "no"), ("reject", "no"), ("disagree", "no"), ("oppose", "no"),
            ("uncertain", "uncertain"), ("maybe", "uncertain"),
        ];

        for response in responses {
            let lower = response.to_lowercase();
            let mut voted = false;
            
            for (keyword, vote) in &decision_keywords {
                if lower.contains(keyword) {
                    *vote_counts.entry(vote.to_string()).or_insert(0) += 1;
                    voted = true;
                    break;
                }
            }
            
            if !voted {
                // Use first sentence or summary as the "vote"
                let summary = response.lines().next().unwrap_or(response).to_string();
                *vote_counts.entry(summary).or_insert(0) += 1;
            }
        }

        // Find majority vote
        let total = responses.len() as f64;
        let (consensus, count) = vote_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .unwrap_or((String::new(), 0));

        let percentage = count as f64 / total;
        (consensus, percentage)
    }

    /// Determine if consensus was reached
    fn consensus_reached(&self, percentage: f64) -> bool {
        percentage >= self.threshold
    }
}

#[async_trait]
impl OrchestratorPattern for ConsensusOrchestrator {
    async fn execute(&self, input: &str) -> Result<OrchestratorResult> {
        let start = Instant::now();
        
        // All agents respond independently in parallel
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

        let results = join_all(futures).await;

        let mut result = OrchestratorResult::new("", "consensus");
        let mut responses = Vec::new();

        for (name, output_result, time_ms) in results {
            match output_result {
                Ok(output) => {
                    responses.push(output.content.clone());
                    result = result.with_agent_output(AgentOutput {
                        agent_name: name,
                        content: output.content,
                        loops_executed: output.trace.iteration_count(),
                        execution_time_ms: time_ms,
                    });
                }
                Err(e) => {
                    tracing::warn!("Agent {} failed: {}", name, e);
                }
            }
        }

        // Perform majority vote
        let (consensus, percentage) = self.majority_vote(&responses);
        let reached = self.consensus_reached(percentage);

        // Format final output
        result.content = if reached {
            format!(
                "# Consensus Reached ({:.0}% agreement)\n\n\
                 **Decision:** {}\n\n\
                 ## Individual Responses\n\n{}",
                percentage * 100.0,
                consensus,
                responses.iter().enumerate()
                    .map(|(i, r)| format!("### Agent {}\n{}", i + 1, r))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            )
        } else {
            format!(
                "# No Consensus ({:.0}% < {:.0}% threshold)\n\n\
                 **Majority position:** {}\n\n\
                 ## Individual Responses\n\n{}",
                percentage * 100.0,
                self.threshold * 100.0,
                consensus,
                responses.iter().enumerate()
                    .map(|(i, r)| format!("### Agent {}\n{}", i + 1, r))
                    .collect::<Vec<_>>()
                    .join("\n\n")
            )
        };

        result = result
            .with_time(start.elapsed().as_millis() as u64)
            .with_handoffs(0) // No handoffs in consensus pattern
            .with_extra("consensus_reached", serde_json::json!(reached))
            .with_extra("agreement_percentage", serde_json::json!(percentage))
            .with_extra("threshold", serde_json::json!(self.threshold));

        Ok(result)
    }

    fn pattern_type(&self) -> &str {
        "consensus"
    }

    fn agent_count(&self) -> usize {
        self.agents.len()
    }
}
