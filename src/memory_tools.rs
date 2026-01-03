//! Memory management tools for agentic context engineering
//!
//! These tools allow agents to:
//! - Edit their own memory blocks
//! - Move blocks in/out of context
//! - Search their memory
//! - Manage the context window

use crate::error::Result;
use crate::memory::{AgentMemory, MemoryBlockId};
use crate::tools::{JsonSchema, Tool, ToolContext, ToolOutput};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Tool for updating a memory block's content
pub struct UpdateMemoryTool {
    memory: Arc<AgentMemory>,
}

impl UpdateMemoryTool {
    pub fn new(memory: Arc<AgentMemory>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl Tool for UpdateMemoryTool {
    fn id(&self) -> &str {
        "update_memory"
    }

    fn description(&self) -> &str {
        "Update the content of a memory block. Use this to edit your persona, \
         save important information, or modify any of your memory blocks."
    }

    fn name(&self) -> &str {
        self.id()
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "block_id".to_string(),
            json!({
                "type": "string",
                "description": "The ID of the memory block to update"
            }),
        );
        properties.insert(
            "new_value".to_string(),
            json!({
                "type": "string",
                "description": "The new content for this memory block"
            }),
        );

        JsonSchema::object(properties)
            .with_required(vec!["block_id".to_string(), "new_value".to_string()])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let block_id: String = params["block_id"]
            .as_str()
            .ok_or_else(|| crate::error::Error::tool_execution("update_memory", "Missing block_id"))?
            .to_string();

        let new_value = params["new_value"]
            .as_str()
            .ok_or_else(|| crate::error::Error::tool_execution("update_memory", "Missing new_value"))?
            .to_string();

        // Parse block ID
        let id: MemoryBlockId = serde_json::from_str(&format!("\"{}\"", block_id))?;

        // Update the block
        self.memory.update_block(id, new_value).await?;

        Ok(ToolOutput::success_with_data(
            format!("Successfully updated memory block {}", block_id),
            json!({"block_id": block_id}),
        ))
    }
}

/// Tool for moving a memory block out of context (to save context window space)
pub struct MoveOutOfContextTool {
    memory: Arc<AgentMemory>,
}

impl MoveOutOfContextTool {
    pub fn new(memory: Arc<AgentMemory>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl Tool for MoveOutOfContextTool {
    fn id(&self) -> &str {
        "move_out_of_context"
    }

    fn description(&self) -> &str {
        "Move a memory block out of the active context window. Use this when you need \
         to free up context space but don't want to delete the memory. You can bring it \
         back later with move_into_context."
    }

    fn name(&self) -> &str {
        self.id()
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "block_id".to_string(),
            json!({
                "type": "string",
                "description": "The ID of the memory block to move out of context"
            }),
        );

        JsonSchema::object(properties).with_required(vec!["block_id".to_string()])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let block_id: String = params["block_id"]
            .as_str()
            .ok_or_else(|| {
                crate::error::Error::tool_execution("move_out_of_context", "Missing block_id")
            })?
            .to_string();

        let id: MemoryBlockId = serde_json::from_str(&format!("\"{}\"", block_id))?;

        self.memory.move_out_of_context(id).await?;

        Ok(ToolOutput::success_with_data(
            format!("Moved memory block {} out of context", block_id),
            json!({"block_id": block_id}),
        ))
    }
}

/// Tool for moving a memory block into context
pub struct MoveIntoContextTool {
    memory: Arc<AgentMemory>,
}

impl MoveIntoContextTool {
    pub fn new(memory: Arc<AgentMemory>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl Tool for MoveIntoContextTool {
    fn id(&self) -> &str {
        "move_into_context"
    }

    fn description(&self) -> &str {
        "Move a memory block into the active context window. Use this to bring back \
         previously archived memories that are relevant to the current task."
    }

    fn name(&self) -> &str {
        self.id()
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "block_id".to_string(),
            json!({
                "type": "string",
                "description": "The ID of the memory block to move into context"
            }),
        );

        JsonSchema::object(properties).with_required(vec!["block_id".to_string()])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let block_id: String = params["block_id"]
            .as_str()
            .ok_or_else(|| {
                crate::error::Error::tool_execution("move_into_context", "Missing block_id")
            })?
            .to_string();

        let id: MemoryBlockId = serde_json::from_str(&format!("\"{}\"", block_id))?;

        self.memory.move_into_context(id).await?;

        Ok(ToolOutput::success_with_data(
            format!("Moved memory block {} into context", block_id),
            json!({"block_id": block_id}),
        ))
    }
}

/// Tool for listing all memory blocks
pub struct ListMemoryBlocksTool {
    memory: Arc<AgentMemory>,
}

impl ListMemoryBlocksTool {
    pub fn new(memory: Arc<AgentMemory>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl Tool for ListMemoryBlocksTool {
    fn id(&self) -> &str {
        "list_memory_blocks"
    }

    fn description(&self) -> &str {
        "List all your memory blocks with their IDs, labels, and whether they're in context. \
         Use this to see what memories you have available."
    }

    fn name(&self) -> &str {
        self.id()
    }

    fn input_schema(&self) -> JsonSchema {
        JsonSchema::object(HashMap::new())
    }

    async fn execute(&self, _params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let in_context = self.memory.in_context_blocks().await;
        let out_of_context = self.memory.out_of_context_blocks().await;
        let in_context_len = in_context.len();
        let out_of_context_len = out_of_context.len();

        let mut blocks_info = Vec::new();

        for block in in_context {
            blocks_info.push(json!({
                "id": block.id.to_string(),
                "label": block.label,
                "description": block.description,
                "in_context": true,
                "size": block.size(),
                "value_preview": if block.value.len() > 100 {
                    format!("{}...", &block.value[..100])
                } else {
                    block.value.clone()
                }
            }));
        }

        for block in out_of_context {
            blocks_info.push(json!({
                "id": block.id.to_string(),
                "label": block.label,
                "description": block.description,
                "in_context": false,
                "size": block.size(),
                "value_preview": if block.value.len() > 100 {
                    format!("{}...", &block.value[..100])
                } else {
                    block.value.clone()
                }
            }));
        }

        let context_size = self.memory.context_size().await;
        let max_size = self.memory.config.max_context_size;

        Ok(ToolOutput::success_with_data(
            format!(
                "Found {} memory blocks ({} in context, {} archived)\nContext usage: {}/{}",
                blocks_info.len(),
                in_context_len,
                out_of_context_len,
                context_size,
                max_size
            ),
            json!({
                "blocks": blocks_info,
                "context_size": context_size,
                "max_context_size": max_size
            }),
        ))
    }
}

/// Tool for searching message history
pub struct SearchMessagesTool {
    memory: Arc<AgentMemory>,
}

impl SearchMessagesTool {
    pub fn new(memory: Arc<AgentMemory>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl Tool for SearchMessagesTool {
    fn id(&self) -> &str {
        "search_messages"
    }

    fn description(&self) -> &str {
        "Search your perpetual message history for specific content. Use this to recall \
         past conversations or information you've encountered before."
    }

    fn name(&self) -> &str {
        self.id()
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "query".to_string(),
            json!({
                "type": "string",
                "description": "The text to search for in message history"
            }),
        );

        JsonSchema::object(properties).with_required(vec!["query".to_string()])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| crate::error::Error::tool_execution("search_messages", "Missing query"))?;

        let results = self.memory.search_messages(query).await;

        let messages: Vec<Value> = results
            .iter()
            .map(|m| {
                json!({
                    "timestamp": m.timestamp.to_rfc3339(),
                    "role": m.role,
                    "content_preview": if m.content.len() > 200 {
                        format!("{}...", &m.content[..200])
                    } else {
                        m.content.clone()
                    }
                })
            })
            .collect();

        Ok(ToolOutput::success_with_data(
            format!("Found {} messages matching '{}'", results.len(), query),
            json!({
                "query": query,
                "results": messages,
                "count": results.len()
            }),
        ))
    }
}

/// Create all standard memory tools for an agent
pub fn create_memory_tools(memory: Arc<AgentMemory>) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(UpdateMemoryTool::new(memory.clone())),
        Arc::new(MoveOutOfContextTool::new(memory.clone())),
        Arc::new(MoveIntoContextTool::new(memory.clone())),
        Arc::new(ListMemoryBlocksTool::new(memory.clone())),
        Arc::new(SearchMessagesTool::new(memory)),
    ]
}
