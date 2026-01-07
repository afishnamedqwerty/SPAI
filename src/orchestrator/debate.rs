//! Debate orchestrator pattern
//!
//! Pro and con agents argue positions for multiple rounds,
//! with a synthesizer agent producing the final balanced conclusion.

use crate::error::Result;
use crate::Agent;
use crate::orchestrator::pattern::{OrchestratorPattern, OrchestratorResult, AgentOutput};
use async_trait::async_trait;
use std::time::Instant;

/// Debate orchestrator - pro/con with synthesis
pub struct DebateOrchestrator {
    pro_agent: Agent,
    con_agent: Agent,
    synthesizer: Agent,
    rounds: usize,
}

impl DebateOrchestrator {
    /// Create a new debate orchestrator
    pub fn new(pro_agent: Agent, con_agent: Agent, synthesizer: Agent) -> Self {
        Self {
            pro_agent,
            con_agent,
            synthesizer,
            rounds: 2,
        }
    }

    /// Set number of debate rounds
    pub fn with_rounds(mut self, rounds: usize) -> Self {
        self.rounds = rounds;
        self
    }

    /// Debate synthesis handoff function
    fn debate_synthesis(&self, pro_args: &[String], con_args: &[String]) -> String {
        let mut synthesis = String::new();
        synthesis.push_str("# Debate Summary\n\n");
        
        synthesis.push_str("## Arguments For (Pro)\n");
        for (i, arg) in pro_args.iter().enumerate() {
            synthesis.push_str(&format!("\n### Round {}\n{}\n", i + 1, arg));
        }
        
        synthesis.push_str("\n## Arguments Against (Con)\n");
        for (i, arg) in con_args.iter().enumerate() {
            synthesis.push_str(&format!("\n### Round {}\n{}\n", i + 1, arg));
        }
        
        synthesis
    }
}

#[async_trait]
impl OrchestratorPattern for DebateOrchestrator {
    async fn execute(&self, input: &str) -> Result<OrchestratorResult> {
        let start = Instant::now();
        let mut result = OrchestratorResult::new("", "debate");
        
        let mut pro_arguments = Vec::new();
        let mut con_arguments = Vec::new();
        let mut all_outputs = Vec::new();

        // Opening statements
        let pro_opening = format!(
            "You are arguing IN FAVOR of the following position. Present your strongest arguments:\n\n{}",
            input
        );
        let con_opening = format!(
            "You are arguing AGAINST the following position. Present your strongest arguments:\n\n{}",
            input
        );

        // Debate rounds
        for round in 0..self.rounds {
            // Pro agent's turn
            let pro_prompt = if round == 0 {
                pro_opening.clone()
            } else {
                format!(
                    "Previous con argument:\n{}\n\nRespond to their points and strengthen your position IN FAVOR of:\n{}",
                    con_arguments.last().unwrap_or(&String::new()),
                    input
                )
            };

            let pro_start = Instant::now();
            let pro_output = self.pro_agent.react_loop(&pro_prompt).await?;
            pro_arguments.push(pro_output.content.clone());
            
            all_outputs.push(AgentOutput {
                agent_name: format!("{} (Round {})", self.pro_agent.name, round + 1),
                content: pro_output.content.clone(),
                loops_executed: pro_output.trace.iteration_count(),
                execution_time_ms: pro_start.elapsed().as_millis() as u64,
            });

            // Con agent's turn
            let con_prompt = if round == 0 {
                format!(
                    "Pro has argued:\n{}\n\nPresent your counter-arguments AGAINST:\n{}",
                    pro_output.content,
                    input
                )
            } else {
                format!(
                    "Pro has responded:\n{}\n\nCounter their points and strengthen your position AGAINST:\n{}",
                    pro_output.content,
                    input
                )
            };

            let con_start = Instant::now();
            let con_output = self.con_agent.react_loop(&con_prompt).await?;
            con_arguments.push(con_output.content.clone());
            
            all_outputs.push(AgentOutput {
                agent_name: format!("{} (Round {})", self.con_agent.name, round + 1),
                content: con_output.content.clone(),
                loops_executed: con_output.trace.iteration_count(),
                execution_time_ms: con_start.elapsed().as_millis() as u64,
            });
        }

        // Store all outputs
        for output in all_outputs {
            result = result.with_agent_output(output);
        }

        // Synthesizer produces final balanced conclusion
        let debate_summary = self.debate_synthesis(&pro_arguments, &con_arguments);
        let synthesis_prompt = format!(
            "{}\n\nProvide a balanced, nuanced synthesis of this debate. Consider:\n\
             1. The strongest points from each side\n\
             2. Where the positions might agree\n\
             3. A reasoned conclusion\n\
             Original question: {}",
            debate_summary,
            input
        );

        let synth_start = Instant::now();
        let synth_output = self.synthesizer.react_loop(&synthesis_prompt).await?;
        
        result = result.with_agent_output(AgentOutput {
            agent_name: format!("{} (Synthesis)", self.synthesizer.name),
            content: synth_output.content.clone(),
            loops_executed: synth_output.trace.iteration_count(),
            execution_time_ms: synth_start.elapsed().as_millis() as u64,
        });

        result.content = synth_output.content;
        result = result
            .with_time(start.elapsed().as_millis() as u64)
            .with_handoffs(self.rounds * 2) // Each round has pro->con handoff
            .with_extra("rounds", serde_json::json!(self.rounds));

        Ok(result)
    }

    fn pattern_type(&self) -> &str {
        "debate"
    }

    fn agent_count(&self) -> usize {
        3 // pro, con, synthesizer
    }
}
