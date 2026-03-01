use crate::error::StoreError;
use crate::lockfile::Lockfile;
use std::fs;
use std::path::PathBuf;
use telos_core::object::intent_stream::IntentStreamRef;

/// Manages HEAD and stream references on disk.
///
/// Layout:
/// - `HEAD`            — contains `"ref: refs/streams/<name>"`
/// - `refs/streams/`   — each file is JSON-serialized `IntentStreamRef`
pub struct RefStore {
    telos_dir: PathBuf,
}

impl RefStore {
    pub fn new(telos_dir: impl Into<PathBuf>) -> Self {
        Self {
            telos_dir: telos_dir.into(),
        }
    }

    /// Validate a stream name. Rejects path traversal, null bytes, empty names, and leading dots.
    fn validate_stream_name(name: &str) -> Result<(), StoreError> {
        if name.is_empty() {
            return Err(StoreError::InvalidStreamName(
                name.into(),
                "stream name cannot be empty".into(),
            ));
        }
        if name.contains('\0') {
            return Err(StoreError::InvalidStreamName(
                name.replace('\0', "\\0"),
                "stream name cannot contain null bytes".into(),
            ));
        }
        if name.starts_with('.') {
            return Err(StoreError::InvalidStreamName(
                name.into(),
                "stream name cannot start with '.'".into(),
            ));
        }
        if name.contains("..") {
            return Err(StoreError::InvalidStreamName(
                name.into(),
                "stream name cannot contain '..'".into(),
            ));
        }
        // Each segment between '/' must be non-empty and not start with '.'
        for segment in name.split('/') {
            if segment.is_empty() {
                return Err(StoreError::InvalidStreamName(
                    name.into(),
                    "stream name cannot have empty path segments".into(),
                ));
            }
            if segment.starts_with('.') {
                return Err(StoreError::InvalidStreamName(
                    name.into(),
                    "path segments cannot start with '.'".into(),
                ));
            }
        }
        Ok(())
    }

    fn head_path(&self) -> PathBuf {
        self.telos_dir.join("HEAD")
    }

    fn streams_dir(&self) -> PathBuf {
        self.telos_dir.join("refs").join("streams")
    }

    fn stream_path(&self, name: &str) -> PathBuf {
        self.streams_dir().join(name)
    }

    // --- HEAD ---

    /// Read the current stream name from HEAD.
    ///
    /// HEAD contains `"ref: refs/streams/<name>"`.
    pub fn read_head(&self) -> Result<String, StoreError> {
        let content = fs::read_to_string(self.head_path())
            .map_err(|_| StoreError::InvalidHead("HEAD file not found".into()))?;
        let content = content.trim();
        content
            .strip_prefix("ref: refs/streams/")
            .map(|s| s.to_string())
            .ok_or_else(|| StoreError::InvalidHead(content.to_string()))
    }

    /// Set HEAD to point to a stream.
    pub fn set_head(&self, stream_name: &str) -> Result<(), StoreError> {
        Self::validate_stream_name(stream_name)?;
        let content = format!("ref: refs/streams/{}\n", stream_name);
        let mut lock = Lockfile::acquire(self.head_path())?;
        lock.write_all(content.as_bytes())?;
        lock.commit()
    }

    // --- Streams ---

    /// Read a stream reference by name.
    pub fn read_stream(&self, name: &str) -> Result<IntentStreamRef, StoreError> {
        let path = self.stream_path(name);
        let data = fs::read_to_string(&path)
            .map_err(|_| StoreError::StreamNotFound(name.to_string()))?;
        Ok(serde_json::from_str(&data)?)
    }

    /// Write (create or update) a stream reference.
    pub fn write_stream(&self, stream: &IntentStreamRef) -> Result<(), StoreError> {
        Self::validate_stream_name(&stream.name)?;
        let path = self.stream_path(&stream.name);
        let json = serde_json::to_string_pretty(stream)?;
        let mut lock = Lockfile::acquire(&path)?;
        lock.write_all(json.as_bytes())?;
        lock.commit()
    }

    /// Create a new stream. Fails if it already exists.
    pub fn create_stream(&self, stream: &IntentStreamRef) -> Result<(), StoreError> {
        Self::validate_stream_name(&stream.name)?;
        let path = self.stream_path(&stream.name);
        if path.exists() {
            return Err(StoreError::StreamExists(stream.name.clone()));
        }
        self.write_stream(stream)
    }

    /// Delete a stream reference. Cannot delete the stream HEAD points to.
    pub fn delete_stream(&self, name: &str) -> Result<(), StoreError> {
        Self::validate_stream_name(name)?;
        let head = self.read_head()?;
        if head == name {
            return Err(StoreError::StreamNotFound(format!(
                "cannot delete current stream '{}'",
                name
            )));
        }
        let path = self.stream_path(name);
        if !path.exists() {
            return Err(StoreError::StreamNotFound(name.to_string()));
        }
        fs::remove_file(&path)?;
        // Clean up empty parent directories up to the streams root
        let streams_dir = self.streams_dir();
        let mut parent = path.parent().map(|p| p.to_path_buf());
        while let Some(dir) = parent {
            if dir == streams_dir {
                break;
            }
            if fs::read_dir(&dir).map(|mut d| d.next().is_none()).unwrap_or(true) {
                let _ = fs::remove_dir(&dir);
                parent = dir.parent().map(|p| p.to_path_buf());
            } else {
                break;
            }
        }
        Ok(())
    }

