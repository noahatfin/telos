use anyhow::{Context, Result};
use chrono::Utc;
use std::env;
use telos_core::object::intent_stream::IntentStreamRef;
use telos_store::repository::Repository;

pub fn create(name: String) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    // New stream starts at the same tip as the current stream
    let current = repo.refs.current_stream()?;

    let stream = IntentStreamRef {
        name: name.clone(),
        tip: current.tip,
        created_at: Utc::now(),
        description: None,
    };
    repo.refs
        .create_stream(&stream)
        .context("failed to create stream")?;
    println!("Created stream '{}'", name);
    Ok(())
}

pub fn list() -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;
    let head = repo.refs.read_head()?;
    let streams = repo.refs.list_streams()?;

    for name in streams {
        if name == head {
            println!("* {}", name);
        } else {
            println!("  {}", name);
        }
    }
    Ok(())
}

pub fn switch(name: String) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    // Verify the stream exists
    repo.refs
        .read_stream(&name)
        .context(format!("stream '{}' not found", name))?;

    repo.refs.set_head(&name)?;
    println!("Switched to stream '{}'", name);
    Ok(())
}

pub fn delete(name: String) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;
    repo.refs
        .delete_stream(&name)
        .context("failed to delete stream")?;
    println!("Deleted stream '{}'", name);
    Ok(())
}
