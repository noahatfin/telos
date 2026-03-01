use crate::error::StoreError;
use crate::index_store::IndexStore;
use crate::odb::ObjectDatabase;
use crate::refs::RefStore;
use chrono::Utc;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use telos_core::hash::ObjectId;
use telos_core::object::intent_stream::IntentStreamRef;
use telos_core::object::{
    AgentOperation, ChangeSet, CodeBinding, Constraint, DecisionRecord, Intent, TelosObject,
};

const TELOS_DIR: &str = ".telos";

/// High-level repository abstraction.
///
/// Combines the object database, reference store, and provides
/// operations like creating intents and walking the DAG.
pub struct Repository {
    root: PathBuf,
    pub odb: ObjectDatabase,
    pub refs: RefStore,
    pub indexes: IndexStore,
}

impl Repository {
    /// Initialize a new Telos repository at `path`.
    pub fn init(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let root = path.as_ref().to_path_buf();
        let telos_dir = root.join(TELOS_DIR);

        if telos_dir.exists() {
            return Err(StoreError::RepositoryExists(
                telos_dir.display().to_string(),
            ));
        }

        // Create directory structure
        fs::create_dir_all(telos_dir.join("objects"))?;
        fs::create_dir_all(telos_dir.join("refs").join("streams"))?;
        fs::create_dir_all(telos_dir.join("logs").join("streams"))?;
        fs::create_dir_all(telos_dir.join("indexes"))?;

        // Write default config
        let config = serde_json::json!({
            "version": 1,
            "created_at": Utc::now().to_rfc3339(),
        });
        fs::write(
            telos_dir.join("config.json"),
            serde_json::to_string_pretty(&config)?,
        )?;

        let repo = Self {
            odb: ObjectDatabase::new(telos_dir.join("objects")),
            refs: RefStore::new(&telos_dir),
            indexes: IndexStore::new(telos_dir.join("indexes")),
            root,
        };

        // Initialize HEAD -> main
        repo.refs.set_head("main")?;

        // Create the main stream
        let main_stream = IntentStreamRef {
            name: "main".into(),
            tip: None,
            created_at: Utc::now(),
            description: Some("Default intent stream".into()),
        };
        repo.refs.create_stream(&main_stream)?;

        Ok(repo)
    }

    /// Open an existing repository at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let root = path.as_ref().to_path_buf();
        let telos_dir = root.join(TELOS_DIR);

        if !telos_dir.exists() {
            return Err(StoreError::RepositoryNotFound(
                root.display().to_string(),
            ));
        }

        Ok(Self {
            odb: ObjectDatabase::new(telos_dir.join("objects")),
            refs: RefStore::new(&telos_dir),
            indexes: IndexStore::new(telos_dir.join("indexes")),
            root,
        })
    }

    /// Search upward from `start` for a `.telos/` directory and open that repo.
    pub fn discover(start: impl AsRef<Path>) -> Result<Self, StoreError> {
        let mut current = start.as_ref().to_path_buf();
        loop {
            if current.join(TELOS_DIR).exists() {
                return Self::open(&current);
            }
            if !current.pop() {
                return Err(StoreError::RepositoryNotFound(
                    start.as_ref().display().to_string(),
                ));
            }
        }
    }

    /// Root path of the repository.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Create an intent, store it, and advance the current stream tip.
    pub fn create_intent(&self, intent: Intent) -> Result<ObjectId, StoreError> {
        // Validate parent references exist and are Intents
        for parent_id in &intent.parents {
            match self.odb.read(parent_id)? {
                TelosObject::Intent(_) => {}
                other => {
                    return Err(StoreError::InvalidReference(format!(
                        "parent {} is a {}, expected intent",
                        parent_id, other.type_tag()
                    )));
                }
            }
        }
        let obj = TelosObject::Intent(intent);
        let id = self.odb.write(&obj)?;
        self.indexes.update_for_object(&id, &obj)?;
        self.refs.update_current_tip(id.clone())?;
        Ok(id)
    }

    /// Create a decision record and store it.
    pub fn create_decision(&self, record: DecisionRecord) -> Result<ObjectId, StoreError> {
        // Validate intent_id exists and is an Intent
        match self.odb.read(&record.intent_id)? {
            TelosObject::Intent(_) => {}
            other => {
                return Err(StoreError::InvalidReference(format!(
                    "intent_id {} is a {}, expected intent",
                    record.intent_id, other.type_tag()
                )));
            }
        }
        let obj = TelosObject::DecisionRecord(record);
        let id = self.odb.write(&obj)?;
        self.indexes.update_for_object(&id, &obj)?;
        Ok(id)
    }

    /// Create a constraint and store it.
    pub fn create_constraint(&self, constraint: Constraint) -> Result<ObjectId, StoreError> {
        let obj = TelosObject::Constraint(constraint);
        let id = self.odb.write(&obj)?;
        self.indexes.update_for_object(&id, &obj)?;
        Ok(id)
    }

    /// Create a code binding and store it.
    pub fn create_code_binding(&self, binding: CodeBinding) -> Result<ObjectId, StoreError> {
        let obj = TelosObject::CodeBinding(binding);
        let id = self.odb.write(&obj)?;
        self.indexes.update_for_object(&id, &obj)?;
        Ok(id)
    }

    /// Create an agent operation and store it.
    pub fn create_agent_operation(&self, op: AgentOperation) -> Result<ObjectId, StoreError> {
        let obj = TelosObject::AgentOperation(op);
        let id = self.odb.write(&obj)?;
        self.indexes.update_for_object(&id, &obj)?;
        Ok(id)
    }

    /// Create a change set and store it.
    pub fn create_change_set(&self, cs: ChangeSet) -> Result<ObjectId, StoreError> {
        let obj = TelosObject::ChangeSet(cs);
        let id = self.odb.write(&obj)?;
        self.indexes.update_for_object(&id, &obj)?;
        Ok(id)
    }

    /// Read any object by ID (exact or prefix).
    pub fn read_object(&self, id_or_prefix: &str) -> Result<(ObjectId, TelosObject), StoreError> {
        // Try exact parse first
        if let Ok(id) = ObjectId::parse(id_or_prefix) {
            let obj = self.odb.read(&id)?;
            return Ok((id, obj));
        }
        // Try prefix resolution
        let id = self.odb.resolve_prefix(id_or_prefix)?;
        let obj = self.odb.read(&id)?;
        Ok((id, obj))
    }

    /// Walk the intent DAG starting from `start`, yielding (ObjectId, Intent) in BFS order.
    pub fn walk_intents(&self, start: &ObjectId) -> IntentWalker<'_> {
        IntentWalker::new(&self.odb, start.clone())
    }
}

