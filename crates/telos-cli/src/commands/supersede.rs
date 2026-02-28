use anyhow::{Context, Result};
use chrono::Utc;
use std::env;
use telos_core::object::constraint::{ConstraintSeverity, ConstraintStatus};
use telos_core::object::intent::Author;
use telos_core::object::TelosObject;
use telos_store::repository::Repository;

pub fn run(
    old_id: String,
    statement: String,
    severity: String,
    reason: Option<String>,
) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    // Resolve old_id to full ObjectId
    let (old_oid, old_obj) = repo
        .read_object(&old_id)
        .context(format!("constraint '{}' not found", old_id))?;

    let old_constraint = match old_obj {
        TelosObject::Constraint(c) => c,
        other => anyhow::bail!(
            "object {} is a {}, not a constraint",
            old_oid.short(),
            other.type_tag()
        ),
    };

    if old_constraint.status != ConstraintStatus::Active {
        anyhow::bail!(
            "constraint {} is not active (status: {:?})",
            old_oid.short(),
            old_constraint.status
        );
    }

    let sev = match severity.to_lowercase().as_str() {
        "must" => ConstraintSeverity::Must,
        "should" => ConstraintSeverity::Should,
        "prefer" => ConstraintSeverity::Prefer,
        _ => ConstraintSeverity::Should,
    };

    let author_name = env::var("TELOS_AUTHOR_NAME").unwrap_or_else(|_| "Unknown".into());
    let author_email = env::var("TELOS_AUTHOR_EMAIL").unwrap_or_else(|_| "unknown@unknown".into());

    // Create the new replacement constraint
    let mut new_constraint = old_constraint.clone();
    new_constraint.author = Author {
        name: author_name,
        email: author_email,
    };
    new_constraint.timestamp = Utc::now();
    new_constraint.statement = statement;
    new_constraint.severity = sev;
    new_constraint.status = ConstraintStatus::Active;
    new_constraint.superseded_by = None;
    new_constraint.deprecation_reason = None;

    let new_id = repo.create_constraint(new_constraint)?;

    // Write a superseded copy of the old constraint pointing to the new one
    let mut superseded = old_constraint;
    superseded.status = ConstraintStatus::Superseded;
    superseded.superseded_by = Some(new_id.clone());
    if let Some(r) = reason {
        superseded.deprecation_reason = Some(r);
    }

    let superseded_id = repo.create_constraint(superseded)?;

    println!(
        "Superseded {} -> {} (superseded record: {})",
        old_oid.short(),
        new_id.short(),
        superseded_id.short()
    );
    Ok(())
}
