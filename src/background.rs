//! Background execution and resumable streaming for long-running agent tasks
//!
//! This module provides:
//! - Asynchronous agent execution with run IDs
//! - Resumable streaming with sequence IDs
//! - Cursor-based pagination for results
//! - Connection recovery and state management
//! - Background job tracking

use crate::agent::{Agent, AgentOutput};
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Unique identifier for a background run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(Uuid);

impl RunId {
    /// Create a new run ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Sequence ID for ordering events within a run
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SeqId(u64);

impl SeqId {
    /// Create a new sequence ID
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the next sequence ID
    pub fn next(&self) -> Self {
        Self(self.0 + 1)
    }

    /// Get the underlying value
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl Default for SeqId {
    fn default() -> Self {
        Self(0)
    }
}

impl std::fmt::Display for SeqId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a background run
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    /// Run is queued but not started
    Queued,
    /// Run is currently executing
    Running,
    /// Run completed successfully
    Completed,
    /// Run failed with error
    Failed { error: String },
    /// Run was cancelled
    Cancelled,
}

/// A single event in a run (streamed output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    /// Sequence ID for ordering
    pub seq_id: SeqId,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Event type
    pub event_type: RunEventType,

    /// Event data
    pub data: serde_json::Value,
}

/// Types of run events
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RunEventType {
    /// Agent started processing
    Started,

    /// Agent produced a thought
    Thought,

    /// Agent is executing a tool
    ToolCall,

    /// Tool execution completed
    ToolResult,

    /// Agent produced final output
    Output,

    /// Run completed
    Completed,

    /// Run failed
    Failed,

    /// Progress update
    Progress,
}

/// Metadata about a background run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetadata {
    /// Unique run ID
    pub run_id: RunId,

    /// Agent name
    pub agent_name: String,

    /// Input that started the run
    pub input: String,

    /// Current status
    pub status: RunStatus,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Start timestamp
    pub started_at: Option<DateTime<Utc>>,

    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,

    /// Total events generated
    pub total_events: usize,

    /// Last sequence ID
    pub last_seq_id: SeqId,

    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

/// A background run with all its state
struct BackgroundRun {
    /// Run metadata
    metadata: RunMetadata,

    /// All events (for replay/resume)
    events: Vec<RunEvent>,

    /// Optional handle to the background task
    task_handle: Option<tokio::task::JoinHandle<Result<AgentOutput>>>,
}

/// Manager for background runs
pub struct BackgroundExecutor {
    /// All active and completed runs
    runs: Arc<RwLock<HashMap<RunId, BackgroundRun>>>,
}

