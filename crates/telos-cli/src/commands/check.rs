use anyhow::{Context, Result};
use std::env;
use telos_core::object::TelosObject;
use telos_store::repository::Repository;

pub fn run(bindings: bool, all: bool) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    if bindings || all {
        let all_objects = repo.odb.iter_all()?;
        let mut ok_count = 0;
        let mut unresolved_count = 0;

        println!("Checking bindings...");
        for (_id, obj) in &all_objects {
            if let TelosObject::CodeBinding(cb) = obj {
                let full_path = repo.root().join(&cb.path);
                if full_path.exists() {
                    ok_count += 1;
                } else {
                    println!("  UNRESOLVED  {}  (file not found)", cb.path);
                    unresolved_count += 1;
                }
            }
        }
        println!("  OK          {} bindings resolved", ok_count);
        if unresolved_count > 0 {
            println!("  UNRESOLVED  {} bindings unresolved", unresolved_count);
        }
    }

    if !bindings && !all {
        println!("Nothing to check. Use --bindings or --all.");
    }

    Ok(())
}
