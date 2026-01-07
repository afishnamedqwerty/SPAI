//! # SPAI Agent Harness
//!
//! A production-grade multi-agent orchestration framework built with Rust.
//!
//! **ATHPTTGH** stands for: **A**gents, **T**ools, **H**andoffs, **P**atterns, **T**urns,
//! **T**racing, **G**uardrails, and **H**uman-in-the-Loop.
//!
//! ## Features
//!
//! - **ReAct-Native**: Every agent implements the Thought→Action→Observation loop
//! - **OpenRouter Integration**: Access to 200+ LLM providers through a single API
//! - **Comprehensive Observability**: Full tracing of agent decisions and actions
//! - **Safety-First**: Input/output guardrails and human approval workflows
//! - **Flexible Orchestration**: Multiple workflow patterns for diverse use cases
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use athpttgh::{Agent, OpenRouterClient, ReActConfig, ReasoningFormat};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize OpenRouter client
//!     let client = OpenRouterClient::from_env()?;
//!
//!     // Create an agent with ReAct enabled
//!     let agent = Agent::builder()
//!         .name("Research Agent")
//!         .model("anthropic/claude-sonnet-4")
//!         .system_prompt("You are a helpful research assistant.")
//!         .react_config(ReActConfig {
//!             enable_reasoning_traces: true,
//!             reasoning_format: ReasoningFormat::ThoughtAction,
//!             max_reasoning_tokens: 1000,
//!             expose_reasoning: true,
//!         })
//!         .build()?;
//!
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod agent;
pub mod agent_file;
pub mod background;
pub mod config;
pub mod error;
pub mod filesystem;
pub mod guardrails;
pub mod handoffs;
pub mod hitl;
pub mod llm_client;
pub mod memory;
pub mod memory_tools;
pub mod openrouter;
pub mod patterns;
pub mod orchestrator;
pub mod react;
pub mod sleeptime;
#[cfg(feature = "storage")]
pub mod storage;
pub mod tools;
pub mod security_tools;
pub mod tracing_ext;
pub mod turns;
pub mod types;
pub mod vllm;

// Solid Pod integration (optional feature)
#[cfg(feature = "solid-integration")]
pub mod solid;

// Re-exports for convenience
pub use agent::{Agent, AgentBuilder, AgentHooks, AgentOutput};
pub use agent_file::{AgentFile, CheckpointManager};
pub use background::{BackgroundExecutor, RunId, SeqId, RunStatus, RunEvent, RunEventType, PaginatedEvents};
pub use config::{ModelConfig, OpenRouterConfig};
pub use error::{Error, Result};
pub use filesystem::{FilesystemManager, AttachedFolder};
pub use guardrails::{GuardrailContext, GuardrailResult, InputGuardrail, OutputGuardrail};
pub use handoffs::{Handoff, HandoffContext, HandoffStrategy};
pub use hitl::{ApprovalDecision, ApprovalHandler, ApprovalRequest};
pub use llm_client::LlmClient;
pub use memory::{AgentMemory, MemoryBlock, MemoryConfig, SharedMemoryManager};
pub use openrouter::{OpenRouterClient, CompletionRequest, StreamChunk};
pub use sleeptime::{SleepTimeAgent, SleepTimeConfig};
#[cfg(feature = "storage")]
pub use storage::{MemoryStorage, PostgresStorage, SqliteStorage};
pub use patterns::{PatternConfig, WorkflowPattern};
pub use orchestrator::{
    OrchestratorConfig, OrchestratorPattern, OrchestratorResult,
    PatternType, AgentConfig, SubagentConfig,
    SequentialOrchestrator, ConcurrentOrchestrator, HierarchicalOrchestrator,
    DebateOrchestrator, RouterOrchestrator, ConsensusOrchestrator,
};
pub use react::{ReActConfig, ReActTrace, ReasoningFormat};
pub use tools::{Tool, ToolContext, ToolOutput};
#[cfg(feature = "mcp-tools")]
pub use tools::McpSubprocessTool;
pub use security_tools::{SecurityToolRegistry, SecurityTool, SecurityCategory, ListSecurityTools, RunSecurityTool, TaggedSecurityTools};
pub use turns::{Session, Turn, TurnManager};
pub use types::{AgentId, SessionId, SpanId, TraceId, TurnId};
pub use vllm::{VllmClient, VllmConfig};

/// Prelude module for common imports
pub mod prelude {
    pub use crate::agent::{Agent, AgentBuilder, AgentOutput};
    pub use crate::error::{Error, Result};
    pub use crate::llm_client::LlmClient;
    pub use crate::openrouter::OpenRouterClient;
    pub use crate::react::{ReActConfig, ReasoningFormat};
    pub use crate::tools::Tool;
    #[cfg(feature = "mcp-tools")]
    pub use crate::tools::McpSubprocessTool;
    pub use crate::types::*;
    pub use crate::vllm::{VllmClient, VllmConfig};
}
