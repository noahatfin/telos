use anyhow::Result;
use std::process::Command;
use std::time::Instant;

#[allow(dead_code)]
pub struct CodexRunner {
    pub binary: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CodexResponse {
    pub output: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

impl Default for CodexRunner {
    fn default() -> Self {
        Self {
            binary: "codex".into(),
            timeout_secs: 120,
        }
    }
}

impl CodexRunner {
    pub fn run(&self, prompt: &str) -> Result<CodexResponse> {
        let start = Instant::now();

        let output = Command::new(&self.binary)
            .args(["-q", "--prompt", prompt])
            .output()?;

        let duration_ms = start.elapsed().as_millis() as u64;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            anyhow::bail!(
                "codex exited with {}: {}",
                output.status.code().unwrap_or(-1),
                stderr
            );
        }

        Ok(CodexResponse {
            output: stdout,
            exit_code: output.status.code().unwrap_or(0),
            duration_ms,
        })
    }

    /// Check if the codex binary is available.
    pub fn is_available(&self) -> bool {
        Command::new(&self.binary)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
