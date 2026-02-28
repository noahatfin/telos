use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid object id: {0}")]
    InvalidObjectId(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("unknown object type tag: {0}")]
    UnknownTypeTag(String),
}
