use crate::runner::TrialResult;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct ScenarioReport {
    pub scenario_name: String,
    pub category: String,
    pub conditions: HashMap<String, ConditionStats>,
}

#[derive(Debug, Serialize)]
pub struct ConditionStats {
    pub trials: usize,
    pub caught_issue_rate: f64,
    pub rejection_rate: f64,
    pub cited_constraint_rate: f64,
    pub avg_reasoning_quality: f64,
    pub avg_duration_ms: f64,
}

pub fn aggregate(results: &[TrialResult], category: &str) -> Vec<ScenarioReport> {
    let mut by_scenario: HashMap<String, Vec<&TrialResult>> = HashMap::new();
    for r in results {
        by_scenario
            .entry(r.scenario_name.clone())
            .or_default()
            .push(r);
    }

    let mut reports = Vec::new();
    for (name, trials) in by_scenario {
        let mut conditions: HashMap<String, ConditionStats> = HashMap::new();

        let mut by_condition: HashMap<String, Vec<&TrialResult>> = HashMap::new();
        for t in &trials {
            by_condition
                .entry(t.condition.clone())
                .or_default()
                .push(t);
        }

        for (cond, cond_trials) in by_condition {
            let n = cond_trials.len();
            let scored: Vec<_> = cond_trials
                .iter()
                .filter_map(|t| t.score.as_ref())
                .collect();
            let s = scored.len() as f64;

            conditions.insert(
                cond,
                ConditionStats {
                    trials: n,
                    caught_issue_rate: if s > 0.0 {
                        scored.iter().filter(|sc| sc.caught_issue).count() as f64 / s
                    } else {
                        0.0
                    },
                    rejection_rate: if s > 0.0 {
                        scored
                            .iter()
                            .filter(|sc| sc.recommended_rejection)
                            .count() as f64
                            / s
                    } else {
                        0.0
                    },
                    cited_constraint_rate: if s > 0.0 {
                        scored.iter().filter(|sc| sc.cited_constraint).count() as f64 / s
                    } else {
                        0.0
                    },
                    avg_reasoning_quality: if s > 0.0 {
                        scored
                            .iter()
                            .map(|sc| sc.reasoning_quality as f64)
                            .sum::<f64>()
                            / s
                    } else {
                        0.0
                    },
                    avg_duration_ms: cond_trials.iter().map(|t| t.duration_ms as f64).sum::<f64>()
                        / n as f64,
                },
            );
        }

        reports.push(ScenarioReport {
            scenario_name: name,
            category: category.into(),
            conditions,
        });
    }

    reports
}

pub fn print_table(reports: &[ScenarioReport]) {
    for report in reports {
        println!(
            "\n=== {} ({}) ===",
            report.scenario_name, report.category
        );
        println!(
            "{:<20} {:>10} {:>10} {:>10}",
            "Metric", "Git-only", "CONST.md", "Telos"
        );

        let git = report.conditions.get("git_only");
        let cmd = report.conditions.get("constraints_md");
        let telos = report.conditions.get("telos");

        let fmt = |stats: Option<&ConditionStats>, f: fn(&ConditionStats) -> f64| -> String {
            stats
                .map(|s| format!("{:.0}%", f(s) * 100.0))
                .unwrap_or_else(|| "—".into())
        };

        println!(
            "{:<20} {:>10} {:>10} {:>10}",
            "Caught issue",
            fmt(git, |s| s.caught_issue_rate),
            fmt(cmd, |s| s.caught_issue_rate),
            fmt(telos, |s| s.caught_issue_rate)
        );
        println!(
            "{:<20} {:>10} {:>10} {:>10}",
            "Rejected",
            fmt(git, |s| s.rejection_rate),
            fmt(cmd, |s| s.rejection_rate),
            fmt(telos, |s| s.rejection_rate)
        );
        println!(
            "{:<20} {:>10} {:>10} {:>10}",
            "Cited constraint",
            fmt(git, |s| s.cited_constraint_rate),
            fmt(cmd, |s| s.cited_constraint_rate),
            fmt(telos, |s| s.cited_constraint_rate)
        );
    }
}
