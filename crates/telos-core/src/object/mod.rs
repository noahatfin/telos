pub mod agent_operation;
pub mod behavior_diff;
pub mod change_set;
pub mod code_binding;
pub mod constraint;
pub mod decision_record;
pub mod intent;
pub mod intent_stream;

use crate::error::CoreError;
use crate::hash::ObjectId;
use crate::serialize::{canonical_serialize, content_hash};
use serde::{Deserialize, Serialize};

pub use agent_operation::AgentOperation;
pub use behavior_diff::BehaviorDiff;
pub use change_set::ChangeSet;
pub use code_binding::CodeBinding;
pub use constraint::Constraint;
pub use decision_record::DecisionRecord;
pub use intent::Intent;
pub use intent_stream::IntentStreamSnapshot;

/// Type tags used in content-addressable hashing.
const TAG_INTENT: &str = "intent";
const TAG_BEHAVIOR_DIFF: &str = "behavior_diff";
const TAG_STREAM_SNAPSHOT: &str = "intent_stream_snapshot";
const TAG_DECISION_RECORD: &str = "decision_record";
const TAG_CONSTRAINT: &str = "constraint";
const TAG_CODE_BINDING: &str = "code_binding";
const TAG_AGENT_OPERATION: &str = "agent_operation";
const TAG_CHANGE_SET: &str = "change_set";

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
    #[serde(rename = "constraint")]
    Constraint(Constraint),
    #[serde(rename = "code_binding")]
    CodeBinding(CodeBinding),
    #[serde(rename = "agent_operation")]
    AgentOperation(AgentOperation),
    #[serde(rename = "change_set")]
    ChangeSet(ChangeSet),
}

