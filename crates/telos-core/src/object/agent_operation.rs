use crate::hash::ObjectId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// What the agent did.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    Review,
    Generate,
    Decide,
    Query,
    Violation,
    Custom(String),
}

/// Outcome of the operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OperationResult {
    Success,
    Warning(String),
    Failure(String),
    Skipped,
}

/// A content-addressable log entry for an AI agent's operation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentOperation {
    /// Caller-controlled agent identifier.
    pub agent_id: String,
    /// Caller-controlled session identifier.
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub operation: OperationType,
    pub result: OperationResult,
    /// Human-readable summary of what happened.
    pub summary: String,
    /// Telos objects the agent consulted during this operation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_refs: Vec<ObjectId>,
    /// Files the agent read or modified.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files_touched: Vec<String>,
    /// Parent operation (for multi-step workflows).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_op: Option<ObjectId>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}
