mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "telos", about = "Intent-native development platform", version)]
struct Cli {
    /// Output in JSON format
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Telos repository
    Init,

    /// Create a new intent (analogous to git commit)
    Intent {
        /// Intent statement (natural language)
        #[arg(short, long)]
        statement: String,

        /// Constraints (repeatable)
        #[arg(long)]
        constraint: Vec<String>,

        /// Impact area tags (repeatable)
        #[arg(long)]
        impact: Vec<String>,

        /// Behavior clauses (repeatable, format: "GIVEN x|WHEN y|THEN z")
        #[arg(long)]
        behavior: Vec<String>,
    },

    /// Manage intent streams (analogous to git branch)
    Stream {
        #[command(subcommand)]
        action: StreamAction,
    },

    /// Show intent history (analogous to git log)
    Log {
        /// Maximum number of entries
        #[arg(short = 'n', long, default_value = "20")]
        max_count: usize,
    },

    /// Show details of any object by ID or prefix
    Show {
        /// Object ID (full or prefix, minimum 4 chars)
        id: String,
    },

    /// Record a decision about an intent
    Decide {
        /// Intent ID this decision is about
        #[arg(long)]
        intent: String,

        /// The question being decided
        #[arg(long)]
        question: String,

        /// The decision made
        #[arg(long)]
        decision: String,

        /// Rationale for the decision
        #[arg(long)]
        rationale: Option<String>,

        /// Alternatives considered (repeatable, format: "description|rejection_reason")
        #[arg(long)]
        alternative: Vec<String>,

        /// Tags (repeatable)
        #[arg(long)]
        tag: Vec<String>,
    },

    /// Query objects in the repository
    Query {
        #[command(subcommand)]
        action: QueryAction,
    },

    /// Show aggregated context for an impact area (for AI agents)
    Context {
        /// Impact area to retrieve context for
        #[arg(long)]
        impact: String,
    },

    /// Create a standalone constraint
    Constraint {
        /// Constraint statement
        #[arg(short, long)]
        statement: String,

        /// Severity level (must, should, prefer)
        #[arg(long, default_value = "should")]
        severity: String,

        /// Impact area tags (repeatable)
        #[arg(long)]
        impact: Vec<String>,

        /// Scope file paths (repeatable)
        #[arg(long)]
        scope: Vec<String>,
    },

    /// Supersede an existing constraint
    Supersede {
        /// Constraint ID to supersede
        id: String,

        /// New constraint statement
        #[arg(short, long)]
        statement: String,

        /// Severity level (must, should, prefer)
        #[arg(long, default_value = "should")]
        severity: String,

        /// Reason for superseding
        #[arg(long)]
        reason: Option<String>,
    },

    /// Deprecate a constraint
    Deprecate {
        /// Constraint ID to deprecate
        id: String,

        /// Reason for deprecation
        #[arg(long)]
        reason: String,
    },

    /// Create a code binding
    Bind {
        /// Object ID to bind
        id: String,

        /// File path to bind to
        #[arg(long)]
        file: String,

        /// Symbol name (function, type, etc.)
        #[arg(long)]
        symbol: Option<String>,

        /// Binding type (file, function, module, api, type)
        #[arg(long, default_value = "file")]
        r#type: String,
    },

    /// Validate bindings and constraints against code
    Check {
        /// Check code bindings
        #[arg(long)]
        bindings: bool,

        /// Run all checks
        #[arg(long)]
        all: bool,
    },

    /// Log an agent operation
    AgentLog {
        /// Agent identifier
        #[arg(long)]
        agent: String,

        /// Session identifier
        #[arg(long)]
        session: String,

        /// Operation type (review, generate, decide, query, violation, or custom)
        #[arg(long)]
        operation: String,

        /// Summary of the operation
        #[arg(long)]
        summary: String,

        /// Context object references (repeatable)
        #[arg(long)]
        context_ref: Vec<String>,

        /// Files touched (repeatable)
        #[arg(long)]
        file: Vec<String>,
    },

