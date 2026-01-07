//! Router orchestrator pattern
//!
//! A router agent triages requests and routes them
//! to specialized agents based on domain expertise.

use crate::error::Result;
use crate::Agent;
use crate::handoffs::{Handoff, HandoffContext};
use crate::orchestrator::pattern::{OrchestratorPattern, OrchestratorResult, AgentOutput};
use crate::types::AgentId;
use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;

/// Router orchestrator - triage and route to specialists
pub struct RouterOrchestrator {
    router_agent: Agent,
    specialists: HashMap<String, Agent>,
}

impl RouterOrchestrator {
    /// Create a new router orchestrator
    pub fn new(router_agent: Agent) -> Self {
        Self {
            router_agent,
            specialists: HashMap::new(),
        }
    }

    /// Add a specialist agent
    pub fn with_specialist(mut self, domain: impl Into<String>, agent: Agent) -> Self {
        self.specialists.insert(domain.into(), agent);
        self
    }

    /// Add multiple specialists
    pub fn with_specialists(mut self, specialists: HashMap<String, Agent>) -> Self {
        self.specialists.extend(specialists);
        self
    }

    /// Route to specialist handoff function
    fn route_to_specialist(&self, domain: &str, query: &str) -> Option<Handoff> {
        self.specialists.get(domain).map(|specialist| {
            Handoff::new(
                AgentId::new(),
                AgentId::new(),
                format!("Routing to {} specialist", domain),
                HandoffContext::new(query),
            )
        })
    }

    /// Parse routing decision from router output
    fn parse_routing(&self, router_output: &str) -> Option<String> {
        let output_lower = router_output.to_lowercase();
        
        // Look for explicit routing mentions
        for domain in self.specialists.keys() {
            let domain_lower = domain.to_lowercase();
            if output_lower.contains(&format!("route to {}", domain_lower)) ||
               output_lower.contains(&format!("routing to {}", domain_lower)) ||
               output_lower.contains(&format!("specialist: {}", domain_lower)) ||
               output_lower.contains(&format!("domain: {}", domain_lower)) {
                return Some(domain.clone());
            }
        }
        
        // Try to find any specialist domain mentioned
        for domain in self.specialists.keys() {
            if output_lower.contains(&domain.to_lowercase()) {
                return Some(domain.clone());
            }
        }
        
        None
    }
}

#[async_trait]
impl OrchestratorPattern for RouterOrchestrator {
    async fn execute(&self, input: &str) -> Result<OrchestratorResult> {
        let start = Instant::now();
        let mut result = OrchestratorResult::new("", "router");

        // Build routing prompt with available specialists
        let specialist_list: Vec<_> = self.specialists.keys().collect();
        let routing_prompt = format!(
            "You are a routing agent. Analyze this request and determine which specialist should handle it.\n\n\
             Available specialists: {:?}\n\n\
             Request: {}\n\n\
             Respond with the domain you're routing to in the format: 'Route to [domain]'",
            specialist_list,
            input
        );

        // Router agent makes decision
        let router_start = Instant::now();
        let router_output = self.router_agent.react_loop(&routing_prompt).await?;
        
        result = result.with_agent_output(AgentOutput {
            agent_name: format!("{} (Routing)", self.router_agent.name),
            content: router_output.content.clone(),
            loops_executed: router_output.trace.iteration_count(),
            execution_time_ms: router_start.elapsed().as_millis() as u64,
        });

        // Parse routing decision
        let routed_domain = self.parse_routing(&router_output.content);

        if let Some(domain) = &routed_domain {
            if let Some(specialist) = self.specialists.get(domain) {
                // Create handoff
                let _handoff = self.route_to_specialist(domain, input);

                let specialist_prompt = format!(
                    "You are a {} specialist. Handle this request:\n\n{}",
                    domain, input
                );

                let spec_start = Instant::now();
                let spec_output = specialist.react_loop(&specialist_prompt).await?;
                
                result = result.with_agent_output(AgentOutput {
                    agent_name: format!("{} ({})", specialist.name, domain),
                    content: spec_output.content.clone(),
                    loops_executed: spec_output.trace.iteration_count(),
                    execution_time_ms: spec_start.elapsed().as_millis() as u64,
                });

                result.content = spec_output.content;
                result = result.with_handoffs(1);
            }
        } else {
            // No specialist found, router handles directly
            result.content = format!(
                "No specialist matched. Router response:\n\n{}",
                router_output.content
            );
        }

        result = result
            .with_time(start.elapsed().as_millis() as u64)
            .with_extra("routed_to", serde_json::json!(routed_domain));

        Ok(result)
    }

    fn pattern_type(&self) -> &str {
        "router"
    }

    fn agent_count(&self) -> usize {
        1 + self.specialists.len()
    }
}
