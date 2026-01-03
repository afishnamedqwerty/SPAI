//! # ATHPTTGH Agent Harness
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
pub mod config;
pub mod error;
pub mod guardrails;
pub mod handoffs;
pub mod hitl;
pub mod openrouter;
pub mod patterns;
pub mod react;
pub mod tools;
pub mod tracing_ext;
pub mod turns;
pub mod types;

// Re-exports for convenience
pub use agent::{Agent, AgentBuilder, AgentHooks, AgentOutput};
pub use config::{ModelConfig, OpenRouterConfig};
pub use error::{Error, Result};
pub use guardrails::{GuardrailContext, GuardrailResult, InputGuardrail, OutputGuardrail};
pub use handoffs::{Handoff, HandoffContext, HandoffStrategy};
pub use hitl::{ApprovalDecision, ApprovalHandler, ApprovalRequest};
pub use openrouter::{OpenRouterClient, CompletionRequest, StreamChunk};
pub use patterns::{PatternConfig, WorkflowPattern};
pub use react::{ReActConfig, ReActTrace, ReasoningFormat};
pub use tools::{Tool, ToolContext, ToolOutput};
pub use turns::{Session, Turn, TurnManager};
pub use types::{AgentId, SessionId, SpanId, TraceId, TurnId};

/// Prelude module for common imports
pub mod prelude {
    pub use crate::agent::{Agent, AgentBuilder, AgentOutput};
    pub use crate::error::{Error, Result};
    pub use crate::openrouter::OpenRouterClient;
    pub use crate::react::{ReActConfig, ReasoningFormat};
    pub use crate::tools::Tool;
    pub use crate::types::*;
}
