use crate::hash::ObjectId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single clause in a behavior specification (GIVEN/WHEN/THEN).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorClause {
    pub given: String,
    pub when: String,
    pub then: String,
}

/// Author identity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Author {
    pub name: String,
    pub email: String,
}

/// An immutable intent declaration — the fundamental unit in Telos (replaces Git commit).
///
/// `parents` forms a DAG: empty = root, one = linear, multiple = merge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Intent {
    pub author: Author,
    pub timestamp: DateTime<Utc>,
    pub statement: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behavior_spec: Vec<BehaviorClause>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parents: Vec<ObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub impacts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub behavior_diff: Option<ObjectId>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}
