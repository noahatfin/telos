use anyhow::{Context, Result};
use std::env;
use telos_store::repository::Repository;

pub fn run() -> Result<()> {
    let cwd = env::current_dir().context("failed to get current directory")?;
    Repository::init(&cwd).context("failed to initialize repository")?;
    println!("Initialized empty Telos repository in {}", cwd.join(".telos").display());
    Ok(())
}
