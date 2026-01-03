//! Agent File (.af) format for serializing and persisting complete agent state
//!
//! Inspired by Letta's Agent File format, this allows:
//! - Complete agent checkpointing
//! - Agent migration between servers
//! - Agent versioning and rollback
//! - Portable agent sharing

use crate::agent::Agent;
use crate::error::{Error, Result};
use crate::memory::{AgentMemory, MemoryBlock, MemoryConfig, MessageEntry};
use crate::react::ReActConfig;
use crate::types::AgentId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

/// Agent File format version
pub const AGENT_FILE_VERSION: &str = "1.0.0";

/// Complete serializable agent state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFile {
    /// File format version
    pub version: String,

    /// Agent metadata
    pub metadata: AgentMetadata,

    /// Agent configuration
    pub config: AgentConfig,

    /// Memory state
    pub memory: MemoryState,

    /// Message history
    pub messages: Vec<MessageEntry>,

    /// Custom data
    pub custom_data: HashMap<String, serde_json::Value>,
}

/// Agent metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Agent ID
    pub agent_id: String,

    /// Agent name
    pub name: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last modified timestamp
    pub updated_at: DateTime<Utc>,

    /// Agent description
    pub description: Option<String>,

    /// Agent tags
    pub tags: Vec<String>,

    /// Export timestamp
    pub exported_at: DateTime<Utc>,

    /// Export source
    pub exported_from: Option<String>,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// System prompt
    pub system_prompt: String,

    /// Model identifier
    pub model: String,

    /// ReAct configuration
    pub react_config: ReActConfig,

    /// Maximum reasoning loops
    pub max_loops: u32,

    /// Temperature
    pub temperature: f32,

    /// Client type ("openrouter", "vllm", etc.)
    pub client_type: String,

    /// Client endpoint
    pub client_endpoint: Option<String>,
}

/// Memory state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryState {
    /// Memory configuration
    pub config: MemoryConfig,

    /// All memory blocks
    pub blocks: Vec<MemoryBlock>,

    /// IDs of shared memory blocks (references only)
    pub shared_block_ids: Vec<String>,
}

impl AgentFile {
    /// Create a new agent file from an agent
    pub fn from_agent(
        agent: &Agent,
        memory: &AgentMemory,
        client_type: String,
        client_endpoint: Option<String>,
    ) -> Self {
        let now = Utc::now();

        Self {
            version: AGENT_FILE_VERSION.to_string(),
            metadata: AgentMetadata {
                agent_id: agent.id.to_string(),
                name: agent.name.clone(),
                created_at: now, // Would need to track this in Agent
                updated_at: now,
                description: None,
                tags: Vec::new(),
                exported_at: now,
                exported_from: None,
            },
            config: AgentConfig {
                system_prompt: agent.system_prompt.clone(),
                model: agent.model.model.clone(),
                react_config: agent.react_config.clone(),
                max_loops: agent.max_loops,
                temperature: agent.temperature,
                client_type,
                client_endpoint,
            },
            memory: MemoryState {
                config: memory.config.clone(),
                blocks: Vec::new(), // Would be populated from memory
                shared_block_ids: Vec::new(),
            },
            messages: Vec::new(), // Would be populated from memory.message_history
            custom_data: HashMap::new(),
        }
    }

    /// Save agent file to disk
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let serialized = serde_json::to_string_pretty(self)?;
        let mut file = File::create(path)?;
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }

    /// Load agent file from disk
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let agent_file: AgentFile = serde_json::from_str(&contents)?;

        // Verify version compatibility
        if agent_file.version != AGENT_FILE_VERSION {
            return Err(Error::config(format!(
                "Incompatible agent file version: expected {}, got {}",
                AGENT_FILE_VERSION, agent_file.version
            )));
        }

        Ok(agent_file)
    }

    /// Serialize to bytes (for network transfer)
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(bytes)?)
    }

    /// Get agent ID
    pub fn agent_id(&self) -> Result<AgentId> {
        let id = uuid::Uuid::parse_str(&self.metadata.agent_id)
            .map_err(|e| Error::config(format!("Invalid agent ID: {}", e)))?;
        Ok(AgentId::from_uuid(id))
    }
}

