use anyhow::{Context, Result};
use chrono::Utc;
use std::env;
use telos_core::object::decision_record::{Alternative, DecisionRecord};
use telos_core::object::intent::Author;
use telos_store::repository::Repository;

pub fn run(
    intent_id_str: String,
    question: String,
    decision: String,
    rationale: Option<String>,
    alternatives_raw: Vec<String>,
    tags: Vec<String>,
) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    // Resolve the intent ID (supports prefix)
    let (intent_oid, obj) = repo
        .read_object(&intent_id_str)
        .context(format!("intent '{}' not found", intent_id_str))?;

    // Verify it's actually an intent
    if obj.type_tag() != "intent" {
        anyhow::bail!("object {} is a {}, not an intent", intent_oid.short(), obj.type_tag());
    }

    // Parse alternative strings into Alternative structs
    let alternatives: Vec<Alternative> = alternatives_raw
        .iter()
        .map(|a| {
            let parts: Vec<&str> = a.splitn(2, '|').collect();
            if parts.len() != 2 {
                anyhow::bail!(
                    "Invalid alternative format: expected 'description|rejection_reason', got '{}'",
                    a
                );
            }
            Ok(Alternative {
                description: parts[0].trim().to_string(),
                rejection_reason: parts[1].trim().to_string(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let record = DecisionRecord {
        intent_id: intent_oid.clone(),
        author: Author {
            name: env::var("TELOS_AUTHOR_NAME").unwrap_or_else(|_| "Anonymous".into()),
            email: env::var("TELOS_AUTHOR_EMAIL").unwrap_or_else(|_| "anonymous@telos".into()),
        },
        timestamp: Utc::now(),
        question,
        decision,
        rationale,
        alternatives,
        tags,
    };

    let id = repo.create_decision(record)?;
    println!("Recorded decision {} for intent {}", id.short(), intent_oid.short());
    Ok(())
}
