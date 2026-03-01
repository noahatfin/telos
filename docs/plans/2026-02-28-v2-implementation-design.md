# Telos v2 Implementation Design

**Date:** 2026-02-28
**Status:** Approved
**Scope:** Security hardening, ChangeSet, LLM experiment framework

---

## Overview

Three parallel workstreams to advance Telos from prototype to validated tool:

- **WF1: Security Hardening** — Fix P0/P1 issues identified by Codex review
- **WF2: ChangeSet** — Bridge Git commits to Telos reasoning chains
- **WF3: Experiment Framework** — Real LLM validation via codex CLI

---

## WF1: Security Hardening

### 1.1 Stream Name Path Traversal Protection

- `RefStore` adds `validate_stream_name()` — rejects names containing `/`, `..`, `\0`, empty strings, or leading `.`
- CLI retains input validation as early-fail optimization
- Regression tests: `../../etc/passwd`, `foo/bar`, `.hidden`, `\0evil`

### 1.2 Object Read Hash Verification

- `ObjectDatabase::read()` recomputes SHA-256 after reading, compares to filename
- Mismatch returns `TelosError::IntegrityError { expected, actual }`
- New error variant in `telos-store/src/error.rs`

### 1.3 `iter_all` Error Handling

- Current: silently skips corrupted objects
- New: returns `(Vec<TelosObject>, Vec<CorruptedObject>)` where `CorruptedObject` contains path and error
- CLI query/context commands output warnings to stderr when corrupted objects found

### 1.4 Store Layer Integrity Validation

- `Repository::create_decision()` validates `intent_id` exists and is type Intent
- `Repository::create_intent()` validates `parents` exist and are type Intent
- Unified `Repository::create_constraint()`, `create_code_binding()`, `create_agent_operation()` with type-correct reference validation

---

## WF2: ChangeSet Implementation

### 2.1 Data Model

```rust
pub struct ChangeSet {
    pub author: Author,
    pub timestamp: DateTime<Utc>,
    pub git_commit: String,
    pub parents: Vec<ObjectId>,
    pub intents: Vec<ObjectId>,
    pub constraints: Vec<ObjectId>,
    pub decisions: Vec<ObjectId>,
    pub code_bindings: Vec<ObjectId>,
    pub agent_operations: Vec<ObjectId>,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

### 2.2 Core Layer

- `object/change_set.rs` — already exists, verify serialization/deserialization/hashing
- `TelosObject` enum includes `ChangeSet` variant
- Unit tests: round-trip, content_id determinism, type_tag correctness

### 2.3 Store Layer

- `Repository::create_changeset()` — validates all referenced IDs exist with correct types
- `Repository::find_changeset_by_commit()` — lookup by git commit SHA
- `IndexStore` adds `commits.json` index: `git_commit_sha -> ObjectId`
- `query.rs` adds `query_changesets()`: filter by git commit, impact, time range

### 2.4 CLI Layer

```bash
telos changeset create --commit HEAD --intent abc123 --constraint def456
telos changeset show <id> [--json]
telos changeset for-commit <git-sha> [--json]
```

- `telos log` enhanced: shows linked git commit when ChangeSet exists

### 2.5 Tests

- Core: serialization round-trip, hash determinism
- Store: create/query/index/reference validation
- CLI integration: create -> show -> for-commit -> log full workflow

### 2.6 Out of Scope

- No git hook auto-creation of ChangeSets
- No ChangeSet conflict detection
- No ChangeSet sync with `git rebase`

---

## WF3: Experiment Framework (`telos-experiment`)

### 3.1 Crate Structure

```
crates/telos-experiment/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── scenario.rs      # Scenario definition and loading
│   ├── codex.rs          # Codex CLI invocation wrapper
│   ├── runner.rs         # Experiment execution engine
│   ├── scorer.rs         # LLM-as-judge scoring
│   ├── report.rs         # Result aggregation and statistics
│   └── main.rs           # Binary: telos-experiment
└── scenarios/
    ├── constraint_violation.toml
    ├── status_validation_removal.toml
    ├── info_leak.toml
    ├── privilege_escalation.toml
    ├── benign_refactor.toml
    ├── benign_performance.toml
    └── benign_bugfix.toml
