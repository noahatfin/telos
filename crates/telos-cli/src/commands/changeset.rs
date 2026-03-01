use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::env;
use std::process::Command;
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
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd)?;

    // Resolve git commit SHA
    let git_commit = if commit == "HEAD" {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&cwd)
            .output()?;
        if !output.status.success() {
            anyhow::bail!("failed to resolve HEAD: not a git repository or no commits");
        }
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
            name: env::var("TELOS_AUTHOR_NAME").unwrap_or_else(|_| "telos-cli".into()),
            email: env::var("TELOS_AUTHOR_EMAIL").unwrap_or_default(),
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
        println!(
            "Created changeset {} for commit {}",
            id.short(),
            &git_commit[..8.min(git_commit.len())]
        );
    }
    Ok(())
}

/// Show a changeset by ID.
pub fn show(id: String, json: bool) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd)?;
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
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd)?;

    let sha = if commit_sha == "HEAD" {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&cwd)
            .output()?;
        if !output.status.success() {
            anyhow::bail!("failed to resolve HEAD: not a git repository or no commits");
        }
        String::from_utf8(output.stdout)?.trim().to_string()
    } else {
        commit_sha
    };

    let results = query::query_changesets(&repo.odb, &repo.indexes, Some(&sha), None)?;

    if results.is_empty() {
        eprintln!(
            "No changeset found for commit {}",
            &sha[..8.min(sha.len())]
        );
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
            println!(
                "ChangeSet {} -> commit {}",
                id,
                &cs.git_commit[..8.min(cs.git_commit.len())]
            );
            println!("  Intents: {}", cs.intents.len());
            println!("  Constraints: {}", cs.constraints.len());
            println!("  Decisions: {}", cs.decisions.len());
        }
    }
    Ok(())
}
