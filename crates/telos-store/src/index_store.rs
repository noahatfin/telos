//! Index layer for accelerating Telos queries.
//!
//! Indexes are caches stored in `.telos/indexes/`. They can be rebuilt
//! from the object store at any time via `rebuild_all()`.

use crate::error::StoreError;
use crate::odb::ObjectDatabase;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use telos_core::hash::ObjectId;
use telos_core::object::TelosObject;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub id: String,
    pub object_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathIndexEntry {
    pub id: String,
    pub object_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub binding_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexFile<T> {
    version: u32,
    entries: HashMap<String, Vec<T>>,
}

impl<T> Default for IndexFile<T> {
    fn default() -> Self {
        Self {
            version: 2,
            entries: HashMap::new(),
        }
    }
}

/// Manages query indexes stored in `.telos/indexes/`.
pub struct IndexStore {
    indexes_dir: PathBuf,
}

impl IndexStore {
    pub fn new(indexes_dir: impl Into<PathBuf>) -> Self {
        Self {
            indexes_dir: indexes_dir.into(),
        }
    }

    pub fn ensure_dir(&self) -> Result<(), StoreError> {
        fs::create_dir_all(&self.indexes_dir)?;
        Ok(())
    }

    fn impact_path(&self) -> PathBuf {
        self.indexes_dir.join("impact.json")
    }

    fn codepath_path(&self) -> PathBuf {
        self.indexes_dir.join("codepath.json")
    }

    fn symbols_path(&self) -> PathBuf {
        self.indexes_dir.join("symbols.json")
    }

    fn load_index<T: for<'de> Deserialize<'de>>(&self, path: &PathBuf) -> IndexFile<T> {
        match fs::read_to_string(path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => IndexFile::default(),
        }
    }

    fn save_index<T: Serialize>(&self, path: &PathBuf, index: &IndexFile<T>) -> Result<(), StoreError> {
        self.ensure_dir()?;
        let json = serde_json::to_string_pretty(index)?;
        let tmp_path = path.with_extension("json.tmp");
        let mut f = fs::File::create(&tmp_path)?;
        f.write_all(json.as_bytes())?;
        f.flush()?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    }

    /// Update indexes for a newly written object.
    pub fn update_for_object(&self, id: &ObjectId, obj: &TelosObject) -> Result<(), StoreError> {
        match obj {
            TelosObject::Intent(intent) => {
                if !intent.impacts.is_empty() {
                    let mut index: IndexFile<IndexEntry> = self.load_index(&self.impact_path());
                    let entry = IndexEntry {
                        id: id.hex().to_string(),
                        object_type: "intent".into(),
                    };
                    for tag in &intent.impacts {
                        index.entries.entry(tag.clone()).or_default().push(entry.clone());
                    }
                    self.save_index(&self.impact_path(), &index)?;
                }
            }
            TelosObject::Constraint(c) => {
                if !c.impacts.is_empty() {
                    let mut index: IndexFile<IndexEntry> = self.load_index(&self.impact_path());
                    let entry = IndexEntry {
                        id: id.hex().to_string(),
                        object_type: "constraint".into(),
                    };
                    for tag in &c.impacts {
                        index.entries.entry(tag.clone()).or_default().push(entry.clone());
                    }
                    self.save_index(&self.impact_path(), &index)?;
                }
            }
            TelosObject::CodeBinding(cb) => {
                let entry = PathIndexEntry {
                    id: id.hex().to_string(),
                    object_type: "code_binding".into(),
                    symbol: cb.symbol.clone(),
                    binding_type: Some(format!("{:?}", cb.binding_type).to_lowercase()),
                };
                let mut codepath: IndexFile<PathIndexEntry> = self.load_index(&self.codepath_path());
                codepath.entries.entry(cb.path.clone()).or_default().push(entry.clone());
                self.save_index(&self.codepath_path(), &codepath)?;

                if let Some(ref sym) = cb.symbol {
                    let mut symbols: IndexFile<PathIndexEntry> = self.load_index(&self.symbols_path());
                    symbols.entries.entry(sym.clone()).or_default().push(entry);
                    self.save_index(&self.symbols_path(), &symbols)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Rebuild all indexes from the object store.
    pub fn rebuild_all(&self, odb: &ObjectDatabase) -> Result<(usize, usize, usize), StoreError> {
        self.ensure_dir()?;
        let mut impact: IndexFile<IndexEntry> = IndexFile::default();
        let mut codepath: IndexFile<PathIndexEntry> = IndexFile::default();
        let mut symbols: IndexFile<PathIndexEntry> = IndexFile::default();

        for (id, obj) in odb.iter_all()? {
            match &obj {
                TelosObject::Intent(intent) => {
                    let entry = IndexEntry {
                        id: id.hex().to_string(),
                        object_type: "intent".into(),
                    };
                    for tag in &intent.impacts {
                        impact.entries.entry(tag.clone()).or_default().push(entry.clone());
                    }
                }
                TelosObject::Constraint(c) => {
                    let entry = IndexEntry {
                        id: id.hex().to_string(),
                        object_type: "constraint".into(),
                    };
                    for tag in &c.impacts {
                        impact.entries.entry(tag.clone()).or_default().push(entry.clone());
                    }
                }
                TelosObject::CodeBinding(cb) => {
                    let entry = PathIndexEntry {
                        id: id.hex().to_string(),
                        object_type: "code_binding".into(),
                        symbol: cb.symbol.clone(),
                        binding_type: Some(format!("{:?}", cb.binding_type).to_lowercase()),
                    };
                    codepath.entries.entry(cb.path.clone()).or_default().push(entry.clone());
                    if let Some(ref sym) = cb.symbol {
                        symbols.entries.entry(sym.clone()).or_default().push(entry);
                    }
                }
                _ => {}
            }
        }

        let impact_count = impact.entries.len();
        let path_count = codepath.entries.len();
        let sym_count = symbols.entries.len();

        self.save_index(&self.impact_path(), &impact)?;
        self.save_index(&self.codepath_path(), &codepath)?;
        self.save_index(&self.symbols_path(), &symbols)?;

        Ok((impact_count, path_count, sym_count))
    }

    /// Lookup entries by impact tag.
    pub fn by_impact(&self, tag: &str) -> Vec<IndexEntry> {
        let index: IndexFile<IndexEntry> = self.load_index(&self.impact_path());
        index.entries.get(tag).cloned().unwrap_or_default()
    }

    /// Lookup entries by file path.
    pub fn by_path(&self, path: &str) -> Vec<PathIndexEntry> {
        let index: IndexFile<PathIndexEntry> = self.load_index(&self.codepath_path());
        index.entries.get(path).cloned().unwrap_or_default()
    }

    /// Lookup entries by symbol name.
    pub fn by_symbol(&self, name: &str) -> Vec<PathIndexEntry> {
        let index: IndexFile<PathIndexEntry> = self.load_index(&self.symbols_path());
        index.entries.get(name).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use telos_core::object::constraint::{Constraint, ConstraintSeverity, ConstraintStatus};
    use telos_core::object::code_binding::{CodeBinding, BindingType, BindingResolution};
    use telos_core::object::intent::{Author, Intent};

    fn make_odb_and_index() -> (tempfile::TempDir, ObjectDatabase, IndexStore) {
        let dir = tempfile::TempDir::new().unwrap();
        let odb = ObjectDatabase::new(dir.path().join("objects"));
        let index = IndexStore::new(dir.path().join("indexes"));
        (dir, odb, index)
    }

    #[test]
    fn update_and_lookup_by_impact() {
        let (_dir, odb, index) = make_odb_and_index();
        let intent = TelosObject::Intent(Intent {
            author: Author { name: "T".into(), email: "t@t".into() },
            timestamp: Utc::now(),
            statement: "test".into(),
            constraints: vec![],
            behavior_spec: vec![],
            parents: vec![],
            impacts: vec!["auth".into(), "security".into()],
            behavior_diff: None,
            metadata: std::collections::HashMap::new(),
        });
        let id = odb.write(&intent).unwrap();
        index.update_for_object(&id, &intent).unwrap();

        let results = index.by_impact("auth");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id.hex());
    }

    #[test]
    fn update_and_lookup_by_path() {
        let (_dir, odb, index) = make_odb_and_index();
        let cb = TelosObject::CodeBinding(CodeBinding {
            path: "src/auth/mod.rs".into(),
            symbol: Some("validate".into()),
            span: None,
            binding_type: BindingType::Function,
            resolution: BindingResolution::Unchecked,
            bound_object: ObjectId::hash(b"test"),
            metadata: std::collections::HashMap::new(),
        });
        let id = odb.write(&cb).unwrap();
        index.update_for_object(&id, &cb).unwrap();

        let results = index.by_path("src/auth/mod.rs");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].symbol, Some("validate".into()));

        let sym_results = index.by_symbol("validate");
        assert_eq!(sym_results.len(), 1);
    }

    #[test]
    fn rebuild_all_indexes() {
        let (_dir, odb, index) = make_odb_and_index();

        // Write some objects directly to odb without updating index
        let intent = TelosObject::Intent(Intent {
            author: Author { name: "T".into(), email: "t@t".into() },
            timestamp: Utc::now(),
            statement: "test".into(),
            constraints: vec![],
            behavior_spec: vec![],
            parents: vec![],
            impacts: vec!["payments".into()],
            behavior_diff: None,
            metadata: std::collections::HashMap::new(),
        });
        odb.write(&intent).unwrap();

        let constraint = TelosObject::Constraint(Constraint {
            author: Author { name: "T".into(), email: "t@t".into() },
            timestamp: Utc::now(),
            statement: "must pay".into(),
            severity: ConstraintSeverity::Must,
            status: ConstraintStatus::Active,
            source_intent: ObjectId::hash(b"dummy"),
            superseded_by: None,
            deprecation_reason: None,
            scope: vec![],
            impacts: vec!["payments".into()],
            metadata: std::collections::HashMap::new(),
        });
        odb.write(&constraint).unwrap();

        // Index should be empty before rebuild
        assert!(index.by_impact("payments").is_empty());

        // Rebuild
        let (impact_count, _path_count, _sym_count) = index.rebuild_all(&odb).unwrap();
        assert!(impact_count > 0);

        // Now index should have entries
        let results = index.by_impact("payments");
        assert_eq!(results.len(), 2);
    }
}
