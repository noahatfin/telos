mod codex;
mod report;
mod runner;
mod scenario;
mod scorer;

use clap::{Parser, Subcommand};
use scenario::ScenarioFile;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "telos-experiment",
    about = "LLM experiment framework for Telos validation"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run experiments
    Run {
        /// Number of repetitions per condition
        #[arg(long, default_value = "5")]
        repeats: usize,

        /// Specific scenario to run (by name)
        #[arg(long)]
        scenario: Option<String>,

        /// Conditions to test (repeatable: git_only, constraints_md, telos)
        #[arg(long)]
        condition: Vec<String>,

        /// Directory containing scenario TOML files
        #[arg(long, default_value = "crates/telos-experiment/scenarios")]
        scenarios_dir: PathBuf,
    },

    /// List available scenarios
    List {
        /// Directory containing scenario TOML files
        #[arg(long, default_value = "crates/telos-experiment/scenarios")]
        scenarios_dir: PathBuf,
    },

    /// Show report from latest results
    Report {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Results file path
        #[arg(long, default_value = ".telos-experiment/results/latest.json")]
        results: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Run {
            repeats,
            scenario: scenario_filter,
            condition,
            scenarios_dir,
        } => run_experiments(repeats, scenario_filter, condition, scenarios_dir),
        Commands::List { scenarios_dir } => list_scenarios(scenarios_dir),
        Commands::Report { json, results } => show_report(json, results),
    };

    if let Err(e) = result {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn run_experiments(
    repeats: usize,
    scenario_filter: Option<String>,
    conditions: Vec<String>,
    scenarios_dir: PathBuf,
) -> anyhow::Result<()> {
    let codex_runner = codex::CodexRunner::default();
    if !codex_runner.is_available() {
        anyhow::bail!(
            "codex CLI not found. Install it first: https://github.com/openai/codex"
        );
    }

    let scenarios = load_scenarios(&scenarios_dir, scenario_filter.as_deref())?;
    if scenarios.is_empty() {
        anyhow::bail!("No scenarios found in {}", scenarios_dir.display());
    }

    let active_conditions: Vec<&str> = if conditions.is_empty() {
        runner::CONDITIONS.to_vec()
    } else {
        conditions.iter().map(|s| s.as_str()).collect()
    };

    eprintln!(
        "Running {} scenarios x {} conditions x {} repeats",
        scenarios.len(),
        active_conditions.len(),
        repeats
    );

    let runner = runner::ExperimentRunner::new(repeats);
    let mut all_results = Vec::new();

    for scenario in &scenarios {
        eprintln!(
            "\nScenario: {} ({})",
            scenario.scenario.name, scenario.scenario.category
        );
        let results = runner.run_scenario(scenario, &active_conditions)?;
        all_results.extend(results);
    }

    // Save results
    let results_dir = PathBuf::from(".telos-experiment/results");
    std::fs::create_dir_all(&results_dir)?;
    let results_json = serde_json::to_string_pretty(&all_results)?;
    let latest_path = results_dir.join("latest.json");
    std::fs::write(&latest_path, &results_json)?;

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let archive_path = results_dir.join(format!("run-{}.json", timestamp));
    std::fs::write(&archive_path, &results_json)?;

    eprintln!("\nResults saved to {}", latest_path.display());

    // Print summary
    let reports = report::aggregate(&all_results, "all");
    report::print_table(&reports);

    Ok(())
}

fn list_scenarios(scenarios_dir: PathBuf) -> anyhow::Result<()> {
    let scenarios = load_scenarios(&scenarios_dir, None)?;
    println!(
        "{:<30} {:<15} Description",
        "Name", "Category"
    );
    println!("{}", "-".repeat(80));
    for s in &scenarios {
        println!(
            "{:<30} {:<15} {}",
            s.scenario.name, s.scenario.category, s.scenario.description
        );
    }
    println!("\n{} scenarios found", scenarios.len());
    Ok(())
}

fn show_report(json: bool, results_path: PathBuf) -> anyhow::Result<()> {
    let data = std::fs::read_to_string(&results_path)?;
    let results: Vec<runner::TrialResult> = serde_json::from_str(&data)?;

    if json {
        let reports = report::aggregate(&results, "all");
        println!("{}", serde_json::to_string_pretty(&reports)?);
    } else {
        let reports = report::aggregate(&results, "all");
        report::print_table(&reports);
    }
    Ok(())
}

fn load_scenarios(dir: &PathBuf, filter: Option<&str>) -> anyhow::Result<Vec<ScenarioFile>> {
    let mut scenarios = Vec::new();
    if !dir.exists() {
        return Ok(scenarios);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "toml").unwrap_or(false) {
            match ScenarioFile::load(&path) {
                Ok(s) => {
                    if let Some(f) = filter {
                        if s.scenario.name != f {
                            continue;
                        }
                    }
                    scenarios.push(s);
                }
                Err(e) => eprintln!("Warning: failed to load {}: {}", path.display(), e),
            }
        }
    }
    scenarios.sort_by(|a, b| a.scenario.name.cmp(&b.scenario.name));
    Ok(scenarios)
}
