use anyhow::{Context, Result};
use std::collections::HashMap;
use std::env;
use telos_core::object::code_binding::{BindingResolution, BindingType, CodeBinding};
use telos_store::repository::Repository;

pub fn run(
    object_id: String,
    file: String,
    symbol: Option<String>,
    binding_type: String,
) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    // Resolve object_id
    let (oid, _obj) = repo
        .read_object(&object_id)
        .context(format!("object '{}' not found", object_id))?;

    let bt = match binding_type.to_lowercase().as_str() {
        "file" => BindingType::File,
        "function" => BindingType::Function,
        "module" => BindingType::Module,
        "api" => BindingType::Api,
        "type" => BindingType::Type,
        other => anyhow::bail!(
            "unknown binding type '{}' (expected: file, function, module, api, type)",
            other
        ),
    };

    let binding = CodeBinding {
        path: file,
        symbol,
        span: None,
        binding_type: bt,
        resolution: BindingResolution::Unchecked,
        bound_object: oid.clone(),
        metadata: HashMap::new(),
    };

    let id = repo.create_code_binding(binding)?;
    println!(
        "Created binding {} for object {}",
        id.short(),
        oid.short()
    );
    Ok(())
}
