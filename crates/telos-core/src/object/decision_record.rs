use crate::hash::ObjectId;
use crate::object::intent::Author;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An alternative that was considered but not chosen.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Alternative {
    pub description: String,
    pub rejection_reason: String,
}

/// A structured human decision record — replaces PR comments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionRecord {
    pub intent_id: ObjectId,
    pub author: Author,
    pub timestamp: DateTime<Utc>,
    pub question: String,
    pub decision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<Alternative>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}
