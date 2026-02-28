use anyhow::{Context, Result};
use std::env;
use telos_core::object::TelosObject;
use telos_store::repository::Repository;

pub fn run(id: String, json: bool) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    let (oid, obj) = repo
        .read_object(&id)
        .context(format!("object '{}' not found", id))?;

    if json {
        let output = serde_json::json!({
            "id": oid.hex(),
            "object": obj,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    match obj {
        TelosObject::Intent(intent) => {
            println!("intent {}", oid.hex());
            println!("Author: {} <{}>", intent.author.name, intent.author.email);
            println!("Date:   {}", intent.timestamp.format("%Y-%m-%d %H:%M:%S %Z"));
            if !intent.parents.is_empty() {
                let parent_strs: Vec<_> = intent.parents.iter().map(|p| p.short().to_string()).collect();
                println!("Parents: {}", parent_strs.join(", "));
            }
            println!();
            println!("    {}", intent.statement);

            if !intent.constraints.is_empty() {
                println!();
                println!("Constraints:");
                for c in &intent.constraints {
                    println!("  - {}", c);
                }
            }
            if !intent.behavior_spec.is_empty() {
                println!();
                println!("Behavior spec:");
                for b in &intent.behavior_spec {
                    println!("  GIVEN {}", b.given);
                    println!("  WHEN  {}", b.when);
                    println!("  THEN  {}", b.then);
                    println!();
                }
            }
            if !intent.impacts.is_empty() {
                println!();
                println!("Impacts: {}", intent.impacts.join(", "));
            }
        }
        TelosObject::BehaviorDiff(diff) => {
            println!("behavior_diff {}", oid.hex());
            println!("Intent: {}", diff.intent_id.short());
            println!();
            println!("Changes:");
            for change in &diff.changes {
                println!("  - {}", change.description);
                if let Some(before) = &change.before {
                    println!("    Before: {}", before);
                }
                println!("    After:  {}", change.after);
            }
            println!();
            println!("Impact radius:");
            println!("  Direct: {}", diff.impact.direct.join(", "));
            if !diff.impact.indirect.is_empty() {
                println!("  Indirect: {}", diff.impact.indirect.join(", "));
            }
        }
        TelosObject::IntentStreamSnapshot(snap) => {
            println!("intent_stream_snapshot {}", oid.hex());
            println!("Stream: {}", snap.name);
            println!("Tip:    {}", snap.tip.short());
            println!("Date:   {}", snap.created_at.format("%Y-%m-%d %H:%M:%S %Z"));
            if let Some(desc) = &snap.description {
                println!("Description: {}", desc);
            }
        }
        TelosObject::DecisionRecord(dr) => {
            println!("decision_record {}", oid.hex());
            println!("Intent:   {}", dr.intent_id.short());
            println!("Author:   {} <{}>", dr.author.name, dr.author.email);
            println!("Date:     {}", dr.timestamp.format("%Y-%m-%d %H:%M:%S %Z"));
            println!();
            println!("Question: {}", dr.question);
            println!("Decision: {}", dr.decision);
            if let Some(rationale) = &dr.rationale {
                println!("Rationale: {}", rationale);
            }
            if !dr.alternatives.is_empty() {
                println!();
                println!("Alternatives considered:");
                for alt in &dr.alternatives {
                    println!("  - {} (rejected: {})", alt.description, alt.rejection_reason);
                }
            }
            if !dr.tags.is_empty() {
                println!();
                println!("Tags: {}", dr.tags.join(", "));
            }
        }
    }

    Ok(())
}