/// BFS walker over the intent DAG (follows `parents` links).
pub struct IntentWalker<'a> {
    odb: &'a ObjectDatabase,
    queue: VecDeque<ObjectId>,
    visited: HashSet<String>,
}

impl<'a> IntentWalker<'a> {
    fn new(odb: &'a ObjectDatabase, start: ObjectId) -> Self {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        visited.insert(start.hex().to_string());
        queue.push_back(start);
        Self {
            odb,
            queue,
            visited,
        }
    }
}

impl<'a> Iterator for IntentWalker<'a> {
    type Item = Result<(ObjectId, Intent), StoreError>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(id) = self.queue.pop_front() {
            let obj = match self.odb.read(&id) {
                Ok(obj) => obj,
                Err(e) => return Some(Err(e)),
            };

            if let TelosObject::Intent(intent) = obj {
                // Enqueue unvisited parents
                for parent_id in &intent.parents {
                    if self.visited.insert(parent_id.hex().to_string()) {
                        self.queue.push_back(parent_id.clone());
                    }
                }
                return Some(Ok((id, intent)));
            }
            // Skip non-Intent objects in the walk
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use telos_core::object::intent::Author;
    use std::collections::HashMap;

    fn make_intent(statement: &str, parents: Vec<ObjectId>) -> Intent {
        Intent {
            author: Author {
                name: "Test".into(),
                email: "test@test.com".into(),
            },
            timestamp: Utc::now(),
            statement: statement.into(),
            constraints: vec![],
            behavior_spec: vec![],
            parents,
            impacts: vec![],
            behavior_diff: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn init_and_open() {
        let dir = tempfile::tempdir().unwrap();
        Repository::init(dir.path()).unwrap();
        let repo = Repository::open(dir.path()).unwrap();
        assert_eq!(repo.refs.read_head().unwrap(), "main");
    }

    #[test]
    fn init_twice_fails() {
        let dir = tempfile::tempdir().unwrap();
        Repository::init(dir.path()).unwrap();
        assert!(Repository::init(dir.path()).is_err());
    }

    #[test]
    fn discover_from_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        Repository::init(dir.path()).unwrap();
        let sub = dir.path().join("a").join("b").join("c");
        fs::create_dir_all(&sub).unwrap();
        let repo = Repository::discover(&sub).unwrap();
        assert_eq!(repo.root(), dir.path());
    }

    #[test]
    fn create_intent_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        let intent = make_intent("First intent", vec![]);
        let id = repo.create_intent(intent.clone()).unwrap();

        let (read_id, obj) = repo.read_object(id.hex()).unwrap();
        assert_eq!(read_id, id);
        if let TelosObject::Intent(read_intent) = obj {
            assert_eq!(read_intent.statement, "First intent");
        } else {
            panic!("expected Intent");
        }

        // Stream tip should be updated
        let stream = repo.refs.current_stream().unwrap();
        assert_eq!(stream.tip.unwrap(), id);
    }

    #[test]
    fn read_object_by_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        let intent = make_intent("Test", vec![]);
        let id = repo.create_intent(intent).unwrap();

        let prefix = &id.hex()[..8];
        let (resolved_id, _) = repo.read_object(prefix).unwrap();
        assert_eq!(resolved_id, id);
    }

    #[test]
    fn create_intent_validates_parents_exist() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        let fake_parent = ObjectId::hash(b"nonexistent");
        let intent = make_intent("Bad parent", vec![fake_parent]);
        let result = repo.create_intent(intent);
        assert!(result.is_err(), "should reject intent with nonexistent parent");
    }

    #[test]
    fn create_decision_validates_intent_exists() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        let record = DecisionRecord {
            intent_id: ObjectId::hash(b"nonexistent"),
            author: Author { name: "T".into(), email: "t@t".into() },
            timestamp: Utc::now(),
            question: "Q?".into(),
            decision: "D".into(),
            rationale: None,
            alternatives: vec![],
            tags: vec![],
        };
        let result = repo.create_decision(record);
        assert!(result.is_err(), "should reject decision with nonexistent intent");
    }

    #[test]
    fn walk_intent_dag() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        let i1 = make_intent("Root", vec![]);
        let id1 = repo.create_intent(i1).unwrap();

        let i2 = make_intent("Child", vec![id1.clone()]);
        let id2 = repo.create_intent(i2).unwrap();

        let i3 = make_intent("Grandchild", vec![id2.clone()]);
        let id3 = repo.create_intent(i3).unwrap();

        let walked: Vec<_> = repo
            .walk_intents(&id3)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(walked.len(), 3);
        assert_eq!(walked[0].0, id3);
        assert_eq!(walked[1].0, id2);
        assert_eq!(walked[2].0, id1);
    }
}
