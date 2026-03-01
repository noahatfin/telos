# Telos v2 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement three parallel workstreams — security hardening, ChangeSet bridge, and LLM experiment framework — to advance Telos from prototype to validated tool.

**Architecture:** Three independent workstreams touching separate files/crates. WF1 hardens telos-store (refs, odb, error). WF2 adds ChangeSet CLI commands and index. WF3 creates a new telos-experiment crate with codex CLI integration.

**Tech Stack:** Rust 2021, clap 4, serde/serde_json, sha2, chrono, tempfile, std::process::Command (for codex invocation)

---

## WF1: Security Hardening

### Task 1: Stream Name Path Traversal Protection

**Files:**
- Modify: `crates/telos-store/src/refs.rs`
- Modify: `crates/telos-store/src/error.rs`

**Step 1: Add error variant for invalid stream names**

In `crates/telos-store/src/error.rs`, add after the `NoCurrentStream` variant:

```rust
#[error("invalid stream name '{0}': {1}")]
InvalidStreamName(String, String),
```

**Step 2: Write failing tests for path traversal**

In `crates/telos-store/src/refs.rs`, add these tests inside `mod tests`:

```rust
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
```

**Step 3: Run tests to verify they fail**

Run: `cargo test -p telos-store stream_name_rejects -- --nocapture`
Expected: FAIL — no validation exists yet

**Step 4: Implement `validate_stream_name`**

In `crates/telos-store/src/refs.rs`, add this method to `impl RefStore`:

```rust
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
```

**Step 5: Wire validation into `create_stream`, `write_stream`, `set_head`, and `delete_stream`**

Add `Self::validate_stream_name(name)?;` as the first line of:
- `create_stream` — validate `stream.name`
- `set_head` — validate `stream_name`
- `delete_stream` — validate `name`
- `write_stream` — validate `stream.name`

For `create_stream`, add before the `path.exists()` check:
```rust
Self::validate_stream_name(&stream.name)?;
```

**Step 6: Run tests to verify they pass**

Run: `cargo test -p telos-store stream_name -- --nocapture`
Expected: all 3 new tests PASS

**Step 7: Commit**

```bash
git add crates/telos-store/src/refs.rs crates/telos-store/src/error.rs
git commit -m "fix: add stream name validation to prevent path traversal"
```

---

### Task 2: Object Read Hash Verification

**Files:**
- Modify: `crates/telos-store/src/odb.rs`
- Modify: `crates/telos-store/src/error.rs`

**Step 1: Add integrity error variant**

In `crates/telos-store/src/error.rs`, add:

```rust
#[error("integrity error: expected {expected}, got {actual}")]
IntegrityError { expected: String, actual: String },
```

**Step 2: Write failing test**

In `crates/telos-store/src/odb.rs` `mod tests`, add:

```rust
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
```

**Step 3: Run test to verify it fails**

Run: `cargo test -p telos-store read_detects_corrupted -- --nocapture`
Expected: FAIL — current `read()` doesn't verify hash

**Step 4: Add hash verification to `read()`**

Replace the `read` method in `crates/telos-store/src/odb.rs`:

```rust
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
```

**Step 5: Run tests**

Run: `cargo test -p telos-store -- --nocapture`
Expected: all tests PASS including the new corruption test

**Step 6: Commit**

```bash
git add crates/telos-store/src/odb.rs crates/telos-store/src/error.rs
git commit -m "fix: verify object hash on read to detect corruption"
```

---

### Task 3: `iter_all` Error Reporting

**Files:**
- Modify: `crates/telos-store/src/odb.rs`

**Step 1: Add `CorruptedObject` struct and update `iter_all` signature**

At the top of `odb.rs`, add:

```rust
/// A record of an object that failed to load.
#[derive(Debug)]
pub struct CorruptedObject {
    pub path: String,
    pub error: String,
}
```

**Step 2: Write failing test**

```rust
#[test]
fn iter_all_reports_corrupted_objects() {
    let dir = tempfile::tempdir().unwrap();
    let odb = ObjectDatabase::new(dir.path().join("objects"));

    // Write a valid object
    let obj = sample_intent();
    let _id = odb.write(&obj).unwrap();

    // Write a garbage file in a valid fan-out dir
    let corrupt_dir = dir.path().join("objects").join("ab");
    fs::create_dir_all(&corrupt_dir).unwrap();
    fs::write(
        corrupt_dir.join("abcdef1234567890abcdef1234567890abcdef1234567890abcdef12345678"),
        b"NOT VALID DATA",
    ).unwrap();

    let (valid, corrupted) = odb.iter_all_with_errors().unwrap();
    assert_eq!(valid.len(), 1);
    assert_eq!(corrupted.len(), 1);
    assert!(corrupted[0].path.contains("ab"));
}
```

**Step 3: Run test to verify it fails**

Run: `cargo test -p telos-store iter_all_reports -- --nocapture`
Expected: FAIL — method doesn't exist yet

**Step 4: Implement `iter_all_with_errors`**

Add new method to `ObjectDatabase`:

