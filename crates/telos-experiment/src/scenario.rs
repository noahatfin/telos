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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_scenario_from_toml() {
        let toml_content = r#"
[scenario]
name = "test_scenario"
category = "true_positive"
description = "A test"

[diff]
content = "- old\n+ new"
commit_message = "Update thing"

[context]
git_only = "git log output"
constraints_md = "- Must do X"
telos_json = '{"constraints": []}'

[prompt]
template = "Review: {{commit_message}}\n{{diff}}\n{{context}}"

[expected]
should_reject = true
key_findings = ["finding1"]
"#;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.write_all(toml_content.as_bytes()).unwrap();
        let scenario = ScenarioFile::load(tmp.path()).unwrap();
        assert_eq!(scenario.scenario.name, "test_scenario");
        assert_eq!(scenario.scenario.category, "true_positive");
        assert_eq!(scenario.scenario.description, "A test");
        assert_eq!(scenario.diff.commit_message, "Update thing");
        assert!(scenario.expected.should_reject);
        assert_eq!(scenario.expected.key_findings, vec!["finding1"]);
    }

    #[test]
    fn render_prompt_substitutes_context() {
        let scenario = ScenarioFile {
            scenario: ScenarioMeta {
                name: "test".into(),
                category: "true_positive".into(),
                description: "desc".into(),
            },
            diff: DiffConfig {
                content: "- old\n+ new".into(),
                commit_message: "fix stuff".into(),
            },
            context: ContextConfig {
                git_only: "GIT CONTEXT".into(),
                constraints_md: "MD CONTEXT".into(),
                telos_json: "TELOS CONTEXT".into(),
            },
            prompt: PromptConfig {
                template: "Msg: {{commit_message}}\nDiff: {{diff}}\nCtx: {{context}}".into(),
            },
            expected: ExpectedConfig {
                should_reject: true,
                key_findings: vec![],
            },
        };

        let git_prompt = scenario.render_prompt("git_only");
        assert!(git_prompt.contains("GIT CONTEXT"));
        assert!(git_prompt.contains("fix stuff"));
        assert!(git_prompt.contains("- old\n+ new"));
        assert!(!git_prompt.contains("{{commit_message}}"));
        assert!(!git_prompt.contains("{{diff}}"));
        assert!(!git_prompt.contains("{{context}}"));

        let md_prompt = scenario.render_prompt("constraints_md");
        assert!(md_prompt.contains("MD CONTEXT"));
        assert!(md_prompt.contains("fix stuff"));

        let telos_prompt = scenario.render_prompt("telos");
        assert!(telos_prompt.contains("TELOS CONTEXT"));
        assert!(telos_prompt.contains("fix stuff"));
    }
}
