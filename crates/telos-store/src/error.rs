use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("core error: {0}")]
    Core(#[from] telos_core::error::CoreError),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("object not found: {0}")]
    ObjectNotFound(String),

    #[error("ambiguous object prefix '{prefix}': matches {count} objects")]
    AmbiguousPrefix { prefix: String, count: usize },

    #[error("repository not found (searched upward from {0})")]
    RepositoryNotFound(String),

    #[error("repository already exists at {0}")]
    RepositoryExists(String),

    #[error("stream not found: {0}")]
    StreamNotFound(String),

    #[error("stream already exists: {0}")]
    StreamExists(String),

    #[error("lock file conflict: {0}")]
    LockConflict(String),

    #[error("invalid HEAD: {0}")]
    InvalidHead(String),

    #[error("no current stream (HEAD is detached or invalid)")]
    NoCurrentStream,

    #[error("invalid stream name '{0}': {1}")]
    InvalidStreamName(String, String),
}