```rust
/// Iterate over all objects, reporting corrupted ones separately.
pub fn iter_all_with_errors(
    &self,
) -> Result<(Vec<(ObjectId, TelosObject)>, Vec<CorruptedObject>), StoreError> {
    let mut valid = Vec::new();
    let mut corrupted = Vec::new();

    let entries = match fs::read_dir(&self.objects_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok((valid, corrupted)),
        Err(e) => return Err(StoreError::Io(e)),
    };

    for fan_entry in entries {
        let fan_entry = fan_entry.map_err(StoreError::Io)?;
        let fan_name = fan_entry.file_name().to_string_lossy().to_string();
        if fan_name.len() != 2 || !fan_entry.path().is_dir() {
            continue;
        }
        let sub_entries = fs::read_dir(fan_entry.path()).map_err(StoreError::Io)?;
        for obj_entry in sub_entries {
            let obj_entry = obj_entry.map_err(StoreError::Io)?;
            let obj_name = obj_entry.file_name().to_string_lossy().to_string();
            let hex = format!("{}{}", fan_name, obj_name);
            match ObjectId::parse(&hex) {
                Ok(id) => match self.read(&id) {
                    Ok(obj) => valid.push((id, obj)),
                    Err(e) => corrupted.push(CorruptedObject {
                        path: obj_entry.path().display().to_string(),
                        error: e.to_string(),
                    }),
                },
                Err(_) => continue,
            }
        }
    }
    Ok((valid, corrupted))
}
```

**Step 5: Run tests**

Run: `cargo test -p telos-store -- --nocapture`
Expected: all PASS

**Step 6: Commit**

```bash
git add crates/telos-store/src/odb.rs
git commit -m "feat: add iter_all_with_errors to report corrupted objects"
```

---

### Task 4: Repository Layer Integrity Validation

**Files:**
- Modify: `crates/telos-store/src/repository.rs`
- Modify: `crates/telos-store/src/error.rs`

**Step 1: Add reference validation error**

In `crates/telos-store/src/error.rs`, add:

```rust
#[error("invalid reference: {0}")]
InvalidReference(String),
```

**Step 2: Write failing tests**

In `crates/telos-store/src/repository.rs` `mod tests`, add:

```rust
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
```

Add the necessary import at the top of `mod tests`: `use telos_core::object::decision_record::DecisionRecord;`

**Step 3: Run tests to verify they fail**

Run: `cargo test -p telos-store create_intent_validates create_decision_validates -- --nocapture`
Expected: FAIL — no validation exists

**Step 4: Add validation to `create_intent`**

Replace `create_intent` in `repository.rs`:

```rust
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
```

**Step 5: Add validation to `create_decision`**

Replace `create_decision` in `repository.rs`:

```rust
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
```

**Step 6: Run all tests**

Run: `cargo test -p telos-store -- --nocapture`
Expected: all PASS

**Step 7: Commit**

```bash
git add crates/telos-store/src/repository.rs crates/telos-store/src/error.rs
git commit -m "fix: validate reference integrity in Repository create methods"
```

---

## WF2: ChangeSet Implementation

### Task 5: ChangeSet Index Support

**Files:**
- Modify: `crates/telos-store/src/index_store.rs`

**Step 1: Write failing test for commit index**

In `crates/telos-store/src/index_store.rs` `mod tests`, add:

```rust
#[test]
fn update_and_lookup_by_commit() {
    let (_dir, odb, index) = make_odb_and_index();
    let cs = TelosObject::ChangeSet(telos_core::object::change_set::ChangeSet {
        author: Author { name: "T".into(), email: "t@t".into() },
        timestamp: Utc::now(),
        git_commit: "abc123def456".into(),
        parents: vec![],
        intents: vec![],
        constraints: vec![],
        decisions: vec![],
        code_bindings: vec![],
        agent_operations: vec![],
        metadata: std::collections::HashMap::new(),
    });
    let id = odb.write(&cs).unwrap();
    index.update_for_object(&id, &cs).unwrap();

    let results = index.by_commit("abc123def456");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, id.hex());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p telos-store update_and_lookup_by_commit -- --nocapture`
Expected: FAIL — `by_commit` doesn't exist

**Step 3: Implement commit index**

In `crates/telos-store/src/index_store.rs`, add:

```rust
fn commits_path(&self) -> PathBuf {
    self.indexes_dir.join("commits.json")
}

/// Lookup entries by git commit SHA.
pub fn by_commit(&self, sha: &str) -> Vec<IndexEntry> {
    let index: IndexFile<IndexEntry> = self.load_index(&self.commits_path());
    index.entries.get(sha).cloned().unwrap_or_default()
}
```

In `update_for_object`, add a new arm after the `TelosObject::CodeBinding` arm:

```rust
TelosObject::ChangeSet(cs) => {
    let mut index: IndexFile<IndexEntry> = self.load_index(&self.commits_path());
    let entry = IndexEntry {
        id: id.hex().to_string(),
        object_type: "change_set".into(),
    };
    index.entries.entry(cs.git_commit.clone()).or_default().push(entry);
    self.save_index(&self.commits_path(), &index)?;

    // Also index by impact tags from referenced intents/constraints
    // (deferred to query time to avoid cascading lookups)
}
```

In `rebuild_all`, add a `commits` index file and a new arm in the loop:

```rust
// Add at top of rebuild_all:
let mut commits: IndexFile<IndexEntry> = IndexFile::default();

// Add in the match:
TelosObject::ChangeSet(cs) => {
    let entry = IndexEntry {
        id: id.hex().to_string(),
        object_type: "change_set".into(),
    };
    commits.entries.entry(cs.git_commit.clone()).or_default().push(entry);
}

// Add at the end, before the return:
self.save_index(&self.commits_path(), &commits)?;
// Update return to include commit count
```

