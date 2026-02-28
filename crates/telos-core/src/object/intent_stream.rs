use crate::hash::ObjectId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Mutable reference to a stream — stored under `refs/streams/<name>`.
///
/// This is analogous to a Git branch pointer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentStreamRef {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tip: Option<ObjectId>,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Immutable snapshot of a stream — stored in the object database.
///
/// Created when we need to record the state of a stream at a point in time
/// (e.g., before a merge or for archival).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentStreamSnapshot {
    pub name: String,
    pub tip: ObjectId,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_stream: Option<String>,
}

/// Conflict information (placeholder for future conflict detection).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StreamConflict {
    pub stream_a: String,
    pub stream_b: String,
    pub conflicting_intents: Vec<ObjectId>,
    pub description: String,
}
