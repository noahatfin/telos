use crate::hash::ObjectId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// What kind of code element is being bound.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BindingType {
    File,
    Function,
    Module,
    Api,
    Type,
}

/// Resolution state of a code binding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BindingResolution {
    Resolved,
    Unresolved,
    Unchecked,
}

/// Links a Telos object to a specific code location.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeBinding {
    /// File path relative to repository root.
    pub path: String,
    /// Symbol name (function, type, module name). None for file-level bindings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Line range [start, end] inclusive. None for whole-file or symbol-level bindings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<(u32, u32)>,
    pub binding_type: BindingType,
    pub resolution: BindingResolution,
    /// The Telos object this binding belongs to.
    pub bound_object: ObjectId,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
}
