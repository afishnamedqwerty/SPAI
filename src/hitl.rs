//! Human-in-the-Loop approval workflows

use crate::types::{AgentId, ApprovalId, UserId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Unique request identifier
    pub id: ApprovalId,
    /// Agent requesting approval
    pub agent_id: AgentId,
    /// Type of action requiring approval
    pub action_type: ActionType,
    /// Description of the action
    pub description: String,
    /// Detailed context for reviewer
    pub context: ApprovalContext,
    /// Urgency level
    pub priority: Priority,
    /// Deadline for approval (None = no deadline)
    pub deadline: Option<DateTime<Utc>>,
    /// Suggested approvers
    pub suggested_approvers: Vec<UserId>,
}

/// Type of action requiring approval
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    /// Tool execution
    ToolExecution,
    /// Agent handoff
    Handoff,
    /// Final output
    OutputDelivery,
    /// Custom action type
    Custom(String),
}

/// Context for approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalContext {
    /// Additional context data
    pub data: HashMap<String, serde_json::Value>,
}

/// Priority level for approval
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    /// Low priority
    Low,
    /// Medium priority
    Medium,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Approval decision
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// Approve and continue execution
    Approved {
        /// Approver user ID
        approver: UserId,
        /// Optional notes
        notes: Option<String>,
    },
    /// Reject and halt execution
    Rejected {
        /// Approver user ID
        approver: UserId,
        /// Rejection reason
        reason: String,
    },
    /// Request modification before proceeding
    ModificationRequired {
        /// Approver user ID
        approver: UserId,
        /// Modification instructions
        instructions: String,
    },
    /// Escalate to higher authority
    Escalated {
        /// Target user ID for escalation
        target: UserId,
        /// Escalation reason
        reason: String,
    },
    /// Auto-approved due to timeout
    AutoApproved {
        /// Auto-approval reason
        reason: String,
    },
}

/// Approval status
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Pending approval
    Pending,
    /// Approved
    Approved,
    /// Rejected
    Rejected,
    /// Escalated
    Escalated,
    /// Expired
    Expired,
}

/// Approval handler trait
#[async_trait]
pub trait ApprovalHandler: Send + Sync {
    /// Request human approval
    async fn request_approval(
        &self,
        request: ApprovalRequest,
    ) -> crate::error::Result<ApprovalDecision>;

    /// Check status of pending approval
    async fn check_status(&self, id: ApprovalId) -> crate::error::Result<ApprovalStatus>;

    /// Cancel pending approval request
    async fn cancel(&self, id: ApprovalId) -> crate::error::Result<()>;
}
