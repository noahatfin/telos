use crate::error::StoreError;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use telos_core::hash::ObjectId;
use telos_core::object::TelosObject;

/// Content-addressable object database.
///
/// Objects are stored as `objects/<2-char fan-out>/<remaining 62 chars>`.
/// Writes are atomic (temp file + rename).
pub struct ObjectDatabase {
    objects_dir: PathBuf,
}

impl ObjectDatabase {
    pub fn new(objects_dir: impl Into<PathBuf>) -> Self {
        Self {
            objects_dir: objects_dir.into(),
        }
    }

    /// Compute the file path for a given ObjectId.
    fn object_path(&self, id: &ObjectId) -> PathBuf {
        let (dir, file) = id.fan_out();
        self.objects_dir.join(dir).join(file)
    }

    /// Write an object to the store. Returns the ObjectId.
    ///
    /// If the object already exists (same hash), this is a no-op.
    pub fn write(&self, object: &TelosObject) -> Result<ObjectId, StoreError> {
        let bytes = object.canonical_bytes()?;
        let id = ObjectId::hash(&bytes);
        let path = self.object_path(&id);

        if path.exists() {
            return Ok(id); // idempotent
        }

        // Ensure fan-out directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Atomic write: temp file in same directory + rename
        let parent = path.parent().unwrap();
        let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
        tmp.write_all(&bytes)?;
        tmp.flush()?;
        tmp.persist(&path).map_err(|e| StoreError::Io(e.error))?;

        Ok(id)
    }

    /// Read an object by its exact ObjectId.
    pub fn read(&self, id: &ObjectId) -> Result<TelosObject, StoreError> {
        let path = self.object_path(id);
        let bytes = fs::read(&path)
            .map_err(|_| StoreError::ObjectNotFound(id.hex().to_string()))?;

        // Verify integrity: recompute hash and compare to expected ID
        let actual_id = ObjectId::hash(&bytes);
        if &actual_id != id {
            return Err(StoreError::IntegrityError {
                expected: id.hex().to_string(),
                actual: actual_id.hex().to_string(),
            });
        }

        Ok(TelosObject::from_canonical_bytes(&bytes)?)
    }

    /// Iterate over all objects stored in the database.
    pub fn iter_all(&self) -> Result<Vec<(ObjectId, TelosObject)>, StoreError> {
        let mut results = Vec::new();
        // Walk 00-ff fan-out directories
        let entries = match fs::read_dir(&self.objects_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(results),
            Err(e) => return Err(StoreError::Io(e)),
        };
        for fan_entry in entries {
            let fan_entry = fan_entry.map_err(StoreError::Io)?;
            let fan_name = fan_entry.file_name().to_string_lossy().to_string();
            if fan_name.len() != 2 {
                continue;
            }
            if !fan_entry.path().is_dir() {
                continue;
            }
            let sub_entries = fs::read_dir(fan_entry.path()).map_err(StoreError::Io)?;
            for obj_entry in sub_entries {
                let obj_entry = obj_entry.map_err(StoreError::Io)?;
                let obj_name = obj_entry.file_name().to_string_lossy().to_string();
                let hex = format!("{}{}", fan_name, obj_name);
                if let Ok(id) = ObjectId::parse(&hex) {
                    if let Ok(obj) = self.read(&id) {
                        results.push((id, obj));
                    }
                }
            }
        }
        Ok(results)
    }

    /// Check if an object exists.
    pub fn exists(&self, id: &ObjectId) -> bool {
        self.object_path(id).exists()
    }

