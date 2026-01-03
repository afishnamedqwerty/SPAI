//! ReAct (Reasoning and Acting) paradigm implementation

use crate::types::{SpanId, TokenUsage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Configuration for ReAct agent behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReActConfig {
    /// Enable explicit thought traces before actions
    pub enable_reasoning_traces: bool,
    /// Format for reasoning output
    pub reasoning_format: ReasoningFormat,
    /// Maximum tokens for reasoning phase
    pub max_reasoning_tokens: u32,
    /// Whether to expose reasoning to external observers
    pub expose_reasoning: bool,
}

impl Default for ReActConfig {
    fn default() -> Self {
        Self {
            enable_reasoning_traces: true,
            reasoning_format: ReasoningFormat::ThoughtAction,
            max_reasoning_tokens: 1000,
            expose_reasoning: true,
        }
    }
}

/// Format for reasoning output
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningFormat {
    /// Thought: ... Action: ... format
    ThoughtAction,
    /// <thinking>...</thinking> XML format
    XmlThinking,
    /// JSON structured reasoning
    JsonStructured,
}

/// A trace of ReAct loop execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReActTrace {
    /// All thoughts generated during execution
    pub thoughts: Vec<Thought>,
    /// All actions taken during execution
    pub actions: Vec<Action>,
    /// All observations received during execution
    pub observations: Vec<Observation>,
    /// When the trace started
    pub started_at: DateTime<Utc>,
    /// When the trace completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Total token usage across all steps
    pub total_tokens: TokenUsage,
}

impl ReActTrace {
    /// Create a new empty trace
    pub fn new() -> Self {
        Self {
            thoughts: Vec::new(),
            actions: Vec::new(),
            observations: Vec::new(),
            started_at: Utc::now(),
            completed_at: None,
            total_tokens: TokenUsage::default(),
        }
    }

    /// Add a thought to the trace
    pub fn add_thought(&mut self, thought: Thought) {
        self.total_tokens.add(thought.tokens);
        self.thoughts.push(thought);
    }

    /// Add an action to the trace
    pub fn add_action(&mut self, action: Action) {
        self.actions.push(action);
    }

    /// Add an observation to the trace
    pub fn add_observation(&mut self, observation: Observation) {
        self.observations.push(observation);
    }

    /// Mark the trace as completed
    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
    }

    /// Get the iteration count (number of thought-action-observation cycles)
    pub fn iteration_count(&self) -> usize {
        self.thoughts.len()
    }

    /// Format the trace as a human-readable string
    pub fn format(&self) -> String {
        let mut output = String::new();

        for i in 0..self.iteration_count() {
            output.push_str(&format!("=== Iteration {} ===\n", i + 1));

            if let Some(thought) = self.thoughts.get(i) {
                output.push_str(&format!("Thought: {}\n", thought.content));
            }

            if let Some(action) = self.actions.get(i) {
                output.push_str(&format!("Action: {}\n", action.describe()));
            }

            if let Some(observation) = self.observations.get(i) {
                output.push_str(&format!("Observation: {}\n\n", observation.content));
            }
        }

        output
    }
}

impl Default for ReActTrace {
    fn default() -> Self {
        Self::new()
    }
}

/// A thought in the ReAct loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    /// The thought content
    pub content: String,
    /// When this thought occurred
    pub timestamp: DateTime<Utc>,
    /// Span ID for tracing
    pub span_id: Option<SpanId>,
    /// Token usage for generating this thought
    pub tokens: TokenUsage,
}

impl Thought {
    /// Create a new thought
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            timestamp: Utc::now(),
            span_id: None,
            tokens: TokenUsage::default(),
        }
    }

    /// Set the span ID
    pub fn with_span_id(mut self, span_id: SpanId) -> Self {
        self.span_id = Some(span_id);
        self
    }

    /// Set the token usage
    pub fn with_tokens(mut self, tokens: TokenUsage) -> Self {
        self.tokens = tokens;
        self
    }
}

/// An action in the ReAct loop
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Call a tool
    ToolCall {
        /// Tool identifier
        tool_id: String,
        /// Tool parameters
        params: serde_json::Value,
        /// When this action occurred
        timestamp: DateTime<Utc>,
    },
    /// Hand off to another agent
    Handoff {
        /// Target agent ID
        target_agent: String,
        /// Handoff reason
        reason: String,
        /// When this action occurred
        timestamp: DateTime<Utc>,
    },
    /// Provide final answer
    FinalAnswer {
        /// The answer content
        answer: String,
        /// When this action occurred
        timestamp: DateTime<Utc>,
    },
}

impl Action {
    /// Create a tool call action
    pub fn tool_call(tool_id: impl Into<String>, params: serde_json::Value) -> Self {
        Self::ToolCall {
            tool_id: tool_id.into(),
            params,
            timestamp: Utc::now(),
        }
    }

    /// Create a handoff action
    pub fn handoff(target_agent: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Handoff {
            target_agent: target_agent.into(),
            reason: reason.into(),
            timestamp: Utc::now(),
        }
    }

    /// Create a final answer action
    pub fn final_answer(answer: impl Into<String>) -> Self {
        Self::FinalAnswer {
            answer: answer.into(),
            timestamp: Utc::now(),
        }
    }

    /// Get a human-readable description of the action
    pub fn describe(&self) -> String {
        match self {
            Self::ToolCall { tool_id, params, .. } => {
                format!("Call tool '{}' with params: {}", tool_id, params)
            }
            Self::Handoff { target_agent, reason, .. } => {
                format!("Hand off to agent '{}': {}", target_agent, reason)
            }
            Self::FinalAnswer { answer, .. } => {
                format!("Final answer: {}", answer)
            }
        }
    }
}

/// An observation in the ReAct loop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// The observation content
    pub content: String,
    /// When this observation occurred
    pub timestamp: DateTime<Utc>,
    /// Whether this observation indicates an error
    pub is_error: bool,
    /// Span ID for tracing
    pub span_id: Option<SpanId>,
}

impl Observation {
    /// Create a new observation
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            timestamp: Utc::now(),
            is_error: false,
            span_id: None,
        }
    }

    /// Create an error observation
    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            timestamp: Utc::now(),
            is_error: true,
            span_id: None,
        }
    }

    /// Set the span ID
    pub fn with_span_id(mut self, span_id: SpanId) -> Self {
        self.span_id = Some(span_id);
        self
    }
}
