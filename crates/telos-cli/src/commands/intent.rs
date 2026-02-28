use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::env;
use telos_core::object::intent::{Author, BehaviorClause, Intent};
use telos_store::repository::Repository;

pub fn run(statement: String, constraints: Vec<String>, impacts: Vec<String>, behaviors: Vec<String>) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository (or any parent)")?;

    // Get current stream tip as parent
    let current = repo.refs.current_stream()?;
    let parents = current.tip.into_iter().collect();

    // Parse behavior strings into BehaviorClause structs
    let behavior_spec: Vec<BehaviorClause> = behaviors
        .iter()
        .map(|b| {
            let parts: Vec<&str> = b.splitn(3, '|').collect();
            if parts.len() != 3 {
                anyhow::bail!(
                    "Invalid behavior format: expected 'GIVEN x|WHEN y|THEN z', got '{}'",
                    b
                );
            }
            Ok(BehaviorClause {
                given: parts[0].trim_start_matches("GIVEN ").trim().to_string(),
                when: parts[1].trim_start_matches("WHEN ").trim().to_string(),
                then: parts[2].trim_start_matches("THEN ").trim().to_string(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let intent = Intent {
        author: Author {
            name: env::var("TELOS_AUTHOR_NAME").unwrap_or_else(|_| "Anonymous".into()),
            email: env::var("TELOS_AUTHOR_EMAIL").unwrap_or_else(|_| "anonymous@telos".into()),
        },
        timestamp: Utc::now(),
        statement,
        constraints,
        behavior_spec,
        parents,
        impacts,
        behavior_diff: None,
        metadata: HashMap::new(),
    };

    let id = repo.create_intent(intent)?;
    let stream_name = repo.refs.read_head()?;
    println!("[{}] {}", stream_name, id.short());
    Ok(())
}