impl BackgroundExecutor {
    /// Create a new background executor
    pub fn new() -> Self {
        Self {
            runs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start an agent execution in the background
    pub async fn execute_async(
        &self,
        agent: Arc<Agent>,
        input: String,
    ) -> Result<RunId> {
        let run_id = RunId::new();

        let metadata = RunMetadata {
            run_id,
            agent_name: agent.name.clone(),
            input: input.clone(),
            status: RunStatus::Queued,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            total_events: 0,
            last_seq_id: SeqId::default(),
            metadata: HashMap::new(),
        };

        // Spawn background task
        let runs = self.runs.clone();
        let handle = tokio::spawn(async move {
            // Update status to Running
            {
                let mut runs_lock = runs.write().await;
                if let Some(run) = runs_lock.get_mut(&run_id) {
                    run.metadata.status = RunStatus::Running;
                    run.metadata.started_at = Some(Utc::now());

                    // Add started event
                    let event = RunEvent {
                        seq_id: run.metadata.last_seq_id,
                        timestamp: Utc::now(),
                        event_type: RunEventType::Started,
                        data: serde_json::json!({
                            "agent": agent.name,
                            "input": input
                        }),
                    };
                    run.events.push(event);
                    run.metadata.last_seq_id = run.metadata.last_seq_id.next();
                    run.metadata.total_events += 1;
                }
            }

            // Execute the agent
            let result = agent.react_loop(&input).await;

            // Update status based on result
            {
                let mut runs_lock = runs.write().await;
                if let Some(run) = runs_lock.get_mut(&run_id) {
                    run.metadata.completed_at = Some(Utc::now());

                    match &result {
                        Ok(output) => {
                            run.metadata.status = RunStatus::Completed;

                            let tool_calls = output
                                .trace
                                .actions
                                .iter()
                                .filter(|&action| {
                                    matches!(action, crate::react::Action::ToolCall { .. })
                                })
                                .count();

                            // Add output event
                            let event = RunEvent {
                                seq_id: run.metadata.last_seq_id,
                                timestamp: Utc::now(),
                                event_type: RunEventType::Output,
                                data: serde_json::json!({
                                    "content": output.content,
                                    "tool_calls": tool_calls
                                }),
                            };
                            run.events.push(event);
                            run.metadata.last_seq_id = run.metadata.last_seq_id.next();
                            run.metadata.total_events += 1;

                            // Add completed event
                            let event = RunEvent {
                                seq_id: run.metadata.last_seq_id,
                                timestamp: Utc::now(),
                                event_type: RunEventType::Completed,
                                data: serde_json::json!({}),
                            };
                            run.events.push(event);
                            run.metadata.last_seq_id = run.metadata.last_seq_id.next();
                            run.metadata.total_events += 1;
                        }
                        Err(e) => {
                            run.metadata.status = RunStatus::Failed {
                                error: e.to_string(),
                            };

                            // Add failed event
                            let event = RunEvent {
                                seq_id: run.metadata.last_seq_id,
                                timestamp: Utc::now(),
                                event_type: RunEventType::Failed,
                                data: serde_json::json!({
                                    "error": e.to_string()
                                }),
                            };
                            run.events.push(event);
                            run.metadata.last_seq_id = run.metadata.last_seq_id.next();
                            run.metadata.total_events += 1;
                        }
                    }
                }
            }

            result
        });

        let run = BackgroundRun {
            metadata,
            events: Vec::new(),
            task_handle: Some(handle),
        };

        let mut runs = self.runs.write().await;
        runs.insert(run_id, run);

        Ok(run_id)
    }

    /// Get metadata for a run
    pub async fn get_run_metadata(&self, run_id: RunId) -> Result<RunMetadata> {
        let runs = self.runs.read().await;
        runs.get(&run_id)
            .map(|r| r.metadata.clone())
            .ok_or_else(|| Error::config(format!("Run {} not found", run_id)))
    }

    /// Stream events from a run, optionally starting from a specific sequence ID
    pub async fn stream_events(
        &self,
        run_id: RunId,
        starting_after: Option<SeqId>,
    ) -> Result<Vec<RunEvent>> {
        let runs = self.runs.read().await;

        let run = runs
            .get(&run_id)
            .ok_or_else(|| Error::config(format!("Run {} not found", run_id)))?;

        let events: Vec<RunEvent> = if let Some(after) = starting_after {
            run.events
                .iter()
                .filter(|e| e.seq_id > after)
                .cloned()
                .collect()
        } else {
            run.events.clone()
        };

        Ok(events)
    }

    /// Get events with cursor-based pagination
    pub async fn get_events_paginated(
        &self,
        run_id: RunId,
        cursor: Option<SeqId>,
        limit: usize,
    ) -> Result<PaginatedEvents> {
        let runs = self.runs.read().await;

        let run = runs
            .get(&run_id)
            .ok_or_else(|| Error::config(format!("Run {} not found", run_id)))?;

        let start_idx = if let Some(cursor_seq) = cursor {
            run.events
                .iter()
                .position(|e| e.seq_id > cursor_seq)
                .unwrap_or(run.events.len())
        } else {
            0
        };

        let end_idx = (start_idx + limit).min(run.events.len());
        let events = run.events[start_idx..end_idx].to_vec();

        let next_cursor = events.last().map(|e| e.seq_id);
        let has_more = end_idx < run.events.len();

        Ok(PaginatedEvents {
            events,
            next_cursor,
            has_more,
            total_events: run.metadata.total_events,
        })
    }

    /// Wait for a run to complete
    pub async fn wait_for_completion(&self, run_id: RunId) -> Result<AgentOutput> {
        // Get the task handle
        let handle = {
            let mut runs = self.runs.write().await;
            let run = runs
                .get_mut(&run_id)
                .ok_or_else(|| Error::config(format!("Run {} not found", run_id)))?;

            run.task_handle
                .take()
                .ok_or_else(|| Error::config("Run already completed or handle taken".to_string()))?
        };

        // Wait for completion
        let result = handle
            .await
            .map_err(|e| Error::config(format!("Failed to join task: {}", e)))??;

        Ok(result)
    }

    /// Cancel a running execution
    pub async fn cancel_run(&self, run_id: RunId) -> Result<()> {
        let mut runs = self.runs.write().await;

        let run = runs
            .get_mut(&run_id)
            .ok_or_else(|| Error::config(format!("Run {} not found", run_id)))?;

        if let Some(handle) = run.task_handle.take() {
            handle.abort();
            run.metadata.status = RunStatus::Cancelled;
            run.metadata.completed_at = Some(Utc::now());

            // Add cancelled event
            let event = RunEvent {
                seq_id: run.metadata.last_seq_id,
                timestamp: Utc::now(),
                event_type: RunEventType::Failed,
                data: serde_json::json!({
                    "error": "Cancelled by user"
                }),
            };
            run.events.push(event);
            run.metadata.last_seq_id = run.metadata.last_seq_id.next();
            run.metadata.total_events += 1;
        }

        Ok(())
    }

    /// List all runs
    pub async fn list_runs(&self) -> Vec<RunMetadata> {
        let runs = self.runs.read().await;
        runs.values().map(|r| r.metadata.clone()).collect()
    }

    /// Clean up completed runs older than the specified duration
    pub async fn cleanup_old_runs(&self, older_than: chrono::Duration) -> usize {
        let mut runs = self.runs.write().await;
        let cutoff = Utc::now() - older_than;

        let to_remove: Vec<RunId> = runs
            .iter()
            .filter(|(_, run)| {
                matches!(
                    run.metadata.status,
                    RunStatus::Completed | RunStatus::Failed { .. } | RunStatus::Cancelled
                ) && run
                    .metadata
                    .completed_at
                    .map(|t| t < cutoff)
                    .unwrap_or(false)
            })
            .map(|(id, _)| *id)
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            runs.remove(&id);
        }

        count
    }
}

impl Default for BackgroundExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Paginated result set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedEvents {
    /// Events in this page
    pub events: Vec<RunEvent>,

