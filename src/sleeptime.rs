//! Sleep-time agent support for background memory processing
//!
//! Sleep-time agents run in the background and share memory with the primary agent,
//! performing tasks like:
//! - Memory consolidation and summarization
//! - Context window optimization
//! - Automatic archival of old memories
//! - Pattern detection across conversation history

use crate::error::{Error, Result};
use crate::memory::{AgentMemory, MemoryBlock};
use crate::types::AgentId;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, RwLock};
use tokio::time;

/// Configuration for sleep-time agent behavior
#[derive(Debug, Clone)]
pub struct SleepTimeConfig {
    /// How often to run consolidation (default: 5 minutes)
    pub consolidation_interval: Duration,

    /// Minimum messages before consolidation triggers
    pub min_messages_for_consolidation: usize,

    /// Maximum context size before aggressive archival
    pub context_warning_threshold: usize,

    /// Enable automatic summarization
    pub enable_summarization: bool,

    /// Enable pattern detection
    pub enable_pattern_detection: bool,
}

impl Default for SleepTimeConfig {
    fn default() -> Self {
        Self {
            consolidation_interval: Duration::from_secs(300), // 5 minutes
            min_messages_for_consolidation: 20,
            context_warning_threshold: 6000, // 75% of default 8K context
            enable_summarization: true,
            enable_pattern_detection: true,
        }
    }
}

/// Sleep-time agent that processes memory in the background
pub struct SleepTimeAgent {
    /// ID of the primary agent this sleep-time agent serves
    primary_agent_id: AgentId,

    /// Shared memory with primary agent
    shared_memory: Arc<AgentMemory>,

    /// Configuration
    config: SleepTimeConfig,

    /// Flag to control the background task
    running: Arc<RwLock<bool>>,

    /// Shutdown signal for interrupting the interval tick
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,

    /// Optional handle to the background task
    task_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl SleepTimeAgent {
    /// Create a new sleep-time agent
    pub fn new(
        primary_agent_id: AgentId,
        shared_memory: Arc<AgentMemory>,
        config: SleepTimeConfig,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Self {
            primary_agent_id,
            shared_memory,
            config,
            running: Arc::new(RwLock::new(false)),
            shutdown_tx,
            shutdown_rx,
            task_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the background processing loop
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(Error::config(
                "Sleep-time agent is already running".to_string(),
            ));
        }

        *running = true;
        let _ = self.shutdown_tx.send(false);

        // Spawn background task
        let memory = self.shared_memory.clone();
        let config = self.config.clone();
        let running_flag = self.running.clone();
        let mut shutdown_rx = self.shutdown_rx.clone();
        let agent_id = self.primary_agent_id;

        let handle = tokio::spawn(async move {
            let mut interval = time::interval(config.consolidation_interval);

            loop {
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        // Check if we should stop
                        let should_run = *running_flag.read().await;
                        if !should_run {
                            break;
                        }

                        // Perform consolidation
                        if let Err(e) = Self::consolidate_memory(&memory, &config, agent_id).await {
                            eprintln!("Sleep-time agent error during consolidation: {}", e);
                        }
                    }
                }
            }
        });

        let mut task_handle = self.task_handle.write().await;
        *task_handle = Some(handle);

