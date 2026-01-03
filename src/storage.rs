//! Persistent storage backends for agent memory
//!
//! This module provides:
//! - Storage trait for abstracting backend implementations
//! - SQLite backend for local persistence
//! - PostgreSQL backend for distributed deployments
//! - Automatic migrations
//! - Memory block and message history persistence

#[cfg(feature = "storage")]
use crate::error::{Error, Result};
#[cfg(feature = "storage")]
use crate::memory::{MemoryBlock, MemoryBlockId, MessageEntry};
#[cfg(feature = "storage")]
use crate::types::AgentId;
#[cfg(feature = "storage")]
use async_trait::async_trait;
#[cfg(feature = "storage")]
use chrono::{DateTime, Utc};

#[cfg(feature = "storage")]
use sqlx::{Pool, Postgres, Row, Sqlite};

/// Trait for persistent storage of agent memory
#[cfg(feature = "storage")]
#[async_trait]
pub trait MemoryStorage: Send + Sync {
    /// Save or update a memory block
    async fn save_block(&self, agent_id: AgentId, block: &MemoryBlock) -> Result<()>;

    /// Load a memory block by ID
    async fn load_block(&self, block_id: MemoryBlockId) -> Result<Option<MemoryBlock>>;

    /// Load all memory blocks for an agent
    async fn load_agent_blocks(&self, agent_id: AgentId) -> Result<Vec<MemoryBlock>>;

    /// Delete a memory block
    async fn delete_block(&self, block_id: MemoryBlockId) -> Result<()>;

    /// Save a message to history
    async fn save_message(&self, agent_id: AgentId, message: &MessageEntry) -> Result<()>;

    /// Load recent messages for an agent
    async fn load_messages(&self, agent_id: AgentId, limit: usize) -> Result<Vec<MessageEntry>>;

    /// Search messages by content
    async fn search_messages(&self, agent_id: AgentId, query: &str) -> Result<Vec<MessageEntry>>;

    /// Delete all data for an agent
    async fn delete_agent_data(&self, agent_id: AgentId) -> Result<()>;
}

/// SQLite storage backend
#[cfg(feature = "storage")]
pub struct SqliteStorage {
    pool: Pool<Sqlite>,
}

#[cfg(feature = "storage")]
impl SqliteStorage {
    /// Create a new SQLite storage backend
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = sqlx::SqlitePool::connect(database_url)
            .await
            .map_err(|e| Error::config(format!("Failed to connect to SQLite: {}", e)))?;

        let storage = Self { pool };
        storage.run_migrations().await?;

        Ok(storage)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        // Create memory_blocks table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS memory_blocks (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                label TEXT NOT NULL,
                description TEXT NOT NULL,
                value TEXT NOT NULL,
                max_size INTEGER,
                in_context INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                metadata TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to create memory_blocks table: {}", e)))?;

        // Create messages table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                tool_calls TEXT,
                metadata TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to create messages table: {}", e)))?;

