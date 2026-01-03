//! Memory system for stateful, self-improving agents
//!
//! Inspired by Letta's memory architecture, this module implements:
//! - Memory hierarchy (in-context vs out-of-context)
//! - Editable memory blocks with persistence
//! - Agentic context engineering (agents control their memory)
//! - Shared memory blocks for multi-agent coordination
//! - Perpetual message history with Agent File (.af) format

use crate::error::{Error, Result};
use crate::types::AgentId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[cfg(feature = "storage")]
use crate::storage::MemoryStorage;

/// Unique identifier for a memory block
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryBlockId(Uuid);

impl MemoryBlockId {
    /// Create a new memory block ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MemoryBlockId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MemoryBlockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Memory block - a persistent, editable chunk of agent memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBlock {
    /// Unique identifier
    pub id: MemoryBlockId,

    /// Human-readable label (e.g., "persona", "organization", "task_context")
    pub label: String,

    /// Description of what this block contains
    pub description: String,

    /// The actual memory content (editable by agent)
    pub value: String,

    /// Maximum size in characters (for context window management)
    pub max_size: Option<usize>,

    /// Whether this block is currently in the context window
    pub in_context: bool,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,

    /// Metadata for custom fields
    pub metadata: HashMap<String, String>,
}

impl MemoryBlock {
    /// Create a new memory block
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: MemoryBlockId::new(),
            label: label.into(),
            description: String::new(),
            value: value.into(),
            max_size: None,
            in_context: true, // Default to in-context
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Create a new memory block with description
    pub fn with_description(
        label: impl Into<String>,
        description: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        let mut block = Self::new(label, value);
        block.description = description.into();
        block
    }

    /// Update the value of this memory block
    pub fn update_value(&mut self, new_value: impl Into<String>) -> Result<()> {
        let new_val = new_value.into();

        // Check max size constraint
        if let Some(max) = self.max_size {
            if new_val.len() > max {
                return Err(Error::config(format!(
                    "Memory block '{}' value exceeds max size {} (got {})",
                    self.label,
                    max,
                    new_val.len()
                )));
            }
        }

        self.value = new_val;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Append content to this memory block
    pub fn append(&mut self, content: impl Into<String>) -> Result<()> {
        let new_value = format!("{}\n{}", self.value, content.into());
        self.update_value(new_value)
    }

    /// Set whether this block is in context
    pub fn set_in_context(&mut self, in_context: bool) {
        self.in_context = in_context;
        self.updated_at = Utc::now();
    }

    /// Get the size of this block in characters
    pub fn size(&self) -> usize {
        self.value.len()
    }
}

/// Memory hierarchy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum total size of in-context memory (characters)
    pub max_context_size: usize,

    /// Enable automatic context management (agents can move blocks in/out)
    pub enable_agentic_control: bool,

    /// Enable sleep-time agent for background memory processing
    pub enable_sleeptime: bool,

    /// Storage backend configuration
    pub storage_backend: StorageBackend,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_context_size: 8000, // ~8K characters for in-context memory
            enable_agentic_control: true,
            enable_sleeptime: false,
            storage_backend: StorageBackend::Sqlite {
                path: "spai_memory.db".to_string(),
            },
        }
    }
}

/// Storage backend for persisting memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageBackend {
    /// SQLite database (default)
    Sqlite { path: String },

    /// PostgreSQL database
    Postgres { connection_string: String },

    /// In-memory only (no persistence)
    Memory,
}

/// Agent memory manager - handles all memory blocks for an agent
#[derive(Debug, Clone)]
pub struct AgentMemory {
    /// Agent this memory belongs to
    pub agent_id: AgentId,

    /// Memory blocks owned by this agent
    blocks: Arc<RwLock<HashMap<MemoryBlockId, MemoryBlock>>>,

    /// Shared memory blocks (references to blocks owned by other agents)
    shared_blocks: Arc<RwLock<Vec<MemoryBlockId>>>,

    /// Configuration
    pub config: MemoryConfig,

    /// Message history (for perpetual agents)
    message_history: Arc<RwLock<Vec<MessageEntry>>>,
}

