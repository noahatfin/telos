use anyhow::{Context, Result};
use std::env;
use telos_store::query;
use telos_store::repository::Repository;

pub fn run(impact: String, json: bool) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    // Find all intents matching the impact tag
    let intents = query::query_intents(&repo.odb, Some(&impact), None)?;

    if json {
        let mut entries = Vec::new();
        for (intent_id, intent) in &intents {
            let decisions =
                query::query_decisions(&repo.odb, Some(intent_id), None)?;
            let decision_json: Vec<_> = decisions
                .iter()
                .map(|(did, dr)| {
                    serde_json::json!({
                        "id": did.hex(),
                        "object": dr,
                    })
                })
                .collect();
            entries.push(serde_json::json!({
                "intent_id": intent_id.hex(),
                "intent": intent,
                "decisions": decision_json,
            }));
        }
        let output = serde_json::json!({
            "impact": impact,
            "intents": entries,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    if intents.is_empty() {
        println!("No intents found for impact '{}'.", impact);
        return Ok(());
    }

    println!("Context for impact: {}", impact);
    println!("{}", "=".repeat(40));

    for (intent_id, intent) in &intents {
        println!();
        println!("intent {}", intent_id.hex());
        println!("  Statement: {}", intent.statement);
        println!(
            "  Author: {} <{}>",
            intent.author.name, intent.author.email
        );
        println!(
            "  Date:   {}",
            intent.timestamp.format("%Y-%m-%d %H:%M:%S %Z")
        );

        if !intent.constraints.is_empty() {
            println!("  Constraints:");
            for c in &intent.constraints {
                println!("    - {}", c);
            }
        }

        if !intent.behavior_spec.is_empty() {
            println!("  Behavior spec:");
            for b in &intent.behavior_spec {
                println!("    GIVEN {}", b.given);
                println!("    WHEN  {}", b.when);
                println!("    THEN  {}", b.then);
                println!();
            }
        }

        // Show linked decisions
        let decisions =
            query::query_decisions(&repo.odb, Some(intent_id), None)?;
        if !decisions.is_empty() {
            println!("  Decisions:");
            for (did, dr) in &decisions {
                println!("    decision_record {}", did.short());
                println!("      Q: {}", dr.question);
                println!("      A: {}", dr.decision);
                if let Some(rationale) = &dr.rationale {
                    println!("      Rationale: {}", rationale);
                }
                if !dr.tags.is_empty() {
                    println!("      Tags: {}", dr.tags.join(", "));
                }
            }
        }
    }

    Ok(())
}