Update the return type of `rebuild_all` to `(usize, usize, usize, usize)` and add `commits.entries.len()` as the fourth element. Update callers accordingly.

**Step 4: Run tests**

Run: `cargo test -p telos-store -- --nocapture`
Expected: all PASS

**Step 5: Commit**

```bash
git add crates/telos-store/src/index_store.rs
git commit -m "feat: add commits.json index for ChangeSet git commit lookup"
```

---

### Task 6: ChangeSet Query Support

**Files:**
- Modify: `crates/telos-store/src/query.rs`

**Step 1: Write failing test**

In `crates/telos-store/src/query.rs` `mod tests`, add:

```rust
#[test]
fn query_changesets_by_commit() {
    let (_dir, odb) = make_odb();
    let index = crate::index_store::IndexStore::new(_dir.path().join("indexes"));

    let cs = telos_core::object::change_set::ChangeSet {
        author: Author { name: "T".into(), email: "t@t".into() },
        timestamp: Utc::now(),
        git_commit: "a1b2c3d".into(),
        parents: vec![],
        intents: vec![],
        constraints: vec![],
        decisions: vec![],
        code_bindings: vec![],
        agent_operations: vec![],
        metadata: std::collections::HashMap::new(),
    };
    let obj = TelosObject::ChangeSet(cs);
    let id = odb.write(&obj).unwrap();
    index.update_for_object(&id, &obj).unwrap();

    let results = query_changesets(&odb, &index, Some("a1b2c3d"), None).unwrap();
    assert_eq!(results.len(), 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p telos-store query_changesets_by_commit -- --nocapture`
Expected: FAIL — function doesn't exist

**Step 3: Implement `query_changesets`**

Add to `crates/telos-store/src/query.rs`:

```rust
use telos_core::object::change_set::ChangeSet;

/// Query changesets with optional filters.
pub fn query_changesets(
    odb: &ObjectDatabase,
    index: &IndexStore,
    git_commit: Option<&str>,
    _impact: Option<&str>,
) -> Result<Vec<(ObjectId, ChangeSet)>, StoreError> {
    if let Some(commit_sha) = git_commit {
        // Use index for commit-based lookup
        let entries = index.by_commit(commit_sha);
        let mut results = Vec::new();
        for entry in entries {
            if let Ok(id) = ObjectId::parse(&entry.id) {
                if let Ok(TelosObject::ChangeSet(cs)) = odb.read(&id) {
                    results.push((id, cs));
                }
            }
        }
        results.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));
        return Ok(results);
    }

    // Fallback: scan all objects
    let all = odb.iter_all()?;
    let mut results = Vec::new();
    for (id, obj) in all {
        if let TelosObject::ChangeSet(cs) = obj {
            results.push((id, cs));
        }
    }
    results.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));
    Ok(results)
}
```

**Step 4: Run tests**

Run: `cargo test -p telos-store -- --nocapture`
Expected: all PASS

**Step 5: Commit**

```bash
git add crates/telos-store/src/query.rs
git commit -m "feat: add query_changesets with commit-based index lookup"
```

---

### Task 7: ChangeSet CLI Commands

**Files:**
- Create: `crates/telos-cli/src/commands/changeset.rs`
- Modify: `crates/telos-cli/src/commands/mod.rs`
- Modify: `crates/telos-cli/src/main.rs`

**Step 1: Create the changeset command module**

Create `crates/telos-cli/src/commands/changeset.rs`:

```rust
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::process::Command;
use telos_core::hash::ObjectId;
use telos_core::object::change_set::ChangeSet;
use telos_core::object::intent::Author;
use telos_core::object::TelosObject;
use telos_store::query;
use telos_store::repository::Repository;

/// Create a changeset linking a git commit to Telos objects.
pub fn create(
    commit: String,
    intents: Vec<String>,
    constraints: Vec<String>,
    decisions: Vec<String>,
    json: bool,
) -> Result<()> {
    let repo = Repository::discover(".")?;

    // Resolve git commit SHA
    let git_commit = if commit == "HEAD" {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()?;
        String::from_utf8(output.stdout)?.trim().to_string()
    } else {
        commit
    };

    // Resolve all referenced Telos object IDs
    let mut intent_ids = Vec::new();
    for id_str in &intents {
        let (id, obj) = repo.read_object(id_str)?;
        if !matches!(obj, TelosObject::Intent(_)) {
            anyhow::bail!("{} is not an intent", id);
        }
        intent_ids.push(id);
    }

    let mut constraint_ids = Vec::new();
    for id_str in &constraints {
        let (id, obj) = repo.read_object(id_str)?;
        if !matches!(obj, TelosObject::Constraint(_)) {
            anyhow::bail!("{} is not a constraint", id);
        }
        constraint_ids.push(id);
    }

    let mut decision_ids = Vec::new();
    for id_str in &decisions {
        let (id, obj) = repo.read_object(id_str)?;
        if !matches!(obj, TelosObject::DecisionRecord(_)) {
            anyhow::bail!("{} is not a decision record", id);
        }
        decision_ids.push(id);
    }

    let cs = ChangeSet {
        author: Author {
            name: "telos-cli".into(),
            email: "".into(),
        },
        timestamp: Utc::now(),
        git_commit: git_commit.clone(),
        parents: vec![],
        intents: intent_ids,
        constraints: constraint_ids,
        decisions: decision_ids,
        code_bindings: vec![],
        agent_operations: vec![],
        metadata: HashMap::new(),
    };

    let id = repo.create_change_set(cs)?;

    if json {
        let (_, obj) = repo.read_object(id.hex())?;
        println!("{}", serde_json::to_string_pretty(&obj)?);
    } else {
        println!("Created changeset {} for commit {}", id, &git_commit[..8.min(git_commit.len())]);
    }
    Ok(())
}

/// Show a changeset by ID.
pub fn show(id: String, json: bool) -> Result<()> {
    let repo = Repository::discover(".")?;
    let (obj_id, obj) = repo.read_object(&id)?;

    if let TelosObject::ChangeSet(cs) = &obj {
        if json {
            println!("{}", serde_json::to_string_pretty(&obj)?);
        } else {
            println!("ChangeSet {}", obj_id);
            println!("  Git commit: {}", cs.git_commit);
            println!("  Intents: {}", cs.intents.len());
            println!("  Constraints: {}", cs.constraints.len());
            println!("  Decisions: {}", cs.decisions.len());
            println!("  Code bindings: {}", cs.code_bindings.len());
            println!("  Agent ops: {}", cs.agent_operations.len());
        }
    } else {
        anyhow::bail!("{} is not a changeset", obj_id);
    }
    Ok(())
}

/// Find the changeset for a git commit.
pub fn for_commit(commit_sha: String, json: bool) -> Result<()> {
    let repo = Repository::discover(".")?;

    let sha = if commit_sha == "HEAD" {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()?;
        String::from_utf8(output.stdout)?.trim().to_string()
    } else {
        commit_sha
    };

    let results = query::query_changesets(&repo.odb, &repo.indexes, Some(&sha), None)?;

    if results.is_empty() {
        eprintln!("No changeset found for commit {}", &sha[..8.min(sha.len())]);
        return Ok(());
    }

    if json {
        let output: Vec<_> = results
            .iter()
            .map(|(id, cs)| {
                serde_json::json!({
                    "id": id.hex(),
                    "changeset": cs,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        for (id, cs) in &results {
            println!("ChangeSet {} -> commit {}", id, &cs.git_commit[..8.min(cs.git_commit.len())]);
            println!("  Intents: {}", cs.intents.len());
            println!("  Constraints: {}", cs.constraints.len());
            println!("  Decisions: {}", cs.decisions.len());
        }
    }
    Ok(())
}
```

**Step 2: Register the module and CLI commands**

In `crates/telos-cli/src/commands/mod.rs`, add:
```rust
pub mod changeset;
```

In `crates/telos-cli/src/main.rs`, add to the `Commands` enum:

```rust
/// Manage changesets (Git commit <-> Telos reasoning bridge)
Changeset {
    #[command(subcommand)]
    action: ChangesetAction,
},
```

Add the `ChangesetAction` enum:

```rust
#[derive(Subcommand)]
enum ChangesetAction {
    /// Create a changeset linking a git commit to Telos objects
    Create {
        /// Git commit SHA or HEAD
        #[arg(long)]
        commit: String,
        /// Intent IDs (repeatable)
        #[arg(long)]
        intent: Vec<String>,
        /// Constraint IDs (repeatable)
        #[arg(long)]
        constraint: Vec<String>,
        /// Decision IDs (repeatable)
        #[arg(long)]
        decision: Vec<String>,
    },
    /// Show a changeset by ID
    Show {
        /// Changeset ID
        id: String,
    },
    /// Find changeset for a git commit
    ForCommit {
        /// Git commit SHA or HEAD
        commit: String,
    },
}
```

In the `match cli.command` block, add:

```rust
Commands::Changeset { action } => match action {
    ChangesetAction::Create {
        commit,
        intent,
        constraint,
        decision,
    } => commands::changeset::create(commit, intent, constraint, decision, cli.json),
    ChangesetAction::Show { id } => commands::changeset::show(id, cli.json),
    ChangesetAction::ForCommit { commit } => commands::changeset::for_commit(commit, cli.json),
},
```

**Step 3: Run build**

