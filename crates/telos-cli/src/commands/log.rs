use anyhow::{Context, Result};
use std::env;
use telos_store::repository::Repository;

pub fn run(max_count: usize, json: bool) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    let current = repo.refs.current_stream()?;
    let tip = match current.tip {
        Some(tip) => tip,
        None => {
            if json {
                println!("[]");
            } else {
                println!("No intents yet on stream '{}'", current.name);
            }
            return Ok(());
        }
    };

    if json {
        let mut entries = Vec::new();
        for (count, result) in repo.walk_intents(&tip).enumerate() {
            if count >= max_count {
                break;
            }
            let (id, intent) = result.context("failed to read intent")?;
            entries.push(serde_json::json!({
                "id": id.hex(),
                "object": intent,
            }));
        }
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        for (count, result) in repo.walk_intents(&tip).enumerate() {
            if count >= max_count {
                break;
            }
            let (id, intent) = result.context("failed to read intent")?;

            if count > 0 {
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
    }

    Ok(())
}