```

### 3.2 Scenario Definition (TOML)

```toml
[scenario]
name = "token_expiry_violation"
category = "true_positive"
description = "Token expiry changed from 1h to 24h with plausible commit message"

[diff]
content = """
- const TOKEN_EXPIRY_SECS: u64 = 3600;
+ const TOKEN_EXPIRY_SECS: u64 = 86400;
"""
commit_message = "Increase token expiry for better user experience"

[context]
git_only = "git log output..."
constraints_md = "- Token expiry must be <= 1 hour for security"
telos_json = '{"constraints": [{"statement": "Token expiry must be <= 1 hour", "severity": "Must", "status": "Active"}]}'

[prompt]
template = """
Review the following code change. Identify any issues, constraint violations, or security concerns.
If you find violations, recommend rejection. If the change is safe, approve it.

Commit message: {{commit_message}}

Diff:
{{diff}}

Available project context:
{{context}}
"""

[expected]
should_reject = true
key_findings = ["token expiry exceeds 1 hour limit", "security constraint violation"]
```

### 3.3 Codex CLI Wrapper

```rust
pub struct CodexRunner {
    pub binary: String,         // default "codex"
    pub model: Option<String>,
    pub timeout_secs: u64,      // default 120
}

impl CodexRunner {
    pub fn run(&self, prompt: &str) -> Result<CodexResponse>;
}
```

Invokes `codex -q --prompt "..."` via `std::process::Command`, captures stdout, enforces timeout.

### 3.4 Experiment Runner

Three conditions per scenario: `GitOnly`, `ConstraintsMd`, `Telos`.
Each condition runs N times (default 5). Prompt template is identical across conditions — only `{{context}}` differs.

### 3.5 LLM-as-Judge Scoring

Second codex call scores each response:

```rust
pub struct Score {
    pub caught_issue: bool,
    pub recommended_rejection: bool,
    pub cited_constraint: bool,
    pub reasoning_quality: u8,  // 1-5
    pub judge_explanation: String,
}
```

Judge prompt asks codex to evaluate the reviewer response against expected findings and return structured JSON scores.

### 3.6 Report Generation

Human-readable table and `--json` output. Shows per-scenario breakdown across conditions, plus aggregate false positive rate. Results saved to `.telos-experiment/results/`.

### 3.7 CLI Commands

```bash
telos-experiment run --repeats 5
telos-experiment run --scenario constraint_violation --repeats 10
telos-experiment run --condition telos --condition git-only
telos-experiment report [--json | --latest]
telos-experiment list
```

### 3.8 Scenario Matrix

| # | Scenario | Category | Tests |
|---|----------|----------|-------|
| 1 | Token expiry 24x increase | true_positive | Numeric constraint violation |
| 2 | Status validation removal | true_positive | Logic guard removal |
| 3 | Error message info leak | true_positive | Security info disclosure |
| 4 | Default role escalation to Admin | true_positive | Privilege escalation |
| 5 | Normal performance refactor | false_positive | Should NOT reject |
| 6 | Normal bug fix | false_positive | Should NOT reject |
| 7 | Normal feature addition | false_positive | Should NOT reject |

### 3.9 Methodology Fixes vs v1

| v1 Problem | v2 Solution |
|------------|-------------|
| Hand-written simulated responses | Real codex CLI output |
| Prompt asymmetry (E5) | Identical prompt template, only context differs |
| No false-positive testing (E4) | 3 benign scenarios |
| Keyword scoring (E2) | LLM-as-judge scoring |
| N=1 per experiment (E1) | N>=5 with variance |
| No CONSTRAINTS.md comparison (S6) | Three-way: Git-only vs CONSTRAINTS.md vs Telos |

### 3.10 Out of Scope

- No embedding/semantic search scoring
- No CI automation
- No multi-LLM backend support
- No interactive scenario editor

---

## Parallel Execution Plan

| Workstream | Agent | Dependencies |
|------------|-------|-------------|
| WF1: Security Hardening | Agent 1 | None (independent) |
| WF2: ChangeSet | Agent 2 | None (independent) |
| WF3: Experiment Framework | Agent 3 | None (independent) |

All three workstreams operate on different files/crates and can run in parallel via git worktrees.
