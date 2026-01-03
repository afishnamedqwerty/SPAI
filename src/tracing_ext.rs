//! Tracing and observability infrastructure

use crate::types::{SpanId, TokenUsage, TraceId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Trace of agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    /// Unique trace identifier
    pub trace_id: TraceId,
    /// Human-readable workflow name
    pub name: String,
    /// Root span containing all child spans
    pub root_span: Span,
    /// Trace-level metadata
    pub metadata: TraceMetadata,
    /// Total duration of the trace
    pub duration: Duration,
    /// Aggregate token usage
    pub total_tokens: TokenUsage,
}

/// Trace metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceMetadata {
    /// Custom metadata
    pub custom: HashMap<String, serde_json::Value>,
}

/// Span representing a single operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Unique span identifier
    pub span_id: SpanId,
    /// Parent span (None for root)
    pub parent_id: Option<SpanId>,
    /// Span type
    pub span_type: SpanType,
    /// Span name
    pub name: String,
    /// Start timestamp
    pub started_at: DateTime<Utc>,
    /// End timestamp
    pub ended_at: Option<DateTime<Utc>>,
    /// Span-specific data
    pub data: SpanData,
    /// Child spans
    pub children: Vec<Span>,
}

/// Type of span
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanType {
    /// Agent run
    AgentRun,
    /// LLM generation
    LlmGeneration,
    /// Tool call
    ToolCall,
    /// Handoff
    Handoff,
    /// Guardrail check
    GuardrailCheck,
    /// ReAct thought
    ReActThought,
    /// ReAct action
    ReActAction,
    /// ReAct observation
    ReActObservation,
    /// Custom span type
    Custom(String),
}

/// Span data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanData {
    /// Custom data
    pub data: HashMap<String, serde_json::Value>,
}

/// Patterns workflow configuration
#[derive(Debug, Clone)]
pub struct PatternConfig {
    /// Maximum total execution time
    pub timeout: Duration,
    /// How to handle partial failures
    pub failure_mode: FailureMode,
    /// Token budget across all agents
    pub token_budget: Option<u64>,
    /// Custom pattern-specific parameters
    pub params: HashMap<String, serde_json::Value>,
}

/// Failure handling mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureMode {
    /// Fail entire pattern on any agent failure
    FailFast,
    /// Continue with remaining agents
    FailSafe,
    /// Retry failed agents with backoff
    Retry {
        /// Maximum retry attempts
        max_attempts: u32,
        /// Backoff duration
        backoff: Duration,
    },
}
