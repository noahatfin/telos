use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::env;
use telos_core::hash::ObjectId;
use telos_core::object::agent_operation::{AgentOperation, OperationResult, OperationType};
use telos_store::repository::Repository;

pub fn run(
    agent: String,
    session: String,
    operation: String,
    summary: String,
    context_refs: Vec<String>,
    files: Vec<String>,
) -> Result<()> {
    let cwd = env::current_dir()?;
    let repo = Repository::discover(&cwd).context("not a Telos repository")?;

    let op_type = match operation.to_lowercase().as_str() {
        "review" => OperationType::Review,
        "generate" => OperationType::Generate,
        "decide" => OperationType::Decide,
        "query" => OperationType::Query,
        "violation" => OperationType::Violation,
        other => OperationType::Custom(other.to_string()),
    };

    // Resolve context refs to ObjectIds
    let refs: Vec<ObjectId> = context_refs
        .iter()
        .map(|r| {
            let (oid, _) = repo
                .read_object(r)
                .context(format!("context ref '{}' not found", r))?;
            Ok(oid)
        })
        .collect::<Result<Vec<_>>>()?;

    let agent_op = AgentOperation {
        agent_id: agent,
        session_id: session,
        timestamp: Utc::now(),
        operation: op_type,
        result: OperationResult::Success,
        summary,
        context_refs: refs,
        files_touched: files,
        parent_op: None,
        metadata: HashMap::new(),
    };

    let id = repo.create_agent_operation(agent_op)?;
    println!("Logged agent operation {}", id.short());
    Ok(())
}