    /// Rebuild all indexes
    Reindex,
}

#[derive(Subcommand)]
enum QueryAction {
    /// Query intents with optional filters
    Intents {
        /// Filter by impact area tag
        #[arg(long)]
        impact: Option<String>,

        /// Filter by constraint substring (case-insensitive)
        #[arg(long)]
        constraint_contains: Option<String>,
    },
    /// Query decision records with optional filters
    Decisions {
        /// Filter by intent ID (full or prefix)
        #[arg(long)]
        intent: Option<String>,

        /// Filter by tag
        #[arg(long)]
        tag: Option<String>,
    },
    /// Query constraints
    Constraints {
        /// Filter by file path
        #[arg(long)]
        file: Option<String>,

        /// Filter by symbol name
        #[arg(long)]
        symbol: Option<String>,

        /// Filter by impact area
        #[arg(long)]
        impact: Option<String>,

        /// Filter by status (active, superseded, deprecated)
        #[arg(long, default_value = "active")]
        status: String,
    },
    /// Query agent operations
    AgentOps {
        /// Filter by agent identifier
        #[arg(long)]
        agent: Option<String>,

        /// Filter by session identifier
        #[arg(long)]
        session: Option<String>,
    },
}

#[derive(Subcommand)]
enum StreamAction {
    /// Create a new stream
    Create {
        /// Stream name
        name: String,
    },
    /// List all streams
    List,
    /// Switch to a different stream
    Switch {
        /// Stream name to switch to
        name: String,
    },
    /// Delete a stream
    Delete {
        /// Stream name to delete
        name: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init => commands::init::run(),
        Commands::Intent {
            statement,
            constraint,
            impact,
            behavior,
        } => commands::intent::run(statement, constraint, impact, behavior),
        Commands::Stream { action } => match action {
            StreamAction::Create { name } => commands::stream::create(name),
            StreamAction::List => commands::stream::list(),
            StreamAction::Switch { name } => commands::stream::switch(name),
            StreamAction::Delete { name } => commands::stream::delete(name),
        },
        Commands::Log { max_count } => commands::log::run(max_count, cli.json),
        Commands::Show { id } => commands::show::run(id, cli.json),
        Commands::Decide {
            intent,
            question,
            decision,
            rationale,
            alternative,
            tag,
        } => commands::decide::run(intent, question, decision, rationale, alternative, tag),
        Commands::Query { action } => match action {
            QueryAction::Intents {
                impact,
                constraint_contains,
            } => commands::query::intents(impact, constraint_contains, cli.json),
            QueryAction::Decisions { intent, tag } => {
                commands::query::decisions(intent, tag, cli.json)
            }
            QueryAction::Constraints {
                file,
                symbol,
                impact,
                status,
            } => commands::query::constraints(file, symbol, impact, status, cli.json),
            QueryAction::AgentOps { agent, session } => {
                commands::query::agent_ops(agent, session, cli.json)
            }
        },
        Commands::Context { impact } => commands::context::run(impact, cli.json),
        Commands::Constraint {
            statement,
            severity,
            impact,
            scope,
        } => commands::constraint::run(statement, severity, impact, scope),
        Commands::Supersede {
            id,
            statement,
            severity,
            reason,
        } => commands::supersede::run(id, statement, severity, reason),
        Commands::Deprecate { id, reason } => commands::deprecate::run(id, reason),
        Commands::Bind {
            id,
            file,
            symbol,
            r#type,
        } => commands::bind::run(id, file, symbol, r#type),
        Commands::Check { bindings, all } => commands::check::run(bindings, all),
        Commands::AgentLog {
            agent,
            session,
            operation,
            summary,
            context_ref,
            file,
        } => commands::agent_log::run(agent, session, operation, summary, context_ref, file),
        Commands::Reindex => commands::reindex::run(),
    };

    if let Err(e) = result {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
