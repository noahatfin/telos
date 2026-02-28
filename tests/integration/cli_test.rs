use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn telos() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("telos-cli").unwrap()
}

#[test]
fn init_creates_telos_dir() {
    let dir = TempDir::new().unwrap();
    telos()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains(".telos"));

    assert!(dir.path().join(".telos").exists());
    assert!(dir.path().join(".telos/HEAD").exists());
    assert!(dir.path().join(".telos/objects").exists());
    assert!(dir.path().join(".telos/refs/streams/main").exists());
}

#[test]
fn init_twice_fails() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();
    telos()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn intent_on_empty_stream() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    telos()
        .args(["intent", "-s", "First intent"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("[main]"));
}

#[test]
fn intent_with_constraints_and_impacts() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    telos()
        .args([
            "intent",
            "-s",
            "Add user registration",
            "--constraint",
            "Must validate email",
            "--impact",
            "user-registration",
        ])
        .current_dir(dir.path())
        .assert()
        .success();
}

#[test]
fn log_empty_stream() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    telos()
        .arg("log")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("No intents yet"));
}

#[test]
fn log_shows_intents() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    telos()
        .args(["intent", "-s", "First"])
        .current_dir(dir.path())
        .assert()
        .success();

    telos()
        .args(["intent", "-s", "Second"])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = telos()
        .arg("log")
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Second"));
    assert!(stdout.contains("First"));
    // Second should appear before First (reverse chronological)
    assert!(stdout.find("Second").unwrap() < stdout.find("First").unwrap());
}