        // Create indices
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_memory_blocks_agent ON memory_blocks(agent_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::config(format!("Failed to create index: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_agent ON messages(agent_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::config(format!("Failed to create index: {}", e)))?;

        Ok(())
    }
}

#[cfg(feature = "storage")]
#[async_trait]
impl MemoryStorage for SqliteStorage {
    async fn save_block(&self, agent_id: AgentId, block: &MemoryBlock) -> Result<()> {
        let metadata_json = serde_json::to_string(&block.metadata)
            .map_err(|e| Error::config(format!("Failed to serialize metadata: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO memory_blocks
            (id, agent_id, label, description, value, max_size, in_context, created_at, updated_at, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(block.id.to_string())
        .bind(agent_id.to_string())
        .bind(&block.label)
        .bind(&block.description)
        .bind(&block.value)
        .bind(block.max_size.map(|s| s as i64))
        .bind(if block.in_context { 1 } else { 0 })
        .bind(block.created_at.to_rfc3339())
        .bind(block.updated_at.to_rfc3339())
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to save memory block: {}", e)))?;

        Ok(())
    }

    async fn load_block(&self, block_id: MemoryBlockId) -> Result<Option<MemoryBlock>> {
        let row = sqlx::query(
            r#"
            SELECT id, label, description, value, max_size, in_context, created_at, updated_at, metadata
            FROM memory_blocks WHERE id = ?
            "#,
        )
        .bind(block_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to load memory block: {}", e)))?;

        if let Some(row) = row {
            let id_str: String = row.get(0);
            let id = serde_json::from_str(&format!("\"{}\"", id_str))
                .map_err(|e| Error::config(format!("Invalid block ID: {}", e)))?;

            let max_size: Option<i64> = row.get(4);
            let in_context: i32 = row.get(5);
            let created_str: String = row.get(6);
            let updated_str: String = row.get(7);
            let metadata_json: String = row.get(8);

            Ok(Some(MemoryBlock {
                id,
                label: row.get(1),
                description: row.get(2),
                value: row.get(3),
                max_size: max_size.map(|s| s as usize),
                in_context: in_context != 0,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map_err(|e| Error::config(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_str)
                    .map_err(|e| Error::config(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc),
                metadata: serde_json::from_str(&metadata_json)
                    .map_err(|e| Error::config(format!("Invalid metadata JSON: {}", e)))?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn load_agent_blocks(&self, agent_id: AgentId) -> Result<Vec<MemoryBlock>> {
        let rows = sqlx::query(
            r#"
            SELECT id, label, description, value, max_size, in_context, created_at, updated_at, metadata
            FROM memory_blocks WHERE agent_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(agent_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to load agent blocks: {}", e)))?;

        let mut blocks = Vec::new();
        for row in rows {
            let id_str: String = row.get(0);
            let id = serde_json::from_str(&format!("\"{}\"", id_str))
                .map_err(|e| Error::config(format!("Invalid block ID: {}", e)))?;

            let max_size: Option<i64> = row.get(4);
            let in_context: i32 = row.get(5);
            let created_str: String = row.get(6);
            let updated_str: String = row.get(7);
            let metadata_json: String = row.get(8);

            blocks.push(MemoryBlock {
                id,
                label: row.get(1),
                description: row.get(2),
                value: row.get(3),
                max_size: max_size.map(|s| s as usize),
                in_context: in_context != 0,
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map_err(|e| Error::config(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&updated_str)
                    .map_err(|e| Error::config(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc),
                metadata: serde_json::from_str(&metadata_json)
                    .map_err(|e| Error::config(format!("Invalid metadata JSON: {}", e)))?,
            });
        }

        Ok(blocks)
    }

    async fn delete_block(&self, block_id: MemoryBlockId) -> Result<()> {
        sqlx::query("DELETE FROM memory_blocks WHERE id = ?")
            .bind(block_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| Error::config(format!("Failed to delete memory block: {}", e)))?;

        Ok(())
    }

    async fn save_message(&self, agent_id: AgentId, message: &MessageEntry) -> Result<()> {
        let tool_calls_json = message
            .tool_calls
            .as_ref()
            .map(|tc| serde_json::to_string(tc))
            .transpose()
            .map_err(|e| Error::config(format!("Failed to serialize tool_calls: {}", e)))?;

        let metadata_json = serde_json::to_string(&message.metadata)
            .map_err(|e| Error::config(format!("Failed to serialize metadata: {}", e)))?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO messages
            (id, agent_id, timestamp, role, content, tool_calls, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(message.id.to_string())
        .bind(agent_id.to_string())
        .bind(message.timestamp.to_rfc3339())
        .bind(&message.role)
        .bind(&message.content)
        .bind(tool_calls_json)
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to save message: {}", e)))?;

        Ok(())
    }

    async fn load_messages(&self, agent_id: AgentId, limit: usize) -> Result<Vec<MessageEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT id, timestamp, role, content, tool_calls, metadata
            FROM messages WHERE agent_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(agent_id.to_string())
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to load messages: {}", e)))?;

        let mut messages = Vec::new();
        for row in rows {
            let id_str: String = row.get(0);
            let timestamp_str: String = row.get(1);
            let tool_calls_json: Option<String> = row.get(4);
            let metadata_json: String = row.get(5);

            messages.push(MessageEntry {
                id: uuid::Uuid::parse_str(&id_str)
                    .map_err(|e| Error::config(format!("Invalid message ID: {}", e)))?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|e| Error::config(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc),
                role: row.get(2),
                content: row.get(3),
                tool_calls: tool_calls_json
                    .map(|json| serde_json::from_str(&json))
                    .transpose()
                    .map_err(|e| Error::config(format!("Invalid tool_calls JSON: {}", e)))?,
                metadata: serde_json::from_str(&metadata_json)
                    .map_err(|e| Error::config(format!("Invalid metadata JSON: {}", e)))?,
            });
        }

        messages.reverse(); // Return in chronological order
        Ok(messages)
    }

    async fn search_messages(&self, agent_id: AgentId, query: &str) -> Result<Vec<MessageEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT id, timestamp, role, content, tool_calls, metadata
            FROM messages WHERE agent_id = ? AND content LIKE ?
            ORDER BY timestamp DESC
            "#,
        )
        .bind(agent_id.to_string())
        .bind(format!("%{}%", query))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to search messages: {}", e)))?;

        let mut messages = Vec::new();
        for row in rows {
            let id_str: String = row.get(0);
            let timestamp_str: String = row.get(1);
            let tool_calls_json: Option<String> = row.get(4);
            let metadata_json: String = row.get(5);

            messages.push(MessageEntry {
                id: uuid::Uuid::parse_str(&id_str)
                    .map_err(|e| Error::config(format!("Invalid message ID: {}", e)))?,
                timestamp: DateTime::parse_from_rfc3339(&timestamp_str)
                    .map_err(|e| Error::config(format!("Invalid timestamp: {}", e)))?
                    .with_timezone(&Utc),
                role: row.get(2),
                content: row.get(3),
                tool_calls: tool_calls_json
                    .map(|json| serde_json::from_str(&json))
                    .transpose()
                    .map_err(|e| Error::config(format!("Invalid tool_calls JSON: {}", e)))?,
                metadata: serde_json::from_str(&metadata_json)
                    .map_err(|e| Error::config(format!("Invalid metadata JSON: {}", e)))?,
            });
        }

        Ok(messages)
    }

    async fn delete_agent_data(&self, agent_id: AgentId) -> Result<()> {
        sqlx::query("DELETE FROM memory_blocks WHERE agent_id = ?")
            .bind(agent_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| Error::config(format!("Failed to delete agent blocks: {}", e)))?;

        sqlx::query("DELETE FROM messages WHERE agent_id = ?")
            .bind(agent_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| Error::config(format!("Failed to delete agent messages: {}", e)))?;

        Ok(())
    }
}

/// PostgreSQL storage backend
#[cfg(feature = "storage")]
pub struct PostgresStorage {
    pool: Pool<Postgres>,
}

#[cfg(feature = "storage")]
impl PostgresStorage {
    /// Create a new PostgreSQL storage backend
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = sqlx::PgPool::connect(database_url)
            .await
            .map_err(|e| Error::config(format!("Failed to connect to PostgreSQL: {}", e)))?;

        let storage = Self { pool };
        storage.run_migrations().await?;

        Ok(storage)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        // Create memory_blocks table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS memory_blocks (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                label TEXT NOT NULL,
                description TEXT NOT NULL,
                value TEXT NOT NULL,
                max_size INTEGER,
                in_context BOOLEAN NOT NULL,
                created_at TIMESTAMPTZ NOT NULL,
                updated_at TIMESTAMPTZ NOT NULL,
                metadata JSONB NOT NULL DEFAULT '{}'
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to create memory_blocks table: {}", e)))?;

        // Create messages table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id UUID PRIMARY KEY,
                agent_id TEXT NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                tool_calls JSONB,
                metadata JSONB NOT NULL DEFAULT '{}'
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to create messages table: {}", e)))?;

        // Create indices
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_memory_blocks_agent ON memory_blocks(agent_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::config(format!("Failed to create index: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_agent ON messages(agent_id)")
            .execute(&self.pool)
            .await
            .map_err(|e| Error::config(format!("Failed to create index: {}", e)))?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_content ON messages USING gin(to_tsvector('english', content))")
            .execute(&self.pool)
            .await
            .ok(); // Ignore error if GIN extension not available

        Ok(())
    }
}

#[cfg(feature = "storage")]
#[async_trait]
impl MemoryStorage for PostgresStorage {
    async fn save_block(&self, agent_id: AgentId, block: &MemoryBlock) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO memory_blocks
            (id, agent_id, label, description, value, max_size, in_context, created_at, updated_at, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                label = EXCLUDED.label,
                description = EXCLUDED.description,
                value = EXCLUDED.value,
                max_size = EXCLUDED.max_size,
                in_context = EXCLUDED.in_context,
                updated_at = EXCLUDED.updated_at,
                metadata = EXCLUDED.metadata
            "#,
        )
        .bind(block.id.to_string())
        .bind(agent_id.to_string())
        .bind(&block.label)
        .bind(&block.description)
        .bind(&block.value)
        .bind(block.max_size.map(|s| s as i64))
        .bind(block.in_context)
        .bind(block.created_at)
        .bind(block.updated_at)
        .bind(serde_json::to_value(&block.metadata).unwrap())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to save memory block: {}", e)))?;

        Ok(())
    }

    async fn load_block(&self, block_id: MemoryBlockId) -> Result<Option<MemoryBlock>> {
        let row = sqlx::query_as::<_, (String, String, String, String, Option<i64>, bool, DateTime<Utc>, DateTime<Utc>, serde_json::Value)>(
            r#"
            SELECT id, label, description, value, max_size, in_context, created_at, updated_at, metadata
            FROM memory_blocks WHERE id = $1
            "#,
        )
        .bind(block_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to load memory block: {}", e)))?;

        if let Some((id_str, label, description, value, max_size, in_context, created_at, updated_at, metadata)) = row {
            let id = serde_json::from_str(&format!("\"{}\"", id_str))
                .map_err(|e| Error::config(format!("Invalid block ID: {}", e)))?;

            Ok(Some(MemoryBlock {
                id,
                label,
                description,
                value,
                max_size: max_size.map(|s| s as usize),
                in_context,
                created_at,
                updated_at,
                metadata: serde_json::from_value(metadata)
                    .map_err(|e| Error::config(format!("Invalid metadata: {}", e)))?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn load_agent_blocks(&self, agent_id: AgentId) -> Result<Vec<MemoryBlock>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, Option<i64>, bool, DateTime<Utc>, DateTime<Utc>, serde_json::Value)>(
            r#"
            SELECT id, label, description, value, max_size, in_context, created_at, updated_at, metadata
            FROM memory_blocks WHERE agent_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(agent_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to load agent blocks: {}", e)))?;

        let mut blocks = Vec::new();
        for (id_str, label, description, value, max_size, in_context, created_at, updated_at, metadata) in rows {
            let id = serde_json::from_str(&format!("\"{}\"", id_str))
                .map_err(|e| Error::config(format!("Invalid block ID: {}", e)))?;

            blocks.push(MemoryBlock {
                id,
                label,
                description,
                value,
                max_size: max_size.map(|s| s as usize),
                in_context,
                created_at,
                updated_at,
                metadata: serde_json::from_value(metadata)
                    .map_err(|e| Error::config(format!("Invalid metadata: {}", e)))?,
            });
        }

        Ok(blocks)
    }

    async fn delete_block(&self, block_id: MemoryBlockId) -> Result<()> {
        sqlx::query("DELETE FROM memory_blocks WHERE id = $1")
            .bind(block_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| Error::config(format!("Failed to delete memory block: {}", e)))?;

        Ok(())
    }

    async fn save_message(&self, agent_id: AgentId, message: &MessageEntry) -> Result<()> {
        let tool_calls_json = message
            .tool_calls
            .as_ref()
            .map(|tc| serde_json::to_value(tc))
            .transpose()
            .map_err(|e| Error::config(format!("Failed to serialize tool_calls: {}", e)))?;

        sqlx::query(
            r#"
            INSERT INTO messages
            (id, agent_id, timestamp, role, content, tool_calls, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(message.id)
        .bind(agent_id.to_string())
        .bind(message.timestamp)
        .bind(&message.role)
        .bind(&message.content)
        .bind(tool_calls_json)
        .bind(serde_json::to_value(&message.metadata).unwrap())
        .execute(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to save message: {}", e)))?;

        Ok(())
    }

    async fn load_messages(&self, agent_id: AgentId, limit: usize) -> Result<Vec<MessageEntry>> {
        let rows = sqlx::query_as::<_, (uuid::Uuid, DateTime<Utc>, String, String, Option<serde_json::Value>, serde_json::Value)>(
            r#"
            SELECT id, timestamp, role, content, tool_calls, metadata
            FROM messages WHERE agent_id = $1
            ORDER BY timestamp DESC
            LIMIT $2
            "#,
        )
        .bind(agent_id.to_string())
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to load messages: {}", e)))?;

        let mut messages = Vec::new();
        for (id, timestamp, role, content, tool_calls_json, metadata) in rows {
            messages.push(MessageEntry {
                id,
                timestamp,
                role,
                content,
                tool_calls: tool_calls_json
                    .map(|json| serde_json::from_value(json))
                    .transpose()
                    .map_err(|e| Error::config(format!("Invalid tool_calls: {}", e)))?,
                metadata: serde_json::from_value(metadata)
                    .map_err(|e| Error::config(format!("Invalid metadata: {}", e)))?,
            });
        }

        messages.reverse(); // Return in chronological order
        Ok(messages)
    }

    async fn search_messages(&self, agent_id: AgentId, query: &str) -> Result<Vec<MessageEntry>> {
        let rows = sqlx::query_as::<_, (uuid::Uuid, DateTime<Utc>, String, String, Option<serde_json::Value>, serde_json::Value)>(
            r#"
            SELECT id, timestamp, role, content, tool_calls, metadata
            FROM messages WHERE agent_id = $1 AND content ILIKE $2
            ORDER BY timestamp DESC
            "#,
        )
        .bind(agent_id.to_string())
        .bind(format!("%{}%", query))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::config(format!("Failed to search messages: {}", e)))?;

        let mut messages = Vec::new();
        for (id, timestamp, role, content, tool_calls_json, metadata) in rows {
            messages.push(MessageEntry {
                id,
                timestamp,
                role,
                content,
                tool_calls: tool_calls_json
                    .map(|json| serde_json::from_value(json))
                    .transpose()
                    .map_err(|e| Error::config(format!("Invalid tool_calls: {}", e)))?,
                metadata: serde_json::from_value(metadata)
                    .map_err(|e| Error::config(format!("Invalid metadata: {}", e)))?,
            });
        }

        Ok(messages)
    }

    async fn delete_agent_data(&self, agent_id: AgentId) -> Result<()> {
        sqlx::query("DELETE FROM memory_blocks WHERE agent_id = $1")
            .bind(agent_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| Error::config(format!("Failed to delete agent blocks: {}", e)))?;

        sqlx::query("DELETE FROM messages WHERE agent_id = $1")
            .bind(agent_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| Error::config(format!("Failed to delete agent messages: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "storage")]
mod tests {
    use super::*;
    use crate::memory::MemoryBlock;
    use crate::types::AgentId;

    #[tokio::test]
    async fn test_sqlite_storage() {
        let storage = SqliteStorage::new("sqlite::memory:")
            .await
            .expect("Failed to create SQLite storage");

        let agent_id = AgentId::new();
        let block = MemoryBlock::new("test", "test value");

        storage
            .save_block(agent_id, &block)
            .await
            .expect("Failed to save block");

        let loaded = storage
            .load_block(block.id)
            .await
            .expect("Failed to load block")
            .expect("Block not found");

        assert_eq!(loaded.label, "test");
        assert_eq!(loaded.value, "test value");
    }
}