    /// Resolve a hex prefix to a full ObjectId.
    ///
    /// Scans the fan-out directory for matching objects.
    pub fn resolve_prefix(&self, prefix: &str) -> Result<ObjectId, StoreError> {
        if prefix.len() < 4 {
            return Err(StoreError::AmbiguousPrefix {
                prefix: prefix.to_string(),
                count: 0,
            });
        }

        let fan_out = &prefix[..2];
        let rest_prefix = &prefix[2..];
        let fan_dir = self.objects_dir.join(fan_out);

        if !fan_dir.exists() {
            return Err(StoreError::ObjectNotFound(prefix.to_string()));
        }

        let mut matches = Vec::new();
        for entry in fs::read_dir(&fan_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(rest_prefix) {
                let full_hex = format!("{}{}", fan_out, name);
                matches.push(full_hex);
            }
        }

        match matches.len() {
            0 => Err(StoreError::ObjectNotFound(prefix.to_string())),
            1 => Ok(ObjectId::parse(&matches[0])?),
            n => Err(StoreError::AmbiguousPrefix {
                prefix: prefix.to_string(),
                count: n,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use telos_core::object::decision_record::DecisionRecord;
    use telos_core::object::intent::{Author, Intent};

    fn sample_intent() -> TelosObject {
        TelosObject::Intent(Intent {
            author: Author {
                name: "Test".into(),
                email: "test@test.com".into(),
            },
            timestamp: Utc::now(),
            statement: "Test intent".into(),
            constraints: vec![],
            behavior_spec: vec![],
            parents: vec![],
            impacts: vec![],
            behavior_diff: None,
            metadata: HashMap::new(),
        })
    }

    #[test]
    fn write_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let odb = ObjectDatabase::new(dir.path().join("objects"));
        let obj = sample_intent();

        let id = odb.write(&obj).unwrap();
        assert!(odb.exists(&id));

        let restored = odb.read(&id).unwrap();
        assert_eq!(restored, obj);
    }

    #[test]
    fn write_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let odb = ObjectDatabase::new(dir.path().join("objects"));
        let obj = sample_intent();

        let id1 = odb.write(&obj).unwrap();
        let id2 = odb.write(&obj).unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn read_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let odb = ObjectDatabase::new(dir.path().join("objects"));
        let id = ObjectId::hash(b"nonexistent");
        assert!(odb.read(&id).is_err());
    }

    #[test]
    fn read_detects_corrupted_object() {
        let dir = tempfile::tempdir().unwrap();
        let odb = ObjectDatabase::new(dir.path().join("objects"));
        let obj = sample_intent();
        let id = odb.write(&obj).unwrap();

        // Corrupt the file by appending garbage
        let path = odb.object_path(&id);
        let mut contents = std::fs::read(&path).unwrap();
        contents.extend_from_slice(b"CORRUPTED");
        std::fs::write(&path, &contents).unwrap();

        let result = odb.read(&id);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(err_str.contains("integrity"), "error should mention integrity: {}", err_str);
    }

    #[test]
    fn resolve_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let odb = ObjectDatabase::new(dir.path().join("objects"));
        let obj = sample_intent();
        let id = odb.write(&obj).unwrap();

        // Use first 8 chars as prefix
        let prefix = &id.hex()[..8];
        let resolved = odb.resolve_prefix(prefix).unwrap();
        assert_eq!(resolved, id);
    }

    #[test]
    fn iter_all_returns_all_objects() {
        let dir = tempfile::tempdir().unwrap();
        let odb = ObjectDatabase::new(dir.path().join("objects"));

        // Write a mix of object types
        let intent = TelosObject::Intent(Intent {
            author: Author {
                name: "Test".into(),
                email: "test@test.com".into(),
            },
            timestamp: Utc::now(),
            statement: "Test intent".into(),
            constraints: vec![],
            behavior_spec: vec![],
            parents: vec![],
            impacts: vec!["test".into()],
            behavior_diff: None,
            metadata: HashMap::new(),
        });
        let id1 = odb.write(&intent).unwrap();

        let record = TelosObject::DecisionRecord(DecisionRecord {
            intent_id: id1.clone(),
            author: Author {
                name: "Test".into(),
                email: "test@test.com".into(),
            },
            timestamp: Utc::now(),
            question: "Q?".into(),
            decision: "D".into(),
            rationale: None,
            alternatives: vec![],
            tags: vec![],
        });
        let _id2 = odb.write(&record).unwrap();

        let all = odb.iter_all().unwrap();
        assert_eq!(all.len(), 2);
    }
}
