use crate::codex::CodexRunner;
use crate::scenario::ScenarioFile;
use crate::scorer::{JudgeScorer, Score};
use serde::{Deserialize, Serialize};

pub const CONDITIONS: [&str; 3] = ["git_only", "constraints_md", "telos"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialResult {
    pub scenario_name: String,
    pub condition: String,
    pub trial_number: usize,
    pub llm_response: String,
    pub score: Option<Score>,
    pub duration_ms: u64,
}

pub struct ExperimentRunner {
    codex: CodexRunner,
    scorer: JudgeScorer,
    pub repeats: usize,
}

impl ExperimentRunner {
    pub fn new(repeats: usize) -> Self {
        let codex = CodexRunner::default();
        let scorer = JudgeScorer::new(CodexRunner::default());
        Self {
            codex,
            scorer,
            repeats,
        }
    }

    pub fn run_scenario(
        &self,
        scenario: &ScenarioFile,
        conditions: &[&str],
    ) -> anyhow::Result<Vec<TrialResult>> {
        let mut results = Vec::new();

        for &condition in conditions {
            let prompt = scenario.render_prompt(condition);

            for trial in 1..=self.repeats {
                eprintln!(
                    "  [{}/{}] {} / {} ...",
                    trial, self.repeats, scenario.scenario.name, condition
                );

                let response = self.codex.run(&prompt)?;

                let score = self.scorer.score(scenario, &response.output).ok();

                results.push(TrialResult {
                    scenario_name: scenario.scenario.name.clone(),
                    condition: condition.into(),
                    trial_number: trial,
                    llm_response: response.output,
                    score,
                    duration_ms: response.duration_ms,
                });
            }
        }

        Ok(results)
    }
}
