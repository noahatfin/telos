use crate::hash::ObjectId;
use crate::object::intent::Author;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintStatus {
    Active,
    Superseded,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintSeverity {
    Must,
    Should,
    Prefer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Constraint {
    pub author: Author,
    pub timestamp: DateTime<Utc>,
    pub statement: String,
    pub severity: ConstraintSeverity,
    pub status: ConstraintStatus,
    pub source_intent: ObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<ObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecation_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<ObjectId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub impacts: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}