    /// Cursor for next page (None if no more pages)
    pub next_cursor: Option<SeqId>,

    /// Whether there are more events
    pub has_more: bool,

    /// Total number of events
    pub total_events: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentBuilder;
    use crate::llm_client::LlmClient;
    use crate::openrouter::CompletionRequest;
    use crate::types::AgentId;
    use async_trait::async_trait;

    // Mock client for testing
    struct MockClient;

    #[async_trait]
    impl LlmClient for MockClient {
        async fn complete(
            &self,
            _request: CompletionRequest,
        ) -> Result<crate::openrouter::CompletionResponse> {
            Ok(crate::openrouter::CompletionResponse {
                id: "test".to_string(),
                model: "test".to_string(),
                choices: vec![crate::openrouter::Choice {
                    message: crate::openrouter::Message {
                        role: "assistant".to_string(),
                        content: Some("Test response".to_string()),
                        tool_calls: None,
                    },
                    finish_reason: Some("stop".to_string()),
                    index: 0,
                }],
                usage: None,
            })
        }

        async fn stream(
            &self,
            _request: CompletionRequest,
        ) -> Result<crate::openrouter::CompletionStream> {
            Err(Error::config("Streaming not supported in mock".to_string()))
        }

        fn client_type(&self) -> &str {
            "mock"
        }

        fn endpoint(&self) -> &str {
            "http://localhost"
        }
    }

    #[tokio::test]
    async fn test_background_execution() {
        let executor = BackgroundExecutor::new();

        let agent = Arc::new(
            AgentBuilder::new()
                .name("Test Agent")
                .model("test")
                .client(Arc::new(MockClient))
                .build()
                .unwrap(),
        );

        let run_id = executor
            .execute_async(agent, "Test input".to_string())
            .await
            .unwrap();

        // Wait a bit for execution
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let metadata = executor.get_run_metadata(run_id).await.unwrap();
        assert_eq!(metadata.agent_name, "Test Agent");
    }

    #[tokio::test]
    async fn test_event_streaming() {
        let executor = BackgroundExecutor::new();

        let agent = Arc::new(
            AgentBuilder::new()
                .name("Test Agent")
                .model("test")
                .client(Arc::new(MockClient))
                .build()
                .unwrap(),
        );

        let run_id = executor
            .execute_async(agent, "Test".to_string())
            .await
            .unwrap();

        // Wait for completion
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let events = executor.stream_events(run_id, None).await.unwrap();
        assert!(!events.is_empty());
    }

    #[tokio::test]
    async fn test_cursor_pagination() {
        let executor = BackgroundExecutor::new();

        let agent = Arc::new(
            AgentBuilder::new()
                .name("Test Agent")
                .model("test")
                .client(Arc::new(MockClient))
                .build()
                .unwrap(),
        );

        let run_id = executor
            .execute_async(agent, "Test".to_string())
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let page1 = executor
            .get_events_paginated(run_id, None, 2)
            .await
            .unwrap();

        assert!(page1.events.len() <= 2);
    }
}
