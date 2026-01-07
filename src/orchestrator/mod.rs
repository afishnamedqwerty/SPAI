//! Orchestrator module - Workflow patterns for multi-agent coordination
//!
//! This module provides YAML-configurable workflow patterns for orchestrating
//! multiple agents with different coordination strategies.
//!
//! # Patterns
//!
//! - **Sequential**: Agents execute in order, output chains to next
//! - **Concurrent**: Parallel execution with aggregation
//! - **Hierarchical**: Lead agent with subagent delegation
//! - **Debate**: Pro/con with synthesis
//! - **Router**: Triage to specialized agents
//! - **Consensus**: Majority voting
//!
//! # Example
//!
//! ```rust,ignore
//! use spai::orchestrator::{OrchestratorConfig, SequentialOrchestrator};
//!
//! let config = OrchestratorConfig::from_file("templates/sequential.yaml")?;
//! let orchestrator = SequentialOrchestrator::from_config(&config, client)?;
//! let result = orchestrator.execute("Analyze this problem").await?;
//! ```

pub mod config;
pub mod pattern;
pub mod sequential;
pub mod concurrent;
pub mod hierarchical;
pub mod debate;
pub mod router;
pub mod consensus;

// Re-exports
pub use config::{
    OrchestratorConfig, 
    PatternType, 
    PatternSpecificConfig,
    AgentConfig, 
    SubagentConfig,
    AggregationStrategy,
};
pub use pattern::{
    OrchestratorPattern, 
    OrchestratorResult, 
    AgentOutput,
    OrchestratorMetadata,
    OrchestratorBuilder,
};
pub use sequential::SequentialOrchestrator;
pub use concurrent::ConcurrentOrchestrator;
pub use hierarchical::HierarchicalOrchestrator;
pub use debate::DebateOrchestrator;
pub use router::RouterOrchestrator;
pub use consensus::ConsensusOrchestrator;