        Ok(())
    }

    /// Stop the background processing loop
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        let _ = self.shutdown_tx.send(true);

        // Wait for task to complete
        let mut task_handle = self.task_handle.write().await;
        if let Some(handle) = task_handle.take() {
            handle.await.map_err(|e| {
                Error::config(format!("Failed to stop sleep-time agent: {}", e))
            })?;
        }

        Ok(())
    }

    /// Perform memory consolidation
    async fn consolidate_memory(
        memory: &Arc<AgentMemory>,
        config: &SleepTimeConfig,
        _agent_id: AgentId,
    ) -> Result<()> {
        // Check message count
        let recent_messages = memory.get_recent_messages(1000).await;
        if recent_messages.len() < config.min_messages_for_consolidation {
            return Ok(());
        }

        // Check context size
        let context_size = memory.context_size().await;
        let needs_archival = context_size > config.context_warning_threshold;

        if needs_archival {
            Self::perform_archival(memory).await?;
        }

        if config.enable_summarization {
            Self::perform_summarization(memory, &recent_messages).await?;
        }

        if config.enable_pattern_detection {
            Self::detect_patterns(memory, &recent_messages).await?;
        }

        Ok(())
    }

    /// Archive old or low-priority memory blocks
    async fn perform_archival(memory: &Arc<AgentMemory>) -> Result<()> {
        let in_context = memory.in_context_blocks().await;

        // Simple heuristic: archive blocks that haven't been updated recently
        let now = chrono::Utc::now();
        for block in in_context {
            let age = now - block.updated_at;

            // Archive blocks older than 1 hour that aren't persona/organization
            if age > chrono::Duration::hours(1)
                && block.label != "persona"
                && block.label != "organization"
                && block.label != "system"
            {
                memory.move_out_of_context(block.id).await?;
            }
        }

        Ok(())
    }

    /// Summarize old messages and create summary memory block
    async fn perform_summarization(
        memory: &Arc<AgentMemory>,
        messages: &[crate::memory::MessageEntry],
    ) -> Result<()> {
        if messages.len() < 50 {
            return Ok(());
        }

        // Take oldest 50 messages for summarization
        let to_summarize = &messages[..50.min(messages.len())];

        // Create a simple summary
        let mut summary = String::new();
        summary.push_str("Summary of recent conversation:\n");

        // Count message types
        let user_msgs = to_summarize.iter().filter(|m| m.role == "user").count();
        let assistant_msgs = to_summarize
            .iter()
            .filter(|m| m.role == "assistant")
            .count();

        summary.push_str(&format!(
            "- {} user messages, {} assistant responses\n",
            user_msgs, assistant_msgs
        ));

        // Extract key topics (simple keyword extraction)
        let mut keywords: Vec<String> = Vec::new();
        for msg in to_summarize {
            let words: Vec<&str> = msg.content.split_whitespace().collect();
            for word in words {
                if word.len() > 5
                    && !keywords.contains(&word.to_lowercase())
                    && keywords.len() < 10
                {
                    keywords.push(word.to_lowercase());
                }
            }
        }

        summary.push_str(&format!("- Key topics: {}\n", keywords.join(", ")));

        // Check if we already have a conversation_summary block
        let blocks = memory.in_context_blocks().await;
        let summary_block = blocks.iter().find(|b| b.label == "conversation_summary");

        if let Some(existing) = summary_block {
            // Append to existing summary
            let mut updated = existing.clone();
            updated.append(summary)?;
            memory.update_block(existing.id, updated.value).await?;
        } else {
            // Create new summary block
            let block = MemoryBlock::with_description(
                "conversation_summary",
                "Automatically generated summary of conversation history",
                summary,
            );
            memory.add_block(block).await?;
        }

        Ok(())
    }

    /// Detect patterns in conversation history
    async fn detect_patterns(
        memory: &Arc<AgentMemory>,
        messages: &[crate::memory::MessageEntry],
    ) -> Result<()> {
        if messages.is_empty() {
            return Ok(());
        }

        // Simple pattern detection: repeated questions
        let mut question_patterns: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for msg in messages.iter().filter(|m| m.role == "user") {
            // Extract first 50 chars as pattern key
            let pattern_key = msg
                .content
                .chars()
                .take(50)
                .collect::<String>()
                .to_lowercase();

            *question_patterns.entry(pattern_key).or_insert(0) += 1;
        }

        // Find repeated patterns (asked 3+ times)
        let repeated: Vec<String> = question_patterns
            .into_iter()
            .filter_map(|(pattern, count)| if count >= 3 { Some(pattern) } else { None })
            .collect();

        if !repeated.is_empty() {
            // Store detected patterns in a memory block
            let patterns_text = format!(
                "Detected repeated questions:\n{}",
                repeated
                    .iter()
                    .map(|p| format!("- {}", p))
                    .collect::<Vec<_>>()
                    .join("\n")
            );

            let blocks = memory.in_context_blocks().await;
            let pattern_block = blocks.iter().find(|b| b.label == "detected_patterns");

            if let Some(existing) = pattern_block {
                memory.update_block(existing.id, patterns_text).await?;
            } else {
                let block = MemoryBlock::with_description(
                    "detected_patterns",
                    "Patterns detected in conversation by sleep-time agent",
                    patterns_text,
                );
                memory.add_block(block).await?;
            }
        }

        Ok(())
    }
}

impl Drop for SleepTimeAgent {
    fn drop(&mut self) {
        // Best effort stop on drop (can't await in Drop)
        let running = self.running.clone();
        let shutdown_tx = self.shutdown_tx.clone();
        tokio::spawn(async move {
            let mut r = running.write().await;
            *r = false;
            let _ = shutdown_tx.send(true);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryConfig;
    use crate::types::AgentId;

    #[tokio::test]
    async fn test_sleeptime_agent_start_stop() {
        let agent_id = AgentId::new();
        let memory = Arc::new(AgentMemory::new(agent_id, MemoryConfig::default()));
        let config = SleepTimeConfig {
            consolidation_interval: Duration::from_millis(100),
            ..Default::default()
        };

        let sleeptime = SleepTimeAgent::new(agent_id, memory, config);

        sleeptime.start().await.unwrap();
        assert!(*sleeptime.running.read().await);

        tokio::time::sleep(Duration::from_millis(50)).await;

        sleeptime.stop().await.unwrap();
        assert!(!*sleeptime.running.read().await);
    }

    #[tokio::test]
    async fn test_sleeptime_agent_stop_interrupts_interval() {
        let agent_id = AgentId::new();
        let memory = Arc::new(AgentMemory::new(agent_id, MemoryConfig::default()));
        let config = SleepTimeConfig {
            consolidation_interval: Duration::from_secs(60),
            ..Default::default()
        };

        let sleeptime = SleepTimeAgent::new(agent_id, memory, config);
        sleeptime.start().await.unwrap();

        tokio::time::timeout(Duration::from_millis(250), sleeptime.stop())
            .await
            .expect("stop() should not block on interval tick")
            .unwrap();
    }

    #[tokio::test]
    async fn test_archival() {
        let agent_id = AgentId::new();
        let memory = Arc::new(AgentMemory::new(agent_id, MemoryConfig::default()));

        // Add an old block
        let mut block = MemoryBlock::new("test_old", "old data");
        block.updated_at = chrono::Utc::now() - chrono::Duration::hours(2);
        memory.add_block(block).await.unwrap();

        // Perform archival
        SleepTimeAgent::perform_archival(&memory).await.unwrap();

        // Check it was moved out of context
        assert_eq!(memory.in_context_blocks().await.len(), 0);
        assert_eq!(memory.out_of_context_blocks().await.len(), 1);
    }
}
