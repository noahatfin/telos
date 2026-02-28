use anyhow::{Context, Result};
use std::env;
use telos_store::repository::Repository;

pub fn run() -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    println!("Rebuilding indexes...");
    let (impact_count, path_count, sym_count) = repo.indexes.rebuild_all(&repo.odb)?;
    println!("  impact tags:  {} entries", impact_count);
    println!("  code paths:   {} entries", path_count);
    println!("  symbols:      {} entries", sym_count);
    println!("Done.");
    Ok(())
}
