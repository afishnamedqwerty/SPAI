//! Turn and session management

use crate::agent::AgentOutput;
use crate::error::Result;
use crate::react::ReActTrace;
use crate::types::{AgentId, SessionId, TokenUsage, TurnId, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Turn in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    /// Unique turn identifier
    pub id: TurnId,
    /// Session this turn belongs to
    pub session_id: SessionId,
    /// Agent that processed this turn
    pub agent_id: AgentId,
    /// User input for this turn
    pub input: String,
    /// Agent output for this turn
    pub output: AgentOutput,
    /// Timestamp of turn completion
    pub timestamp: DateTime<Utc>,
    /// Token usage for this turn
    pub token_usage: TokenUsage,
    /// ReAct trace for this turn
    pub trace: ReActTrace,
}

/// Session grouping related turns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,
    /// User associated with this session
    pub user_id: Option<UserId>,
    /// Current active agent
    pub current_agent: AgentId,
    /// All turns in this session
    pub turns: Vec<Turn>,
    /// Session-level metadata
    pub metadata: SessionMetadata,
    /// Session state
    pub state: SessionState,
}

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Custom metadata
    pub custom: HashMap<String, serde_json::Value>,
}

/// Session state
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Session is active
    Active,
    /// Session is paused
    Paused,
    /// Session is completed
    Completed,
    /// Session is archived
    Archived,
}

/// Turn manager for conversation state
pub struct TurnManager {
    /// Maximum context window tokens
    max_context_tokens: u64,
    /// Strategy for context compaction
    compaction_strategy: CompactionStrategy,
    /// Persistent storage backend
    storage: Option<Arc<dyn TurnStorage>>,
}

impl TurnManager {
    /// Create a new turn manager
    pub fn new(max_context_tokens: u64) -> Self {
        Self {
            max_context_tokens,
            compaction_strategy: CompactionStrategy::SlidingWindow { keep_recent: 10 },
            storage: None,
        }
    }

    /// Set the compaction strategy
    pub fn with_compaction_strategy(mut self, strategy: CompactionStrategy) -> Self {
        self.compaction_strategy = strategy;
        self
    }

    /// Set the storage backend
    pub fn with_storage(mut self, storage: Arc<dyn TurnStorage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Create a new session
    pub async fn create_session(&self, _config: SessionConfig) -> Result<Session> {
        todo!("Implement session creation")
    }

    /// Process a new turn within a session
    pub async fn process_turn(&self, _session: &mut Session, _input: &str) -> Result<Turn> {
        todo!("Implement turn processing")
    }

    /// Compact session history
    pub async fn compact(&self, _session: &mut Session) -> Result<()> {
        todo!("Implement compaction")
    }

    /// Restore session from storage
    pub async fn restore_session(&self, _id: SessionId) -> Result<Session> {
        todo!("Implement session restoration")
    }
}

/// Session configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Initial agent
    pub agent_id: AgentId,
    /// Optional user ID
    pub user_id: Option<UserId>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Compaction strategy for context window management
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CompactionStrategy {
    /// Remove oldest turns first
    SlidingWindow {
        /// Number of recent turns to keep
        keep_recent: usize,
    },
    /// Summarize older turns
    Summarization {
        /// Summarize turns after this count
        summarize_after: usize,
    },
    /// Hybrid approach
    Hybrid {
        /// Number of recent turns to keep
        keep_recent: usize,
        /// Number of middle turns to summarize
        summarize_middle: usize,
    },
}

/// Trait for persistent turn storage
pub trait TurnStorage: Send + Sync {
    /// Store a session
    fn store_session(&self, session: &Session) -> Result<()>;

    /// Load a session
    fn load_session(&self, id: SessionId) -> Result<Option<Session>>;

    /// Store a turn
    fn store_turn(&self, turn: &Turn) -> Result<()>;

    /// Load turns for a session
    fn load_turns(&self, session_id: SessionId) -> Result<Vec<Turn>>;
}
