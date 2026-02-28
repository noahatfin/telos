use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::env;
use telos_core::hash::ObjectId;
use telos_core::object::constraint::{Constraint, ConstraintSeverity, ConstraintStatus};
use telos_core::object::intent::Author;
use telos_store::repository::Repository;

pub fn run(
    statement: String,
    severity: String,
    impacts: Vec<String>,
    _scope_files: Vec<String>,
) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    let sev = match severity.to_lowercase().as_str() {
        "must" => ConstraintSeverity::Must,
        "should" => ConstraintSeverity::Should,
        "prefer" => ConstraintSeverity::Prefer,
        _ => ConstraintSeverity::Should,
    };

    let author_name = env::var("TELOS_AUTHOR_NAME").unwrap_or_else(|_| "Unknown".into());
    let author_email = env::var("TELOS_AUTHOR_EMAIL").unwrap_or_else(|_| "unknown@unknown".into());

    // Get current stream tip as source_intent (or use a dummy if no intents yet)
    let source = repo
        .refs
        .current_stream()?
        .tip
        .unwrap_or_else(|| ObjectId::hash(b"no-intent"));

    let constraint = Constraint {
        author: Author {
            name: author_name,
            email: author_email,
        },
        timestamp: Utc::now(),
        statement,
        severity: sev,
        status: ConstraintStatus::Active,
        source_intent: source,
        superseded_by: None,
        deprecation_reason: None,
        scope: vec![],
        impacts,
        metadata: HashMap::new(),
    };

    let id = repo.create_constraint(constraint)?;
    println!("Created constraint {}", id.short());
    Ok(())
}
