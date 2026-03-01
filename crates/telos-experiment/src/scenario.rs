use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioFile {
    pub scenario: ScenarioMeta,
    pub diff: DiffConfig,
    pub context: ContextConfig,
    pub prompt: PromptConfig,
    pub expected: ExpectedConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioMeta {
    pub name: String,
    pub category: String, // "true_positive" or "false_positive"
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffConfig {
    pub content: String,
    pub commit_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub git_only: String,
    pub constraints_md: String,
    pub telos_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedConfig {
    pub should_reject: bool,
    pub key_findings: Vec<String>,
}

impl ScenarioFile {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let scenario: ScenarioFile = toml::from_str(&content)?;
        Ok(scenario)
    }

    /// Render the prompt template with the given condition's context.
    pub fn render_prompt(&self, condition: &str) -> String {
        let context = match condition {
            "git_only" => &self.context.git_only,
            "constraints_md" => &self.context.constraints_md,
            "telos" => &self.context.telos_json,
            _ => "",
        };
        self.prompt
            .template
            .replace("{{commit_message}}", &self.diff.commit_message)
            .replace("{{diff}}", &self.diff.content)
            .replace("{{context}}", context)
    }
}
