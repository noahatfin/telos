use anyhow::{Context, Result};
use std::env;
use telos_core::hash::ObjectId;
use telos_store::query;
use telos_store::repository::Repository;

pub fn intents(impact: Option<String>, constraint_contains: Option<String>, json: bool) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    let results = query::query_intents(
        &repo.odb,
        impact.as_deref(),
        constraint_contains.as_deref(),
    )?;

    if json {
        let entries: Vec<_> = results
            .iter()
            .map(|(id, intent)| {
                serde_json::json!({
                    "id": id.hex(),
                    "object": intent,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if results.is_empty() {
        println!("No matching intents found.");
        return Ok(());
    }

    for (i, (id, intent)) in results.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("intent {}", id.hex());
        println!("Author: {} <{}>", intent.author.name, intent.author.email);
        println!("Date:   {}", intent.timestamp.format("%Y-%m-%d %H:%M:%S %Z"));
        println!();
        println!("    {}", intent.statement);
        if !intent.impacts.is_empty() {
            println!("    Impacts: {}", intent.impacts.join(", "));
        }
        if !intent.constraints.is_empty() {
            for c in &intent.constraints {
                println!("    Constraint: {}", c);
            }
        }
    }

    Ok(())
}

pub fn decisions(intent: Option<String>, tag: Option<String>, json: bool) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    let intent_id = intent
        .as_deref()
        .map(|s| {
            if let Ok(id) = ObjectId::parse(s) {
                Ok(id)
            } else {
                repo.odb.resolve_prefix(s).context(format!("cannot resolve intent '{}'", s))
            }
        })
        .transpose()?;

    let results = query::query_decisions(
        &repo.odb,
        intent_id.as_ref(),
        tag.as_deref(),
    )?;

    if json {
        let entries: Vec<_> = results
            .iter()
            .map(|(id, record)| {
                serde_json::json!({
                    "id": id.hex(),
                    "object": record,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if results.is_empty() {
        println!("No matching decision records found.");
        return Ok(());
    }

    for (i, (id, record)) in results.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("decision_record {}", id.hex());
        println!("Intent:   {}", record.intent_id.short());
        println!("Author:   {} <{}>", record.author.name, record.author.email);
        println!("Date:     {}", record.timestamp.format("%Y-%m-%d %H:%M:%S %Z"));
        println!();
        println!("Question: {}", record.question);
        println!("Decision: {}", record.decision);
        if let Some(rationale) = &record.rationale {
            println!("Rationale: {}", rationale);
        }
        if !record.tags.is_empty() {
            println!("Tags: {}", record.tags.join(", "));
        }
    }

    Ok(())
}

pub fn constraints(
    file: Option<String>,
    symbol: Option<String>,
    impact: Option<String>,
    status: String,
    json: bool,
) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    // If file or symbol is specified, use indexed code-aware queries
    if let Some(ref f) = file {
        let results = query::query_constraints_by_file(&repo.odb, &repo.indexes, f)?;
        return print_constraints(&results, json);
    }
    if let Some(ref s) = symbol {
        let results = query::query_constraints_by_symbol(&repo.odb, &repo.indexes, s)?;
        return print_constraints(&results, json);
    }

    let results = query::query_constraints(
        &repo.odb,
        impact.as_deref(),
        Some(status.as_str()),
    )?;

    print_constraints(&results, json)
}

fn print_constraints(
    results: &[(ObjectId, telos_core::object::constraint::Constraint)],
    json: bool,
) -> Result<()> {
    if json {
        let entries: Vec<_> = results
            .iter()
            .map(|(id, c)| {
                serde_json::json!({
                    "id": id.hex(),
                    "object": c,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if results.is_empty() {
        println!("No matching constraints found.");
        return Ok(());
    }

    for (i, (id, c)) in results.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("constraint {}", id.hex());
        println!("Author:   {} <{}>", c.author.name, c.author.email);
        println!("Date:     {}", c.timestamp.format("%Y-%m-%d %H:%M:%S %Z"));
        println!("Severity: {:?}", c.severity);
        println!("Status:   {:?}", c.status);
        println!();
        println!("    {}", c.statement);
        if !c.impacts.is_empty() {
            println!("    Impacts: {}", c.impacts.join(", "));
        }
    }

    Ok(())
}

pub fn agent_ops(agent: Option<String>, session: Option<String>, json: bool) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    let results = query::query_agent_operations(
        &repo.odb,
        agent.as_deref(),
        session.as_deref(),
    )?;

    if json {
        let entries: Vec<_> = results
            .iter()
            .map(|(id, op)| {
                serde_json::json!({
                    "id": id.hex(),
                    "object": op,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if results.is_empty() {
        println!("No matching agent operations found.");
        return Ok(());
    }

    for (i, (id, op)) in results.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("agent_operation {}", id.hex());
        println!("Agent:    {}", op.agent_id);
        println!("Session:  {}", op.session_id);
        println!("Date:     {}", op.timestamp.format("%Y-%m-%d %H:%M:%S %Z"));
        println!("Op:       {:?}", op.operation);
        println!("Result:   {:?}", op.result);
        println!();
        println!("    {}", op.summary);
        if !op.files_touched.is_empty() {
            println!("    Files: {}", op.files_touched.join(", "));
        }
    }

    Ok(())
}