#[test]
fn show_intent_by_prefix() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    let output = telos()
        .args(["intent", "-s", "Test intent"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    // Extract the short hash from "[main] abcd1234"
    let short_hash = stdout.trim().split_whitespace().last().unwrap();

    telos()
        .args(["show", short_hash])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Test intent"));
}

#[test]
fn stream_create_list_switch_delete() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    // Create a stream
    telos()
        .args(["stream", "create", "feature"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Created stream 'feature'"));

    // List streams — main should be current
    let output = telos()
        .args(["stream", "list"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("* main"));
    assert!(stdout.contains("  feature"));

    // Switch to feature
    telos()
        .args(["stream", "switch", "feature"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Switched to stream 'feature'"));

    // List again — feature should be current
    let output = telos()
        .args(["stream", "list"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("* feature"));
    assert!(stdout.contains("  main"));

    // Switch back to main
    telos()
        .args(["stream", "switch", "main"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Delete feature
    telos()
        .args(["stream", "delete", "feature"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Deleted stream 'feature'"));
}

#[test]
fn decide_records_decision() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    let output = telos()
        .args(["intent", "-s", "Design auth flow"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let short_hash = stdout.trim().split_whitespace().last().unwrap();

    telos()
        .args([
            "decide",
            "--intent",
            short_hash,
            "--question",
            "Which auth method?",
            "--decision",
            "Use JWT",
            "--rationale",
            "Stateless and scalable",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Recorded decision"));
}

#[test]
fn full_workflow() {
    let dir = TempDir::new().unwrap();

    // Init
    telos().arg("init").current_dir(dir.path()).assert().success();

    // Create intents
    telos()
        .args(["intent", "-s", "建立用户注册流程"])
        .current_dir(dir.path())
        .assert()
        .success();

    telos()
        .args([
            "intent",
            "-s",
            "添加企业邮箱自动识别",
            "--impact",
            "user-registration",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // Log
    let output = telos()
        .arg("log")
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("添加企业邮箱自动识别"));
    assert!(stdout.contains("建立用户注册流程"));

    // Create branch
    telos()
        .args(["stream", "create", "feature/onboarding"])
        .current_dir(dir.path())
        .assert()
        .success();

    // List
    telos()
        .args(["stream", "list"])
        .current_dir(dir.path())
        .assert()
        .success();
}

#[test]
fn log_json_empty_stream() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    let output = telos()
        .args(["log", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed, Value::Array(vec![]));
}

#[test]
fn log_json_outputs_valid_json() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    telos()
        .args(["intent", "-s", "First intent", "--impact", "auth"])
        .current_dir(dir.path())
        .assert()
        .success();

    telos()
        .args(["intent", "-s", "Second intent", "--constraint", "Must be fast"])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = telos()
        .args(["log", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();

    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    // Each entry should have "id" and "object" fields
    for entry in arr {
        assert!(entry.get("id").is_some(), "missing 'id' field");
        assert!(entry["id"].is_string(), "'id' should be a string");
        let obj = entry.get("object").expect("missing 'object' field");
        assert!(obj.get("statement").is_some(), "missing 'statement' in object");
        assert!(obj.get("author").is_some(), "missing 'author' in object");
        assert!(obj.get("timestamp").is_some(), "missing 'timestamp' in object");
    }

    // Second intent should appear first (reverse chronological)
    assert_eq!(arr[0]["object"]["statement"], "Second intent");
    assert_eq!(arr[1]["object"]["statement"], "First intent");
}

#[test]
fn show_json_outputs_valid_json() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    let output = telos()
        .args(["intent", "-s", "JSON show test", "--constraint", "Must work"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let short_hash = stdout.trim().split_whitespace().last().unwrap();

    let output = telos()
        .args(["show", "--json", short_hash])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();

    assert!(parsed.get("id").is_some(), "missing 'id' field");
    assert!(parsed["id"].is_string(), "'id' should be a string");

    let obj = parsed.get("object").expect("missing 'object' field");
    assert_eq!(obj["type"], "intent");
    assert_eq!(obj["statement"], "JSON show test");
    assert!(obj.get("author").is_some(), "missing 'author' in object");
    let constraints = obj["constraints"].as_array().unwrap();
    assert_eq!(constraints.len(), 1);
    assert_eq!(constraints[0], "Must work");
}

#[test]
fn query_intents_by_impact() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    telos()
        .args(["intent", "-s", "Auth setup", "--impact", "auth"])
        .current_dir(dir.path())
        .assert()
        .success();

    telos()
        .args(["intent", "-s", "Task CRUD", "--impact", "tasks"])
        .current_dir(dir.path())
        .assert()
        .success();

    telos()
        .args(["intent", "-s", "Auth tokens", "--impact", "auth", "--impact", "security"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Query by impact=auth should return 2 intents
    let output = telos()
        .args(["query", "intents", "--impact", "auth", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn query_intents_by_constraint() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    telos()
        .args(["intent", "-s", "Auth setup", "--constraint", "Token expiry <= 1 hour"])
        .current_dir(dir.path())
        .assert()
        .success();

    telos()
        .args(["intent", "-s", "Task CRUD", "--constraint", "Must validate input"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Case-insensitive substring search for "token"
    let output = telos()
        .args(["query", "intents", "--constraint-contains", "token", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 1);
}

#[test]
fn query_decisions_by_tag() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    let output = telos()
        .args(["intent", "-s", "Auth setup", "--impact", "auth"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let short_hash = stdout.trim().split_whitespace().last().unwrap();

    telos()
        .args([
            "decide",
            "--intent", short_hash,
            "--question", "Token format?",
            "--decision", "JWT",
            "--tag", "auth",
            "--tag", "security",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    telos()
        .args([
            "decide",
            "--intent", short_hash,
            "--question", "DB choice?",
            "--decision", "Postgres",
            "--tag", "infra",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // Query decisions by tag=auth
    let output = telos()
        .args(["query", "decisions", "--tag", "auth", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["object"]["decision"], "JWT");
}

#[test]
fn context_aggregates_intents_and_decisions() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    // Create an intent with impact "auth"
    let output = telos()
        .args(["intent", "-s", "Auth setup", "--impact", "auth", "--constraint", "Must use HTTPS"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let short_hash = stdout.trim().split_whitespace().last().unwrap();

    // Create a decision linked to this intent
    telos()
        .args([
            "decide",
            "--intent", short_hash,
            "--question", "Token format?",
            "--decision", "JWT",
            "--tag", "auth",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // Create another intent with different impact
    telos()
        .args(["intent", "-s", "Task CRUD", "--impact", "tasks"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Context for "auth" should show auth intent and its decision
    let output = telos()
        .args(["context", "--impact", "auth", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(parsed["impact"], "auth");
    let intents = parsed["intents"].as_array().unwrap();
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0]["intent"]["statement"], "Auth setup");

    let decisions = intents[0]["decisions"].as_array().unwrap();
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0]["object"]["decision"], "JWT");
}

#[test]
fn query_intents_no_results() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    telos()
        .args(["intent", "-s", "Something", "--impact", "tasks"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Query with non-matching impact
    telos()
        .args(["query", "intents", "--impact", "nonexistent"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("No matching intents"));
}

#[test]
fn intent_with_behavior_spec() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    let output = telos()
        .args([
            "intent",
            "-s",
            "User login flow",
            "--behavior",
            "GIVEN a registered user|WHEN they enter valid credentials|THEN they are authenticated",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let short_hash = stdout.trim().split_whitespace().last().unwrap();

    // Verify behavior is displayed in show output
    telos()
        .args(["show", short_hash])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Behavior spec:"))
        .stdout(predicates::str::contains("GIVEN a registered user"))
        .stdout(predicates::str::contains("WHEN  they enter valid credentials"))
        .stdout(predicates::str::contains("THEN  they are authenticated"));
}

#[test]
fn decide_with_alternatives_and_tags() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    let output = telos()
        .args(["intent", "-s", "Design auth"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let intent_hash = stdout.trim().split_whitespace().last().unwrap();

    let output = telos()
        .args([
            "decide",
            "--intent",
            intent_hash,
            "--question",
            "Which auth method?",
            "--decision",
            "Use JWT",
            "--alternative",
            "Session cookies|Doesn't scale across services",
            "--alternative",
            "OAuth only|Too complex for MVP",
            "--tag",
            "auth",
            "--tag",
            "security",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Recorded decision"));

    // Extract decision hash from output "Recorded decision XXXX for intent YYYY"
    let decision_hash = stdout.trim().split_whitespace().nth(2).unwrap();

    // Verify alternatives and tags are displayed in show output
    let show_output = telos()
        .args(["show", decision_hash])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let show_stdout = String::from_utf8(show_output.stdout).unwrap();
    assert!(show_stdout.contains("Alternatives considered:"));
    assert!(show_stdout.contains("Session cookies"));
    assert!(show_stdout.contains("Doesn't scale across services"));
    assert!(show_stdout.contains("OAuth only"));
    assert!(show_stdout.contains("Too complex for MVP"));
    assert!(show_stdout.contains("Tags: auth, security"));
}

#[test]
fn context_no_results() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    telos()
        .args(["context", "--impact", "nonexistent"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("No intents found"));
}

// ========== v2 Integration Tests ==========

#[test]
fn constraint_create_and_query() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    // Create an intent first (needed for source_intent)
    telos()
        .args(["intent", "-s", "Auth system", "--impact", "auth"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Create a constraint
    telos()
        .args([
            "constraint",
            "-s", "All API endpoints must use HTTPS",
            "--severity", "must",
            "--impact", "security",
            "--impact", "auth",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Created constraint"));

    // Query constraints
    let output = telos()
        .args(["query", "constraints", "--impact", "security", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["object"]["statement"], "All API endpoints must use HTTPS");
    assert_eq!(arr[0]["object"]["severity"], "must");
    assert_eq!(arr[0]["object"]["status"], "active");
}

#[test]
fn constraint_supersede() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    telos()
        .args(["intent", "-s", "Security policy", "--impact", "security"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Create original constraint
    let output = telos()
        .args([
            "constraint",
            "-s", "Passwords must be >= 8 chars",
            "--severity", "must",
            "--impact", "security",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let constraint_hash = stdout.trim().split_whitespace().last().unwrap().to_string();

    // Supersede it
    telos()
        .args([
            "supersede",
            &constraint_hash,
            "-s", "Passwords must be >= 12 chars",
            "--severity", "must",
            "--reason", "Updated security policy requires longer passwords",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Superseded"));

    // Query active constraints — should show the new one, not the old
    let output = telos()
        .args(["query", "constraints", "--impact", "security", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();

    // Should have the new active constraint
    let stmts: Vec<&str> = arr
        .iter()
        .map(|e| e["object"]["statement"].as_str().unwrap())
        .collect();
    assert!(stmts.contains(&"Passwords must be >= 12 chars"));

    // Query superseded — should show the old one
    let output = telos()
        .args(["query", "constraints", "--status", "superseded", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();
    assert!(!arr.is_empty());
    let old_stmts: Vec<&str> = arr
        .iter()
        .map(|e| e["object"]["statement"].as_str().unwrap())
        .collect();
    assert!(old_stmts.contains(&"Passwords must be >= 8 chars"));
}

#[test]
fn constraint_deprecate() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    telos()
        .args(["intent", "-s", "Legacy feature"])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = telos()
        .args([
            "constraint",
            "-s", "Must support IE11",
            "--severity", "must",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let constraint_hash = stdout.trim().split_whitespace().last().unwrap().to_string();

    // Deprecate
    telos()
        .args([
            "deprecate",
            &constraint_hash,
            "--reason", "IE11 is end of life",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Deprecated constraint"));

    // Query deprecated
    let output = telos()
        .args(["query", "constraints", "--status", "deprecated", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();
    assert!(!arr.is_empty());
    assert_eq!(arr[0]["object"]["status"], "deprecated");
}

#[test]
fn code_binding_create_and_show() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    // Create an intent to bind to
    let output = telos()
        .args(["intent", "-s", "Auth module", "--impact", "auth"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let intent_hash = stdout.trim().split_whitespace().last().unwrap().to_string();

    // Create a code binding — output is "Created binding XXXX for object YYYY"
    let output = telos()
        .args([
            "bind",
            &intent_hash,
            "--file", "src/auth/mod.rs",
            "--symbol", "validate_token",
            "--type", "function",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Created binding"));
    // Extract binding hash (3rd word: "Created binding XXXX ...")
    let binding_hash = stdout.trim().split_whitespace().nth(2).unwrap().to_string();

    telos()
        .args(["show", &binding_hash])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("code_binding"))
        .stdout(predicates::str::contains("src/auth/mod.rs"))
        .stdout(predicates::str::contains("validate_token"));
}

#[test]
fn agent_log_create_and_query() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    // Log an agent operation
    telos()
        .args([
            "agent-log",
            "--agent", "claude-review",
            "--session", "sess-001",
            "--operation", "review",
            "--summary", "Reviewed auth module for security issues",
            "--file", "src/auth/mod.rs",
            "--file", "src/auth/token.rs",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Logged agent operation"));

    // Log another with different agent
    telos()
        .args([
            "agent-log",
            "--agent", "copilot-gen",
            "--session", "sess-002",
            "--operation", "generate",
            "--summary", "Generated test cases",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // Query by agent
    let output = telos()
        .args(["query", "agent-ops", "--agent", "claude-review", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["object"]["agent_id"], "claude-review");
    assert_eq!(arr[0]["object"]["summary"], "Reviewed auth module for security issues");
}

#[test]
fn reindex_rebuilds_indexes() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    // Create some objects
    telos()
        .args(["intent", "-s", "Auth system", "--impact", "auth"])
        .current_dir(dir.path())
        .assert()
        .success();

    telos()
        .args(["constraint", "-s", "Must use HTTPS", "--impact", "security"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Delete indexes to simulate corruption
    std::fs::remove_dir_all(dir.path().join(".telos/indexes")).unwrap();

    // Reindex
    telos()
        .args(["reindex"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Rebuilding indexes"))
        .stdout(predicates::str::contains("Done"));

    // Verify indexes are back
    assert!(dir.path().join(".telos/indexes").exists());
}

#[test]
fn check_bindings() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    let output = telos()
        .args(["intent", "-s", "Test binding check"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let intent_hash = stdout.trim().split_whitespace().last().unwrap().to_string();

    // Bind to a file that doesn't exist
    telos()
        .args([
            "bind", &intent_hash,
            "--file", "src/nonexistent.rs",
            "--type", "file",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // Check bindings — should report unresolved
    telos()
        .args(["check", "--bindings"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("UNRESOLVED"));
}

#[test]
fn show_constraint_details() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    telos()
        .args(["intent", "-s", "Security policy", "--impact", "security"])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = telos()
        .args([
            "constraint",
            "-s", "All endpoints require authentication",
            "--severity", "must",
            "--impact", "security",
            "--impact", "auth",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let constraint_hash = stdout.trim().split_whitespace().last().unwrap().to_string();

    // Show constraint
    telos()
        .args(["show", &constraint_hash])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("constraint"))
        .stdout(predicates::str::contains("All endpoints require authentication"))
        .stdout(predicates::str::contains("Must"))
        .stdout(predicates::str::contains("Active"))
        .stdout(predicates::str::contains("security, auth"));
}

#[test]
fn show_agent_operation_details() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    let output = telos()
        .args([
            "agent-log",
            "--agent", "claude-review",
            "--session", "s1",
            "--operation", "review",
            "--summary", "Full security audit",
            "--file", "src/main.rs",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let op_hash = stdout.trim().split_whitespace().last().unwrap().to_string();

    telos()
        .args(["show", &op_hash])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("agent_operation"))
        .stdout(predicates::str::contains("claude-review"))
        .stdout(predicates::str::contains("Full security audit"))
        .stdout(predicates::str::contains("src/main.rs"));
}

#[test]
fn full_v2_workflow() {
    let dir = TempDir::new().unwrap();
    telos().arg("init").current_dir(dir.path()).assert().success();

    // 1. Create an intent
    let output = telos()
        .args([
            "intent",
            "-s", "Implement user authentication",
            "--impact", "auth",
            "--impact", "security",
            "--constraint", "Must support OAuth2",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let intent_hash = stdout.trim().split_whitespace().last().unwrap().to_string();

    // 2. Create standalone constraints
    telos()
        .args([
            "constraint",
            "-s", "Token expiry must be <= 1 hour",
            "--severity", "must",
            "--impact", "auth",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    telos()
        .args([
            "constraint",
            "-s", "Prefer bcrypt for password hashing",
            "--severity", "prefer",
            "--impact", "security",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // 3. Make a decision
    telos()
        .args([
            "decide",
            "--intent", &intent_hash,
            "--question", "Which OAuth2 library?",
            "--decision", "Use passport.js",
            "--tag", "auth",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // 4. Create code bindings
    telos()
        .args([
            "bind", &intent_hash,
            "--file", "src/auth/mod.rs",
            "--type", "module",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    telos()
        .args([
            "bind", &intent_hash,
            "--file", "src/auth/oauth.rs",
            "--symbol", "handle_callback",
            "--type", "function",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // 5. Log agent operations
    telos()
        .args([
            "agent-log",
            "--agent", "claude-review",
            "--session", "review-001",
            "--operation", "review",
            "--summary", "Reviewed OAuth2 implementation",
            "--file", "src/auth/oauth.rs",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // 6. Query constraints by impact
    let output = telos()
        .args(["query", "constraints", "--impact", "auth", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1); // Token expiry

    // 7. Query agent ops
    let output = telos()
        .args(["query", "agent-ops", "--agent", "claude-review", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);

    // 8. Reindex
    telos()
        .args(["reindex"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Done"));

    // 9. Context should aggregate everything for auth
    let output = telos()
        .args(["context", "--impact", "auth", "--json"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["impact"], "auth");
    assert!(!parsed["intents"].as_array().unwrap().is_empty());
}