/// Agent checkpoint manager
pub struct CheckpointManager {
    /// Base directory for checkpoints
    checkpoint_dir: String,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new(checkpoint_dir: impl Into<String>) -> Self {
        Self {
            checkpoint_dir: checkpoint_dir.into(),
        }
    }

    /// Create a checkpoint for an agent
    pub fn checkpoint(
        &self,
        agent: &Agent,
        memory: &AgentMemory,
        client_type: String,
        client_endpoint: Option<String>,
    ) -> Result<String> {
        let agent_file = AgentFile::from_agent(agent, memory, client_type, client_endpoint);

        // Create checkpoint filename with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let filename = format!(
            "{}_{}.af",
            agent.name.replace(' ', "_").to_lowercase(),
            timestamp
        );

        let path = Path::new(&self.checkpoint_dir).join(&filename);

        // Ensure directory exists
        std::fs::create_dir_all(&self.checkpoint_dir)?;

        agent_file.save(&path)?;

        Ok(filename)
    }

    /// List all checkpoints for an agent
    pub fn list_checkpoints(&self, agent_name: &str) -> Result<Vec<String>> {
        let dir = Path::new(&self.checkpoint_dir);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let prefix = format!("{}_", agent_name.replace(' ', "_").to_lowercase());
        let mut checkpoints = Vec::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let filename = entry.file_name();
            let filename_str = filename.to_string_lossy();

            if filename_str.starts_with(&prefix) && filename_str.ends_with(".af") {
                checkpoints.push(filename_str.to_string());
            }
        }

        checkpoints.sort();
        Ok(checkpoints)
    }

    /// Load a specific checkpoint
    pub fn load_checkpoint(&self, filename: &str) -> Result<AgentFile> {
        let path = Path::new(&self.checkpoint_dir).join(filename);
        AgentFile::load(path)
    }

    /// Delete a checkpoint
    pub fn delete_checkpoint(&self, filename: &str) -> Result<()> {
        let path = Path::new(&self.checkpoint_dir).join(filename);
        std::fs::remove_file(path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentBuilder;
    use crate::types::AgentId;
    use tempfile::tempdir;

    #[test]
    fn test_agent_file_serialization() {
        let now = Utc::now();

        let agent_file = AgentFile {
            version: AGENT_FILE_VERSION.to_string(),
            metadata: AgentMetadata {
                agent_id: AgentId::new().to_string(),
                name: "Test Agent".to_string(),
                created_at: now,
                updated_at: now,
                description: Some("Test description".to_string()),
                tags: vec!["test".to_string()],
                exported_at: now,
                exported_from: Some("test-server".to_string()),
            },
            config: AgentConfig {
                system_prompt: "Test prompt".to_string(),
                model: "test-model".to_string(),
                react_config: ReActConfig::default(),
                max_loops: 5,
                temperature: 0.7,
                client_type: "test".to_string(),
                client_endpoint: None,
            },
            memory: MemoryState {
                config: MemoryConfig::default(),
                blocks: Vec::new(),
                shared_block_ids: Vec::new(),
            },
            messages: Vec::new(),
            custom_data: HashMap::new(),
        };

        // Test serialization/deserialization
        let bytes = agent_file.to_bytes().unwrap();
        let deserialized = AgentFile::from_bytes(&bytes).unwrap();

        assert_eq!(agent_file.version, deserialized.version);
        assert_eq!(agent_file.metadata.name, deserialized.metadata.name);
    }

    #[test]
    fn test_checkpoint_manager() {
        let temp_dir = tempdir().unwrap();
        let manager = CheckpointManager::new(temp_dir.path().to_str().unwrap());

        // Test listing empty checkpoints
        let checkpoints = manager.list_checkpoints("test_agent").unwrap();
        assert_eq!(checkpoints.len(), 0);
    }
}
