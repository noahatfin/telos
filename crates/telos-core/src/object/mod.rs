pub mod behavior_diff;
pub mod decision_record;
pub mod intent;
pub mod intent_stream;

use crate::error::CoreError;
use crate::hash::ObjectId;
use crate::serialize::{canonical_serialize, content_hash};
use serde::{Deserialize, Serialize};

pub use behavior_diff::BehaviorDiff;
pub use decision_record::DecisionRecord;
pub use intent::Intent;
pub use intent_stream::IntentStreamSnapshot;

/// Type tags used in content-addressable hashing.
const TAG_INTENT: &str = "intent";
const TAG_BEHAVIOR_DIFF: &str = "behavior_diff";
const TAG_STREAM_SNAPSHOT: &str = "intent_stream_snapshot";
const TAG_DECISION_RECORD: &str = "decision_record";

/// A storable Telos object — the union of all content-addressed types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TelosObject {
    #[serde(rename = "intent")]
    Intent(Intent),
    #[serde(rename = "behavior_diff")]
    BehaviorDiff(BehaviorDiff),
    #[serde(rename = "intent_stream_snapshot")]
    IntentStreamSnapshot(IntentStreamSnapshot),
    #[serde(rename = "decision_record")]
    DecisionRecord(DecisionRecord),
}

impl TelosObject {
    /// Return the type tag string for this object.
    pub fn type_tag(&self) -> &'static str {
        match self {
            Self::Intent(_) => TAG_INTENT,
            Self::BehaviorDiff(_) => TAG_BEHAVIOR_DIFF,
            Self::IntentStreamSnapshot(_) => TAG_STREAM_SNAPSHOT,
            Self::DecisionRecord(_) => TAG_DECISION_RECORD,
        }
    }

    /// Compute the content-address (SHA-256) for this object.
    pub fn content_id(&self) -> Result<ObjectId, CoreError> {
        match self {
            Self::Intent(o) => content_hash(TAG_INTENT, o),
            Self::BehaviorDiff(o) => content_hash(TAG_BEHAVIOR_DIFF, o),
            Self::IntentStreamSnapshot(o) => content_hash(TAG_STREAM_SNAPSHOT, o),
            Self::DecisionRecord(o) => content_hash(TAG_DECISION_RECORD, o),
        }
    }

    /// Serialize to canonical bytes (`type_tag\0sorted_json`).
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CoreError> {
        match self {
            Self::Intent(o) => canonical_serialize(TAG_INTENT, o),
            Self::BehaviorDiff(o) => canonical_serialize(TAG_BEHAVIOR_DIFF, o),
            Self::IntentStreamSnapshot(o) => canonical_serialize(TAG_STREAM_SNAPSHOT, o),
            Self::DecisionRecord(o) => canonical_serialize(TAG_DECISION_RECORD, o),
        }
    }

    /// Deserialize from canonical bytes (`type_tag\0json`).
    pub fn from_canonical_bytes(data: &[u8]) -> Result<Self, CoreError> {
        let null_pos = data
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| CoreError::UnknownTypeTag("missing null separator".into()))?;
        let tag = std::str::from_utf8(&data[..null_pos])
            .map_err(|e| CoreError::UnknownTypeTag(e.to_string()))?;
        let json_bytes = &data[null_pos + 1..];
        match tag {
            TAG_INTENT => Ok(Self::Intent(serde_json::from_slice(json_bytes)?)),
            TAG_BEHAVIOR_DIFF => Ok(Self::BehaviorDiff(serde_json::from_slice(json_bytes)?)),
            TAG_STREAM_SNAPSHOT => {
                Ok(Self::IntentStreamSnapshot(serde_json::from_slice(json_bytes)?))
            }
            TAG_DECISION_RECORD => {
                Ok(Self::DecisionRecord(serde_json::from_slice(json_bytes)?))
            }
            _ => Err(CoreError::UnknownTypeTag(tag.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::intent::{Author, BehaviorClause};
    use chrono::Utc;
    use std::collections::HashMap;

    fn sample_intent() -> Intent {
        Intent {
            author: Author {
                name: "Alice".into(),
                email: "alice@example.com".into(),
            },
            timestamp: Utc::now(),
            statement: "Add user registration flow".into(),
            constraints: vec!["Must validate email".into()],
            behavior_spec: vec![BehaviorClause {
                given: "a new user".into(),
                when: "they submit the registration form".into(),
                then: "an account is created".into(),
            }],
            parents: vec![],
            impacts: vec!["user-registration".into()],
            behavior_diff: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn round_trip_intent() {
        let intent = sample_intent();
        let obj = TelosObject::Intent(intent.clone());
        let bytes = obj.canonical_bytes().unwrap();
        let restored = TelosObject::from_canonical_bytes(&bytes).unwrap();
        if let TelosObject::Intent(restored_intent) = restored {
            assert_eq!(restored_intent, intent);
        } else {
            panic!("expected Intent variant");
        }
    }

    #[test]
    fn content_id_deterministic() {
        let intent = sample_intent();
        let obj = TelosObject::Intent(intent);
        let id1 = obj.content_id().unwrap();
        let id2 = obj.content_id().unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn type_tag_correct() {
        assert_eq!(
            TelosObject::Intent(sample_intent()).type_tag(),
            "intent"
        );
    }

    #[test]
    fn round_trip_decision_record() {
        let dr = DecisionRecord {
            intent_id: ObjectId::hash(b"dummy"),
            author: Author {
                name: "Bob".into(),
                email: "bob@example.com".into(),
            },
            timestamp: Utc::now(),
            question: "Which auth method?".into(),
            decision: "Use JWT".into(),
            rationale: Some("Stateless and scalable".into()),
            alternatives: vec![decision_record::Alternative {
                description: "Session cookies".into(),
                rejection_reason: "Requires server state".into(),
            }],
            tags: vec!["auth".into()],
        };
        let obj = TelosObject::DecisionRecord(dr.clone());
        let bytes = obj.canonical_bytes().unwrap();
        let restored = TelosObject::from_canonical_bytes(&bytes).unwrap();
        if let TelosObject::DecisionRecord(restored_dr) = restored {
            assert_eq!(restored_dr, dr);
        } else {
            panic!("expected DecisionRecord variant");
        }
    }
}