    /// List all stream names (supports hierarchical names like `feature/onboarding`).
    pub fn list_streams(&self) -> Result<Vec<String>, StoreError> {
        let dir = self.streams_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut names = Vec::new();
        Self::collect_streams(&dir, &dir, &mut names)?;
        names.sort();
        Ok(names)
    }

    /// Recursively collect stream files relative to the streams root.
    fn collect_streams(
        root: &std::path::Path,
        current: &std::path::Path,
        names: &mut Vec<String>,
    ) -> Result<(), StoreError> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                Self::collect_streams(root, &path, names)?;
            } else if let Ok(rel) = path.strip_prefix(root) {
                if let Some(name) = rel.to_str() {
                    names.push(name.to_string());
                }
            }
        }
        Ok(())
    }

    /// Resolve the current stream (what HEAD points to) and return its ref.
    pub fn current_stream(&self) -> Result<IntentStreamRef, StoreError> {
        let name = self.read_head()?;
        self.read_stream(&name)
    }

    /// Update the tip of the current stream.
    pub fn update_current_tip(
        &self,
        tip: telos_core::hash::ObjectId,
    ) -> Result<(), StoreError> {
        let name = self.read_head()?;
        let mut stream = self.read_stream(&name)?;
        stream.tip = Some(tip);
        self.write_stream(&stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn setup() -> (tempfile::TempDir, RefStore) {
        let dir = tempfile::tempdir().unwrap();
        let telos_dir = dir.path().join(".telos");
        fs::create_dir_all(telos_dir.join("refs").join("streams")).unwrap();
        let store = RefStore::new(&telos_dir);
        store.set_head("main").unwrap();
        (dir, store)
    }

    #[test]
    fn head_round_trip() {
        let (_dir, store) = setup();
        assert_eq!(store.read_head().unwrap(), "main");
        store.set_head("feature").unwrap();
        assert_eq!(store.read_head().unwrap(), "feature");
    }

    #[test]
    fn stream_create_and_read() {
        let (_dir, store) = setup();
        let stream = IntentStreamRef {
            name: "main".into(),
            tip: None,
            created_at: Utc::now(),
            description: None,
        };
        store.create_stream(&stream).unwrap();
        let read = store.read_stream("main").unwrap();
        assert_eq!(read.name, "main");
        assert!(read.tip.is_none());
    }

    #[test]
    fn stream_create_duplicate_fails() {
        let (_dir, store) = setup();
        let stream = IntentStreamRef {
            name: "main".into(),
            tip: None,
            created_at: Utc::now(),
            description: None,
        };
        store.create_stream(&stream).unwrap();
        assert!(store.create_stream(&stream).is_err());
    }

    #[test]
    fn stream_name_rejects_path_traversal() {
        let (_dir, store) = setup();
        let now = Utc::now();
        let bad_names = vec![
            "../../etc/passwd",
            "foo/../../bar",
            "../escape",
            "foo/../bar",
        ];
        for name in bad_names {
            let stream = IntentStreamRef {
                name: name.into(),
                tip: None,
                created_at: now,
                description: None,
            };
            let result = store.create_stream(&stream);
            assert!(result.is_err(), "should reject stream name: {}", name);
        }
    }

    #[test]
    fn stream_name_rejects_dangerous_chars() {
        let (_dir, store) = setup();
        let now = Utc::now();
        let bad_names = vec![".hidden", "\0evil", "", "has\0null"];
        for name in bad_names {
            let stream = IntentStreamRef {
                name: name.into(),
                tip: None,
                created_at: now,
                description: None,
            };
            let result = store.create_stream(&stream);
            assert!(result.is_err(), "should reject stream name: {:?}", name);
        }
    }

    #[test]
    fn stream_name_allows_valid_hierarchical() {
        let (_dir, store) = setup();
        let now = Utc::now();
        let good_names = vec!["feature-auth", "feature/onboarding", "release/v2"];
        for name in good_names {
            let stream = IntentStreamRef {
                name: name.into(),
                tip: None,
                created_at: now,
                description: None,
            };
            let result = store.create_stream(&stream);
            assert!(result.is_ok(), "should allow stream name: {}", name);
        }
    }

    #[test]
    fn list_streams() {
        let (_dir, store) = setup();
        let now = Utc::now();
        for name in ["alpha", "beta", "main"] {
            store
                .create_stream(&IntentStreamRef {
                    name: name.into(),
                    tip: None,
                    created_at: now,
                    description: None,
                })
                .unwrap();
        }
        let names = store.list_streams().unwrap();
        assert_eq!(names, vec!["alpha", "beta", "main"]);
    }
}
