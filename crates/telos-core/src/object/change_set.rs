use crate::hash::ObjectId;
use crate::object::intent::Author;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The bridge between Git and Telos — links a Git commit to its reasoning chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangeSet {
    pub author: Author,
    pub timestamp: DateTime<Utc>,
    /// The Git commit SHA this ChangeSet is linked to.
    pub git_commit: String,
    /// Parent changeset(s) — mirrors the Git commit DAG.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parents: Vec<ObjectId>,
    /// Intents that motivated this change.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intents: Vec<ObjectId>,
    /// Constraints created or referenced by this change.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<ObjectId>,
    /// Decisions made as part of this change.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<ObjectId>,
    /// Code bindings established or updated.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub code_bindings: Vec<ObjectId>,
    /// Agent operations that contributed to this change.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agent_operations: Vec<ObjectId>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}
