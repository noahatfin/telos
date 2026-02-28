use anyhow::{Context, Result};
use chrono::Utc;
use std::env;
use telos_core::object::constraint::ConstraintStatus;
use telos_core::object::intent::Author;
use telos_core::object::TelosObject;
use telos_store::repository::Repository;

pub fn run(constraint_id: String, reason: String) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    // Resolve constraint_id
    let (oid, obj) = repo
        .read_object(&constraint_id)
        .context(format!("constraint '{}' not found", constraint_id))?;

    let constraint = match obj {
        TelosObject::Constraint(c) => c,
        other => anyhow::bail!(
            "object {} is a {}, not a constraint",
            oid.short(),
            other.type_tag()
        ),
    };

    let author_name = env::var("TELOS_AUTHOR_NAME").unwrap_or_else(|_| "Unknown".into());
    let author_email = env::var("TELOS_AUTHOR_EMAIL").unwrap_or_else(|_| "unknown@unknown".into());

    // Create a new constraint object with Deprecated status
    let mut deprecated = constraint;
    deprecated.author = Author {
        name: author_name,
        email: author_email,
    };
    deprecated.timestamp = Utc::now();
    deprecated.status = ConstraintStatus::Deprecated;
    deprecated.deprecation_reason = Some(reason);

    let new_id = repo.create_constraint(deprecated)?;

    println!(
        "Deprecated constraint {} -> {}",
        oid.short(),
        new_id.short()
    );
    Ok(())
}
