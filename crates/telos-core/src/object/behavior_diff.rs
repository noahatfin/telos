use crate::hash::ObjectId;
use serde::{Deserialize, Serialize};

/// A single behavior change entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorChange {
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    pub after: String,
}

/// Impact radius analysis for a behavior diff.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImpactRadius {
    pub direct: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub indirect: Vec<String>,
}

/// Verification result (placeholder for Phase 2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Verification {
    pub status: VerificationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Pending,
    Passed,
    Failed,
}

/// Describes how system behavior changes — replaces code diff.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorDiff {
    pub intent_id: ObjectId,
    pub changes: Vec<BehaviorChange>,
    pub impact: ImpactRadius,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<Verification>,
}