Run: `cargo build -p telos-cli`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/telos-cli/src/commands/changeset.rs crates/telos-cli/src/commands/mod.rs crates/telos-cli/src/main.rs
git commit -m "feat: add changeset CLI commands (create, show, for-commit)"
```

---

### Task 8: ChangeSet CLI Integration Tests

**Files:**
- Modify: `tests/integration/cli_test.rs`

**Step 1: Write integration tests**

Add to `tests/integration/cli_test.rs`:

```rust
#[test]
fn changeset_create_and_show() {
    let dir = setup_repo();

    // Create an intent first
    let output = telos_cmd(&dir)
        .args(["intent", "-s", "Test intent", "--impact", "auth"])
        .output()
        .unwrap();
    let intent_id = extract_id(&String::from_utf8(output.stdout).unwrap());

    // Initialize a git repo for commit SHA
    Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "--allow-empty", "-m", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create changeset
    let output = telos_cmd(&dir)
        .args([
            "changeset", "create",
            "--commit", "HEAD",
            "--intent", &intent_id,
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Created changeset"));

    // Show changeset (extract ID from create output)
    let cs_id = extract_id(&stdout);
    let output = telos_cmd(&dir)
        .args(["changeset", "show", &cs_id, "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["type"], "change_set");
}

#[test]
fn changeset_for_commit() {
    let dir = setup_repo();

    // Create intent
    let output = telos_cmd(&dir)
        .args(["intent", "-s", "Test", "--impact", "auth"])
        .output()
        .unwrap();
    let intent_id = extract_id(&String::from_utf8(output.stdout).unwrap());

    // Init git and commit
    Command::new("git").args(["init"]).current_dir(dir.path()).output().unwrap();
    Command::new("git").args(["commit", "--allow-empty", "-m", "init"]).current_dir(dir.path()).output().unwrap();

    // Create changeset
    telos_cmd(&dir)
        .args(["changeset", "create", "--commit", "HEAD", "--intent", &intent_id])
        .output()
        .unwrap();

    // Query by commit
    let output = telos_cmd(&dir)
        .args(["changeset", "for-commit", "HEAD", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
}
```

Note: `extract_id` and `setup_repo` are helper functions — check existing test helpers in the file and follow the same pattern. `telos_cmd` should be similar to the existing test setup using `assert_cmd`.

**Step 2: Run tests**

Run: `cargo test -p telos-cli -- --nocapture`
Expected: all PASS

**Step 3: Commit**

```bash
git add tests/integration/cli_test.rs
git commit -m "test: add changeset CLI integration tests"
```

---

## WF3: Experiment Framework

### Task 9: Scaffold `telos-experiment` Crate

**Files:**
- Create: `crates/telos-experiment/Cargo.toml`
- Create: `crates/telos-experiment/src/lib.rs`
- Create: `crates/telos-experiment/src/main.rs`
- Create: `crates/telos-experiment/src/scenario.rs`
- Create: `crates/telos-experiment/src/codex.rs`
- Create: `crates/telos-experiment/src/runner.rs`
- Create: `crates/telos-experiment/src/scorer.rs`
- Create: `crates/telos-experiment/src/report.rs`
- Modify: `Cargo.toml` (workspace)

**Step 1: Create Cargo.toml**

```toml
[package]
name = "telos-experiment"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
clap = { workspace = true }
anyhow = { workspace = true }
toml = "0.8"

[[bin]]
name = "telos-experiment"
path = "src/main.rs"
```

**Step 2: Add to workspace**

In root `Cargo.toml`, add `"crates/telos-experiment"` to `members`.

**Step 3: Create minimal source files**

`src/lib.rs`:
```rust
pub mod codex;
pub mod report;
pub mod runner;
pub mod scenario;
pub mod scorer;
```

`src/scenario.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioFile {
    pub scenario: ScenarioMeta,
    pub diff: DiffConfig,
    pub context: ContextConfig,
    pub prompt: PromptConfig,
    pub expected: ExpectedConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioMeta {
    pub name: String,
    pub category: String, // "true_positive" or "false_positive"
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffConfig {
    pub content: String,
    pub commit_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub git_only: String,
    pub constraints_md: String,
    pub telos_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedConfig {
    pub should_reject: bool,
    pub key_findings: Vec<String>,
}

impl ScenarioFile {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let scenario: ScenarioFile = toml::from_str(&content)?;
        Ok(scenario)
    }

    /// Render the prompt template with the given condition's context.
    pub fn render_prompt(&self, condition: &str) -> String {
        let context = match condition {
            "git_only" => &self.context.git_only,
            "constraints_md" => &self.context.constraints_md,
            "telos" => &self.context.telos_json,
            _ => "",
        };
        self.prompt
            .template
            .replace("{{commit_message}}", &self.diff.commit_message)
            .replace("{{diff}}", &self.diff.content)
            .replace("{{context}}", context)
    }
}
```

`src/codex.rs`:
```rust
use anyhow::Result;
use std::process::Command;
use std::time::Instant;

pub struct CodexRunner {
    pub binary: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub struct CodexResponse {
    pub output: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

impl Default for CodexRunner {
    fn default() -> Self {
        Self {
            binary: "codex".into(),
            timeout_secs: 120,
        }
    }
}

impl CodexRunner {
    pub fn run(&self, prompt: &str) -> Result<CodexResponse> {
        let start = Instant::now();

        let output = Command::new(&self.binary)
            .args(["-q", "--prompt", prompt])
            .output()?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            anyhow::bail!(
                "codex exited with {}: {}",
                output.status.code().unwrap_or(-1),
                stderr
            );
        }

        Ok(CodexResponse {
            output: stdout,
            exit_code: output.status.code().unwrap_or(0),
            duration_ms,
        })
    }

    /// Check if the codex binary is available.
    pub fn is_available(&self) -> bool {
        Command::new(&self.binary)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
```

`src/runner.rs`:
```rust
use crate::codex::{CodexRunner, CodexResponse};
use crate::scenario::ScenarioFile;
use crate::scorer::{JudgeScorer, Score};
use serde::{Deserialize, Serialize};

pub const CONDITIONS: [&str; 3] = ["git_only", "constraints_md", "telos"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialResult {
    pub scenario_name: String,
    pub condition: String,
    pub trial_number: usize,
    pub llm_response: String,
    pub score: Option<Score>,
    pub duration_ms: u64,
}

pub struct ExperimentRunner {
    codex: CodexRunner,
    scorer: JudgeScorer,
    pub repeats: usize,
}

impl ExperimentRunner {
    pub fn new(repeats: usize) -> Self {
        let codex = CodexRunner::default();
        let scorer = JudgeScorer::new(CodexRunner::default());
        Self {
            codex,
            scorer,
            repeats,
        }
    }

    pub fn run_scenario(
        &self,
        scenario: &ScenarioFile,
        conditions: &[&str],
    ) -> anyhow::Result<Vec<TrialResult>> {
        let mut results = Vec::new();

        for &condition in conditions {
            let prompt = scenario.render_prompt(condition);

            for trial in 1..=self.repeats {
                eprintln!(
                    "  [{}/{}] {} / {} ...",
                    trial, self.repeats, scenario.scenario.name, condition
                );

                let response = self.codex.run(&prompt)?;

                let score = self
                    .scorer
                    .score(&scenario, &response.output)
                    .ok();

                results.push(TrialResult {
                    scenario_name: scenario.scenario.name.clone(),
                    condition: condition.into(),
                    trial_number: trial,
                    llm_response: response.output,
                    score,
                    duration_ms: response.duration_ms,
                });
            }
        }

        Ok(results)
    }
}
```

`src/scorer.rs`:
```rust
use crate::codex::CodexRunner;
use crate::scenario::ScenarioFile;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub caught_issue: bool,
    pub recommended_rejection: bool,
    pub cited_constraint: bool,
    pub reasoning_quality: u8,
    pub judge_explanation: String,
}

pub struct JudgeScorer {
    codex: CodexRunner,
}

impl JudgeScorer {
    pub fn new(codex: CodexRunner) -> Self {
        Self { codex }
    }

    pub fn score(
        &self,
        scenario: &ScenarioFile,
        llm_response: &str,
    ) -> anyhow::Result<Score> {
        let should_action = if scenario.expected.should_reject {
            "REJECT this change"
        } else {
            "APPROVE this change (it is benign)"
        };

        let findings = scenario.expected.key_findings.join(", ");

        let judge_prompt = format!(
r#"You are evaluating an AI code reviewer's response. Output ONLY valid JSON.

Scenario: {}
The correct action is to: {}
Key findings expected: {}

Reviewer's response:
{}

Score as JSON (no other text):
{{"caught_issue": true/false, "recommended_rejection": true/false, "cited_constraint": true/false, "reasoning_quality": 1-5, "judge_explanation": "brief explanation"}}"#,
            scenario.scenario.description,
            should_action,
            findings,
            llm_response,
        );

        let response = self.codex.run(&judge_prompt)?;
        let output = response.output.trim();

        // Try to extract JSON from the response
        let json_str = if let Some(start) = output.find('{') {
            if let Some(end) = output.rfind('}') {
                &output[start..=end]
            } else {
                output
            }
        } else {
            output
        };

        let score: Score = serde_json::from_str(json_str)?;
        Ok(score)
    }
}
```

`src/report.rs`:
```rust
use crate::runner::TrialResult;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct ScenarioReport {
    pub scenario_name: String,
    pub category: String,
    pub conditions: HashMap<String, ConditionStats>,
}

#[derive(Debug, Serialize)]
pub struct ConditionStats {
    pub trials: usize,
    pub caught_issue_rate: f64,
    pub rejection_rate: f64,
    pub cited_constraint_rate: f64,
    pub avg_reasoning_quality: f64,
    pub avg_duration_ms: f64,
}

pub fn aggregate(results: &[TrialResult], category: &str) -> Vec<ScenarioReport> {
    let mut by_scenario: HashMap<String, Vec<&TrialResult>> = HashMap::new();
    for r in results {
        by_scenario.entry(r.scenario_name.clone()).or_default().push(r);
    }

    let mut reports = Vec::new();
    for (name, trials) in by_scenario {
        let mut conditions: HashMap<String, ConditionStats> = HashMap::new();

        let mut by_condition: HashMap<String, Vec<&TrialResult>> = HashMap::new();
        for t in &trials {
            by_condition.entry(t.condition.clone()).or_default().push(t);
        }

        for (cond, cond_trials) in by_condition {
            let n = cond_trials.len();
            let scored: Vec<_> = cond_trials.iter().filter_map(|t| t.score.as_ref()).collect();
            let s = scored.len() as f64;

            conditions.insert(cond, ConditionStats {
                trials: n,
                caught_issue_rate: if s > 0.0 { scored.iter().filter(|s| s.caught_issue).count() as f64 / s } else { 0.0 },
                rejection_rate: if s > 0.0 { scored.iter().filter(|s| s.recommended_rejection).count() as f64 / s } else { 0.0 },
                cited_constraint_rate: if s > 0.0 { scored.iter().filter(|s| s.cited_constraint).count() as f64 / s } else { 0.0 },
                avg_reasoning_quality: if s > 0.0 { scored.iter().map(|s| s.reasoning_quality as f64).sum::<f64>() / s } else { 0.0 },
                avg_duration_ms: cond_trials.iter().map(|t| t.duration_ms as f64).sum::<f64>() / n as f64,
            });
        }

        reports.push(ScenarioReport {
            scenario_name: name,
            category: category.into(),
            conditions,
        });
    }

    reports
}

pub fn print_table(reports: &[ScenarioReport]) {
    for report in reports {
        println!("\n=== {} ({}) ===", report.scenario_name, report.category);
        println!("{:<20} {:>10} {:>10} {:>10} {:>10}",
            "Metric", "Git-only", "CONST.md", "Telos");

        let git = report.conditions.get("git_only");
        let cmd = report.conditions.get("constraints_md");
        let telos = report.conditions.get("telos");

        let fmt = |stats: Option<&ConditionStats>, f: fn(&ConditionStats) -> f64| -> String {
            stats.map(|s| format!("{:.0}%", f(s) * 100.0)).unwrap_or_else(|| "—".into())
        };

        println!("{:<20} {:>10} {:>10} {:>10}",
            "Caught issue", fmt(git, |s| s.caught_issue_rate), fmt(cmd, |s| s.caught_issue_rate), fmt(telos, |s| s.caught_issue_rate));
        println!("{:<20} {:>10} {:>10} {:>10}",
            "Rejected", fmt(git, |s| s.rejection_rate), fmt(cmd, |s| s.rejection_rate), fmt(telos, |s| s.rejection_rate));
        println!("{:<20} {:>10} {:>10} {:>10}",
            "Cited constraint", fmt(git, |s| s.cited_constraint_rate), fmt(cmd, |s| s.cited_constraint_rate), fmt(telos, |s| s.cited_constraint_rate));
    }
}
```

`src/main.rs`:
```rust
mod codex;
mod report;
mod runner;
mod scenario;
mod scorer;

use clap::{Parser, Subcommand};
use scenario::ScenarioFile;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "telos-experiment", about = "LLM experiment framework for Telos validation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run experiments
    Run {
        /// Number of repetitions per condition
        #[arg(long, default_value = "5")]
        repeats: usize,

        /// Specific scenario to run (by name)
        #[arg(long)]
        scenario: Option<String>,

        /// Conditions to test (repeatable: git_only, constraints_md, telos)
        #[arg(long)]
        condition: Vec<String>,

        /// Directory containing scenario TOML files
        #[arg(long, default_value = "crates/telos-experiment/scenarios")]
        scenarios_dir: PathBuf,
    },

    /// List available scenarios
    List {
        /// Directory containing scenario TOML files
        #[arg(long, default_value = "crates/telos-experiment/scenarios")]
        scenarios_dir: PathBuf,
    },

    /// Show report from latest results
    Report {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Results file path
        #[arg(long, default_value = ".telos-experiment/results/latest.json")]
        results: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run {
            repeats,
            scenario: scenario_filter,
            condition,
            scenarios_dir,
        } => run_experiments(repeats, scenario_filter, condition, scenarios_dir),
        Commands::List { scenarios_dir } => list_scenarios(scenarios_dir),
        Commands::Report { json, results } => show_report(json, results),
    };

    if let Err(e) = result {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn run_experiments(
    repeats: usize,
    scenario_filter: Option<String>,
    conditions: Vec<String>,
    scenarios_dir: PathBuf,
) -> anyhow::Result<()> {
    let codex_runner = codex::CodexRunner::default();
    if !codex_runner.is_available() {
        anyhow::bail!("codex CLI not found. Install it first: https://github.com/openai/codex");
    }

    let scenarios = load_scenarios(&scenarios_dir, scenario_filter.as_deref())?;
    if scenarios.is_empty() {
        anyhow::bail!("No scenarios found in {}", scenarios_dir.display());
    }

    let active_conditions: Vec<&str> = if conditions.is_empty() {
        runner::CONDITIONS.to_vec()
    } else {
        conditions.iter().map(|s| s.as_str()).collect()
    };

    eprintln!("Running {} scenarios x {} conditions x {} repeats",
        scenarios.len(), active_conditions.len(), repeats);

    let runner = runner::ExperimentRunner::new(repeats);
    let mut all_results = Vec::new();

    for scenario in &scenarios {
        eprintln!("\nScenario: {} ({})", scenario.scenario.name, scenario.scenario.category);
        let results = runner.run_scenario(scenario, &active_conditions)?;
        all_results.extend(results);
    }

    // Save results
    let results_dir = PathBuf::from(".telos-experiment/results");
    std::fs::create_dir_all(&results_dir)?;
    let results_json = serde_json::to_string_pretty(&all_results)?;
    let latest_path = results_dir.join("latest.json");
    std::fs::write(&latest_path, &results_json)?;

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let archive_path = results_dir.join(format!("run-{}.json", timestamp));
    std::fs::write(&archive_path, &results_json)?;

    eprintln!("\nResults saved to {}", latest_path.display());

    // Print summary
    let reports = report::aggregate(&all_results, "all");
    report::print_table(&reports);

    Ok(())
}

fn list_scenarios(scenarios_dir: PathBuf) -> anyhow::Result<()> {
    let scenarios = load_scenarios(&scenarios_dir, None)?;
    println!("{:<30} {:<15} {}", "Name", "Category", "Description");
    println!("{}", "-".repeat(80));
    for s in &scenarios {
        println!("{:<30} {:<15} {}",
            s.scenario.name, s.scenario.category, s.scenario.description);
    }
    println!("\n{} scenarios found", scenarios.len());
    Ok(())
}

fn show_report(json: bool, results_path: PathBuf) -> anyhow::Result<()> {
    let data = std::fs::read_to_string(&results_path)?;
    let results: Vec<runner::TrialResult> = serde_json::from_str(&data)?;

    if json {
        let reports = report::aggregate(&results, "all");
        println!("{}", serde_json::to_string_pretty(&reports)?);
    } else {
        let reports = report::aggregate(&results, "all");
        report::print_table(&reports);
    }
    Ok(())
}

fn load_scenarios(dir: &PathBuf, filter: Option<&str>) -> anyhow::Result<Vec<ScenarioFile>> {
    let mut scenarios = Vec::new();
    if !dir.exists() {
        return Ok(scenarios);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "toml").unwrap_or(false) {
            match ScenarioFile::load(&path) {
                Ok(s) => {
                    if let Some(f) = filter {
                        if s.scenario.name != f {
                            continue;
                        }
                    }
                    scenarios.push(s);
                }
                Err(e) => eprintln!("Warning: failed to load {}: {}", path.display(), e),
            }
        }
    }
    scenarios.sort_by(|a, b| a.scenario.name.cmp(&b.scenario.name));
    Ok(scenarios)
}
```

**Step 4: Build**

Run: `cargo build -p telos-experiment`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/telos-experiment/ Cargo.toml
git commit -m "feat: scaffold telos-experiment crate with codex CLI integration"
```

---

### Task 10: Create Experiment Scenarios

**Files:**
- Create: `crates/telos-experiment/scenarios/constraint_violation.toml`
- Create: `crates/telos-experiment/scenarios/status_validation_removal.toml`
- Create: `crates/telos-experiment/scenarios/info_leak.toml`
- Create: `crates/telos-experiment/scenarios/privilege_escalation.toml`
- Create: `crates/telos-experiment/scenarios/benign_refactor.toml`
- Create: `crates/telos-experiment/scenarios/benign_performance.toml`
- Create: `crates/telos-experiment/scenarios/benign_bugfix.toml`

Create each scenario file as per the design doc. The exact content for each is a substantial amount of TOML — create them following the `ScenarioFile` schema from Task 9. Each needs:
- Realistic diff content
- A plausible (possibly deceptive for true_positive) commit message
- Three context variants: git_only, constraints_md, telos_json
- An identical prompt template across all (solving E5)
- Expected findings

Use the EVALUATION.md scenarios as inspiration but write fresh diffs and constraints.

**Step 1: Create all 7 scenario TOML files**

(Each file follows the template from the design doc. Write realistic, self-contained scenarios.)

**Step 2: Verify they load**

Run: `cargo run -p telos-experiment -- list`
Expected: Lists all 7 scenarios

**Step 3: Commit**

```bash
git add crates/telos-experiment/scenarios/
git commit -m "feat: add 7 experiment scenarios (4 true positive, 3 false positive)"
```

---

### Task 11: Unit Tests for Experiment Framework

**Files:**
- Modify: `crates/telos-experiment/src/scenario.rs`
- Modify: `crates/telos-experiment/src/scorer.rs`

**Step 1: Write scenario tests**

Add to `scenario.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_scenario_from_toml() {
        let toml_content = r#"
[scenario]
name = "test_scenario"
category = "true_positive"
description = "A test"

[diff]
content = "- old\n+ new"
commit_message = "Update thing"

[context]
git_only = "git log output"
constraints_md = "- Must do X"
telos_json = '{"constraints": []}'

[prompt]
template = "Review: {{commit_message}}\n{{diff}}\n{{context}}"

[expected]
should_reject = true
key_findings = ["finding1"]
"#;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(toml_content.as_bytes()).unwrap();
        let scenario = ScenarioFile::load(tmp.path()).unwrap();
        assert_eq!(scenario.scenario.name, "test_scenario");
        assert!(scenario.expected.should_reject);
    }

    #[test]
    fn render_prompt_substitutes_context() {
        let scenario = ScenarioFile {
            scenario: ScenarioMeta {
                name: "test".into(),
                category: "true_positive".into(),
                description: "desc".into(),
            },
            diff: DiffConfig {
                content: "- old\n+ new".into(),
                commit_message: "fix stuff".into(),
            },
            context: ContextConfig {
                git_only: "GIT CONTEXT".into(),
                constraints_md: "MD CONTEXT".into(),
                telos_json: "TELOS CONTEXT".into(),
            },
            prompt: PromptConfig {
                template: "Msg: {{commit_message}}\nDiff: {{diff}}\nCtx: {{context}}".into(),
            },
            expected: ExpectedConfig {
                should_reject: true,
                key_findings: vec![],
            },
        };

        let git_prompt = scenario.render_prompt("git_only");
        assert!(git_prompt.contains("GIT CONTEXT"));
        assert!(git_prompt.contains("fix stuff"));

        let telos_prompt = scenario.render_prompt("telos");
        assert!(telos_prompt.contains("TELOS CONTEXT"));
    }
}
```

**Step 2: Add tempfile dev-dependency**

In `crates/telos-experiment/Cargo.toml`, add:

```toml
[dev-dependencies]
tempfile = { workspace = true }
```

**Step 3: Run tests**

Run: `cargo test -p telos-experiment -- --nocapture`
Expected: all PASS

**Step 4: Commit**

```bash
git add crates/telos-experiment/
git commit -m "test: add unit tests for experiment scenario loading and rendering"
```

---

### Task 12: Final Integration — Run Full Test Suite

**Step 1: Run all workspace tests**

Run: `cargo test --workspace`
Expected: all tests PASS (76 existing + new tests)

**Step 2: Run clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: no warnings

**Step 3: Run fmt check**

Run: `cargo fmt --check`
Expected: no formatting issues

**Step 4: Final commit if any fixes needed**

```bash
git add -A
git commit -m "chore: fix clippy warnings and formatting"
```
