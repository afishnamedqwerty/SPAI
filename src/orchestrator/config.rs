//! Orchestrator configuration parsing from YAML templates
//!
//! This module provides configuration structures for defining workflow patterns
//! via YAML templates with dynamic agent instantiation.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Top-level orchestrator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Pattern type (sequential, concurrent, hierarchical, debate, router, consensus)
    pub pattern: PatternType,
    /// Pattern-specific configuration
    #[serde(flatten)]
    pub pattern_config: PatternSpecificConfig,
    /// Optional tool tags to load
    #[serde(default)]
    pub tool_tags: Vec<String>,
}

/// Supported pattern types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    Sequential,
    Concurrent,
    Hierarchical,
    Debate,
    Router,
    Consensus,
}

/// Pattern-specific configuration variants
/// NOTE: Order matters for serde untagged! More specific variants must come first.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PatternSpecificConfig {
    /// Hierarchical pattern with lead and subagents
    Hierarchical {
        lead_agent: AgentConfig,
        subagents: SubagentConfig,
    },
    /// Debate pattern with pro/con and synthesizer
    Debate {
        pro_agent: AgentConfig,
        con_agent: AgentConfig,
        synthesizer: AgentConfig,
        #[serde(default = "default_debate_rounds")]
        rounds: usize,
    },
    /// Router pattern with router and specialists
    Router {
        router_agent: AgentConfig,
        specialists: HashMap<String, AgentConfig>,
    },
    /// Consensus pattern with agents and threshold (must come before AgentList!)
    Consensus {
        agents: Vec<AgentConfig>,
        threshold: f64,  // Required field to differentiate from AgentList
    },
    /// Sequential or concurrent patterns with agent list (last - catch-all for agents array)
    AgentList {
        agents: Vec<AgentConfig>,
        #[serde(default)]
        aggregation: Option<AggregationStrategy>,
    },
}

fn default_debate_rounds() -> usize { 2 }
fn default_consensus_threshold() -> f64 { 0.66 }

/// Agent instantiation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent name
    pub name: String,
    /// LLM model identifier (e.g., "anthropic/claude-sonnet-4")
    pub model: String,
    /// System prompt for the agent
    pub system_prompt: String,
    /// Maximum ReAct loops
    #[serde(default = "default_max_loops")]
    pub max_loops: usize,
    /// Temperature for generation
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Optional tool tags to load for this agent
    #[serde(default)]
    pub tool_tags: Vec<String>,
}

impl AgentConfig {
    /// Build an Agent from this configuration
    pub fn build(&self, client: std::sync::Arc<dyn crate::llm_client::LlmClient>) -> crate::error::Result<crate::Agent> {
        crate::Agent::builder()
            .name(&self.name)
            .model(&self.model)
            .system_prompt(&self.system_prompt)
            .max_loops(self.max_loops as u32)
            .temperature(self.temperature)
            .client(client)
            .build()
    }
}

fn default_max_loops() -> usize { 5 }
fn default_temperature() -> f32 { 0.7 }

/// Subagent configuration for dynamic instantiation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentConfig {
    /// Number of subagents to create
    pub count: usize,
    /// Model for all subagents
    pub model: String,
    /// System prompt template with {index} placeholder
    pub system_prompt_template: String,
    /// Maximum loops per subagent
    #[serde(default = "default_max_loops")]
    pub max_loops: usize,
    /// Temperature for subagents
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Optional tool tags
    #[serde(default)]
    pub tool_tags: Vec<String>,
}

impl SubagentConfig {
    /// Generate agent configs from template
    pub fn generate_agents(&self) -> Vec<AgentConfig> {
        (0..self.count)
            .map(|i| AgentConfig {
                name: format!("Subagent_{}", i + 1),
                model: self.model.clone(),
                system_prompt: self.system_prompt_template.replace("{index}", &(i + 1).to_string()),
                max_loops: self.max_loops,
                temperature: self.temperature,
                tool_tags: self.tool_tags.clone(),
            })
            .collect()
    }
}

/// Aggregation strategy for concurrent patterns
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AggregationStrategy {
    /// Concatenate all outputs
    #[default]
    Concatenate,
    /// Merge outputs (deduplicate similar content)
    Merge,
    /// Take only the first response
    First,
    /// Take the longest response
    Longest,
    /// Custom aggregation via synthesizer agent
    Synthesize,
}

impl OrchestratorConfig {
    /// Load configuration from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml)
            .map_err(|e| Error::Config(format!("Failed to parse YAML: {}", e)))
    }

    /// Load configuration from YAML file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| Error::Config(format!("Failed to read file: {}", e)))?;
        Self::from_yaml(&content)
    }

    /// Get the pattern type
    pub fn pattern_type(&self) -> &PatternType {
        &self.pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sequential_config() {
        let yaml = r#"
pattern: sequential
agents:
  - name: "Researcher"
    model: "anthropic/claude-sonnet-4"
    system_prompt: "Research the topic."
    max_loops: 3
  - name: "Writer"
    model: "anthropic/claude-sonnet-4"
    system_prompt: "Write based on research."
"#;
        let config = OrchestratorConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.pattern, PatternType::Sequential);
    }

    #[test]
    fn test_parse_hierarchical_config() {
        let yaml = r#"
pattern: hierarchical
lead_agent:
  name: "Lead"
  model: "anthropic/claude-sonnet-4"
  system_prompt: "Coordinate subagents."
  max_loops: 5
subagents:
  count: 3
  model: "anthropic/claude-haiku"
  system_prompt_template: "You are Analyst {index}."
"#;
        let config = OrchestratorConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.pattern, PatternType::Hierarchical);
    }

    #[test]
    fn test_subagent_generation() {
        let subconfig = SubagentConfig {
            count: 3,
            model: "test-model".to_string(),
            system_prompt_template: "Agent {index} ready.".to_string(),
            max_loops: 2,
            temperature: 0.5,
            tool_tags: vec![],
        };
        let agents = subconfig.generate_agents();
        assert_eq!(agents.len(), 3);
        assert_eq!(agents[0].system_prompt, "Agent 1 ready.");
        assert_eq!(agents[1].system_prompt, "Agent 2 ready.");
        assert_eq!(agents[2].system_prompt, "Agent 3 ready.");
    }
}