impl TelosObject {
    /// Return the type tag string for this object.
    pub fn type_tag(&self) -> &'static str {
        match self {
            Self::Intent(_) => TAG_INTENT,
            Self::BehaviorDiff(_) => TAG_BEHAVIOR_DIFF,
            Self::IntentStreamSnapshot(_) => TAG_STREAM_SNAPSHOT,
            Self::DecisionRecord(_) => TAG_DECISION_RECORD,
            Self::Constraint(_) => TAG_CONSTRAINT,
            Self::CodeBinding(_) => TAG_CODE_BINDING,
            Self::AgentOperation(_) => TAG_AGENT_OPERATION,
            Self::ChangeSet(_) => TAG_CHANGE_SET,
        }
    }

    /// Compute the content-address (SHA-256) for this object.
    pub fn content_id(&self) -> Result<ObjectId, CoreError> {
        match self {
            Self::Intent(o) => content_hash(TAG_INTENT, o),
            Self::BehaviorDiff(o) => content_hash(TAG_BEHAVIOR_DIFF, o),
            Self::IntentStreamSnapshot(o) => content_hash(TAG_STREAM_SNAPSHOT, o),
            Self::DecisionRecord(o) => content_hash(TAG_DECISION_RECORD, o),
            Self::Constraint(o) => content_hash(TAG_CONSTRAINT, o),
            Self::CodeBinding(o) => content_hash(TAG_CODE_BINDING, o),
            Self::AgentOperation(o) => content_hash(TAG_AGENT_OPERATION, o),
            Self::ChangeSet(o) => content_hash(TAG_CHANGE_SET, o),
        }
    }

    /// Serialize to canonical bytes (`type_tag\0sorted_json`).
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, CoreError> {
        match self {
            Self::Intent(o) => canonical_serialize(TAG_INTENT, o),
            Self::BehaviorDiff(o) => canonical_serialize(TAG_BEHAVIOR_DIFF, o),
            Self::IntentStreamSnapshot(o) => canonical_serialize(TAG_STREAM_SNAPSHOT, o),
            Self::DecisionRecord(o) => canonical_serialize(TAG_DECISION_RECORD, o),
            Self::Constraint(o) => canonical_serialize(TAG_CONSTRAINT, o),
            Self::CodeBinding(o) => canonical_serialize(TAG_CODE_BINDING, o),
            Self::AgentOperation(o) => canonical_serialize(TAG_AGENT_OPERATION, o),
            Self::ChangeSet(o) => canonical_serialize(TAG_CHANGE_SET, o),
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
            TAG_CONSTRAINT => Ok(Self::Constraint(serde_json::from_slice(json_bytes)?)),
            TAG_CODE_BINDING => Ok(Self::CodeBinding(serde_json::from_slice(json_bytes)?)),
            TAG_AGENT_OPERATION => {
                Ok(Self::AgentOperation(serde_json::from_slice(json_bytes)?))
            }
            TAG_CHANGE_SET => Ok(Self::ChangeSet(serde_json::from_slice(json_bytes)?)),
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

    #[test]
    fn round_trip_constraint() {
        let c = constraint::Constraint {
            author: Author {
                name: "Alice".into(),
                email: "alice@example.com".into(),
            },
            timestamp: Utc::now(),
            statement: "Must validate email format".into(),
            severity: constraint::ConstraintSeverity::Must,
            status: constraint::ConstraintStatus::Active,
            source_intent: ObjectId::hash(b"intent1"),
            superseded_by: None,
            deprecation_reason: None,
            scope: vec![],
            impacts: vec!["auth".into()],
            metadata: HashMap::new(),
        };
        let obj = TelosObject::Constraint(c.clone());
        let bytes = obj.canonical_bytes().unwrap();
        let restored = TelosObject::from_canonical_bytes(&bytes).unwrap();
        if let TelosObject::Constraint(restored_c) = restored {
            assert_eq!(restored_c, c);
        } else {
            panic!("expected Constraint variant");
        }
    }

    #[test]
    fn round_trip_code_binding() {
        let cb = code_binding::CodeBinding {
            path: "src/auth/mod.rs".into(),
            symbol: Some("validate_email".into()),
            span: Some((10, 25)),
            binding_type: code_binding::BindingType::Function,
            resolution: code_binding::BindingResolution::Resolved,
            bound_object: ObjectId::hash(b"constraint1"),
            metadata: HashMap::new(),
        };
        let obj = TelosObject::CodeBinding(cb.clone());
        let bytes = obj.canonical_bytes().unwrap();
        let restored = TelosObject::from_canonical_bytes(&bytes).unwrap();
        if let TelosObject::CodeBinding(restored_cb) = restored {
            assert_eq!(restored_cb, cb);
        } else {
            panic!("expected CodeBinding variant");
        }
    }

    #[test]
    fn round_trip_agent_operation() {
        let ao = agent_operation::AgentOperation {
            agent_id: "claude-review".into(),
            session_id: "sess-001".into(),
            timestamp: Utc::now(),
            operation: agent_operation::OperationType::Review,
            result: agent_operation::OperationResult::Success,
            summary: "Reviewed auth module".into(),
            context_refs: vec![ObjectId::hash(b"intent1")],
            files_touched: vec!["src/auth/mod.rs".into()],
            parent_op: None,
            metadata: HashMap::new(),
        };
        let obj = TelosObject::AgentOperation(ao.clone());
        let bytes = obj.canonical_bytes().unwrap();
        let restored = TelosObject::from_canonical_bytes(&bytes).unwrap();
        if let TelosObject::AgentOperation(restored_ao) = restored {
            assert_eq!(restored_ao, ao);
        } else {
            panic!("expected AgentOperation variant");
        }
    }

    #[test]
    fn round_trip_change_set() {
        let cs = change_set::ChangeSet {
            author: Author {
                name: "Alice".into(),
                email: "alice@example.com".into(),
            },
            timestamp: Utc::now(),
            git_commit: "abc123def456".into(),
            parents: vec![],
            intents: vec![ObjectId::hash(b"intent1")],
            constraints: vec![ObjectId::hash(b"constraint1")],
            decisions: vec![],
            code_bindings: vec![ObjectId::hash(b"binding1")],
            agent_operations: vec![],
            metadata: HashMap::new(),
        };
        let obj = TelosObject::ChangeSet(cs.clone());
        let bytes = obj.canonical_bytes().unwrap();
        let restored = TelosObject::from_canonical_bytes(&bytes).unwrap();
        if let TelosObject::ChangeSet(restored_cs) = restored {
            assert_eq!(restored_cs, cs);
        } else {
            panic!("expected ChangeSet variant");
        }
    }
}