/// A single message in the agent's perpetual history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEntry {
    /// Unique message ID
    pub id: Uuid,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Role (user, assistant, system, tool)
    pub role: String,

    /// Message content
    pub content: String,

    /// Optional tool call information
    pub tool_calls: Option<Vec<String>>,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl AgentMemory {
    /// Create a new agent memory manager
    pub fn new(agent_id: AgentId, config: MemoryConfig) -> Self {
        Self {
            agent_id,
            blocks: Arc::new(RwLock::new(HashMap::new())),
            shared_blocks: Arc::new(RwLock::new(Vec::new())),
            config,
            message_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a new memory block
    pub async fn add_block(&self, block: MemoryBlock) -> Result<MemoryBlockId> {
        let id = block.id;
        let mut blocks = self.blocks.write().await;
        blocks.insert(id, block);
        Ok(id)
    }

    /// Get a memory block by ID
    pub async fn get_block(&self, id: MemoryBlockId) -> Option<MemoryBlock> {
        let blocks = self.blocks.read().await;
        blocks.get(&id).cloned()
    }

    /// Update a memory block's value
    pub async fn update_block(&self, id: MemoryBlockId, new_value: String) -> Result<()> {
        let mut blocks = self.blocks.write().await;

        if let Some(block) = blocks.get_mut(&id) {
            block.update_value(new_value)?;
            Ok(())
        } else {
            Err(Error::config(format!("Memory block {} not found", id)))
        }
    }

    /// Delete a memory block
    pub async fn delete_block(&self, id: MemoryBlockId) -> Result<()> {
        let mut blocks = self.blocks.write().await;

        if blocks.remove(&id).is_some() {
            Ok(())
        } else {
            Err(Error::config(format!("Memory block {} not found", id)))
        }
    }

    /// Attach a shared memory block (by ID)
    pub async fn attach_shared_block(&self, block_id: MemoryBlockId) {
        let mut shared = self.shared_blocks.write().await;
        if !shared.contains(&block_id) {
            shared.push(block_id);
        }
    }

    /// Get all in-context memory blocks
    pub async fn in_context_blocks(&self) -> Vec<MemoryBlock> {
        let blocks = self.blocks.read().await;
        blocks
            .values()
            .filter(|b| b.in_context)
            .cloned()
            .collect()
    }

    /// Get all out-of-context memory blocks
    pub async fn out_of_context_blocks(&self) -> Vec<MemoryBlock> {
        let blocks = self.blocks.read().await;
        blocks
            .values()
            .filter(|b| !b.in_context)
            .cloned()
            .collect()
    }

    /// Calculate total in-context memory size
    pub async fn context_size(&self) -> usize {
        let blocks = self.in_context_blocks().await;
        blocks.iter().map(|b| b.size()).sum()
    }

    /// Move a block out of context (to save context window space)
    pub async fn move_out_of_context(&self, id: MemoryBlockId) -> Result<()> {
        let mut blocks = self.blocks.write().await;

        if let Some(block) = blocks.get_mut(&id) {
            block.set_in_context(false);
            Ok(())
        } else {
            Err(Error::config(format!("Memory block {} not found", id)))
        }
    }

    /// Move a block into context
    pub async fn move_into_context(&self, id: MemoryBlockId) -> Result<()> {
        let mut blocks = self.blocks.write().await;

        // Check if adding this block would exceed max context size
        let current_size: usize = blocks
            .values()
            .filter(|b| b.in_context && b.id != id)
            .map(|b| b.size())
            .sum();

        let (block_size, block_label) = match blocks.get(&id) {
            Some(block) => (block.size(), block.label.clone()),
            None => return Err(Error::config(format!("Memory block {} not found", id))),
        };

        if current_size + block_size > self.config.max_context_size {
            return Err(Error::config(format!(
                "Adding block '{}' would exceed max context size ({} + {} > {})",
                block_label, current_size, block_size, self.config.max_context_size
            )));
        }

        if let Some(block) = blocks.get_mut(&id) {
            block.set_in_context(true);
            Ok(())
        } else {
            Err(Error::config(format!("Memory block {} not found", id)))
        }
    }

    /// Add a message to the perpetual history
    pub async fn add_message(&self, role: String, content: String) -> Uuid {
        let message = MessageEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            role,
            content,
            tool_calls: None,
            metadata: HashMap::new(),
        };

        let id = message.id;
        let mut history = self.message_history.write().await;
        history.push(message);
        id
    }

    /// Get message history (last N messages)
    pub async fn get_recent_messages(&self, limit: usize) -> Vec<MessageEntry> {
        let history = self.message_history.read().await;
        let start = history.len().saturating_sub(limit);
        history[start..].to_vec()
    }

    /// Search message history by content
    pub async fn search_messages(&self, query: &str) -> Vec<MessageEntry> {
        let history = self.message_history.read().await;
        history
            .iter()
            .filter(|m| m.content.contains(query))
            .cloned()
            .collect()
    }

    /// Load blocks + messages from a persistent storage backend.
    #[cfg(feature = "storage")]
    pub async fn load_from_storage(
        &self,
        storage: &dyn MemoryStorage,
        message_limit: usize,
    ) -> Result<()> {
        let blocks = storage.load_agent_blocks(self.agent_id).await?;
        let messages = storage.load_messages(self.agent_id, message_limit).await?;

        {
            let mut blocks_map = self.blocks.write().await;
            blocks_map.clear();
            for block in blocks {
                blocks_map.insert(block.id, block);
            }
        }

        {
            let mut history = self.message_history.write().await;
            history.clear();
            history.extend(messages);
        }

        Ok(())
    }

    /// Persist all current blocks + messages to a storage backend.
    #[cfg(feature = "storage")]
    pub async fn persist_to_storage(&self, storage: &dyn MemoryStorage) -> Result<()> {
        let blocks: Vec<MemoryBlock> = {
            let blocks_map = self.blocks.read().await;
            blocks_map.values().cloned().collect()
        };

        for block in &blocks {
            storage.save_block(self.agent_id, block).await?;
        }

        let messages: Vec<MessageEntry> = {
            let history = self.message_history.read().await;
            history.clone()
        };

        for message in &messages {
            storage.save_message(self.agent_id, message).await?;
        }

        Ok(())
    }

    /// Get all memory blocks (owned + shared)
    pub async fn all_blocks(&self, shared_memory_manager: Option<&SharedMemoryManager>) -> Vec<MemoryBlock> {
        let mut all_blocks = Vec::new();

        // Add owned blocks
        let blocks = self.blocks.read().await;
        all_blocks.extend(blocks.values().cloned());

        // Add shared blocks if manager provided
        if let Some(manager) = shared_memory_manager {
            let shared_ids = self.shared_blocks.read().await;
            for id in shared_ids.iter() {
                if let Some(block) = manager.get_block(*id).await {
                    all_blocks.push(block);
                }
            }
        }

        all_blocks
    }
}

/// Shared memory manager - manages blocks shared across multiple agents
#[derive(Debug, Clone)]
pub struct SharedMemoryManager {
    /// All shared memory blocks
    blocks: Arc<RwLock<HashMap<MemoryBlockId, MemoryBlock>>>,
}

impl SharedMemoryManager {
    /// Create a new shared memory manager
    pub fn new() -> Self {
        Self {
            blocks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new shared block
    pub async fn create_block(
        &self,
        label: impl Into<String>,
        description: impl Into<String>,
        value: impl Into<String>,
    ) -> MemoryBlockId {
        let block = MemoryBlock::with_description(label, description, value);
        let id = block.id;

        let mut blocks = self.blocks.write().await;
        blocks.insert(id, block);
        id
    }

    /// Get a shared block by ID
    pub async fn get_block(&self, id: MemoryBlockId) -> Option<MemoryBlock> {
        let blocks = self.blocks.read().await;
        blocks.get(&id).cloned()
    }

    /// Update a shared block
    pub async fn update_block(&self, id: MemoryBlockId, new_value: String) -> Result<()> {
        let mut blocks = self.blocks.write().await;

        if let Some(block) = blocks.get_mut(&id) {
            block.update_value(new_value)?;
            Ok(())
        } else {
            Err(Error::config(format!("Shared block {} not found", id)))
        }
    }
}

impl Default for SharedMemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_block_creation() {
        let block = MemoryBlock::new("persona", "I am a helpful assistant");
        assert_eq!(block.label, "persona");
        assert_eq!(block.value, "I am a helpful assistant");
        assert!(block.in_context);
    }

    #[tokio::test]
    async fn test_memory_block_update() {
        let mut block = MemoryBlock::new("test", "original");
        block.update_value("updated").unwrap();
        assert_eq!(block.value, "updated");
    }

    #[tokio::test]
    async fn test_agent_memory_basics() {
        let memory = AgentMemory::new(AgentId::new(), MemoryConfig::default());

        let block = MemoryBlock::new("persona", "I am an agent");
        let id = memory.add_block(block).await.unwrap();

        let retrieved = memory.get_block(id).await.unwrap();
        assert_eq!(retrieved.label, "persona");
    }

    #[tokio::test]
    async fn test_context_management() {
        let memory = AgentMemory::new(AgentId::new(), MemoryConfig::default());

        let block = MemoryBlock::new("test", "content");
        let id = memory.add_block(block).await.unwrap();

        // Initially in context
        assert_eq!(memory.in_context_blocks().await.len(), 1);

        // Move out of context
        memory.move_out_of_context(id).await.unwrap();
        assert_eq!(memory.in_context_blocks().await.len(), 0);
        assert_eq!(memory.out_of_context_blocks().await.len(), 1);
    }

    #[tokio::test]
    async fn test_shared_memory() {
        let shared_manager = SharedMemoryManager::new();

        let block_id = shared_manager
            .create_block("organization", "Shared org info", "Acme Corp")
            .await;

        let agent1_memory = AgentMemory::new(AgentId::new(), MemoryConfig::default());
        let agent2_memory = AgentMemory::new(AgentId::new(), MemoryConfig::default());

        agent1_memory.attach_shared_block(block_id).await;
        agent2_memory.attach_shared_block(block_id).await;

        let block = shared_manager.get_block(block_id).await.unwrap();
        assert_eq!(block.value, "Acme Corp");
    }
}
