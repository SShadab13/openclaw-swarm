use std::process::{Command, Stdio};
use std::time::Duration;
use anyhow::{Result, Context};
use tracing::{info, warn};
use serde::{Deserialize, Serialize};

/// Kimi CLI Runner — Spawns `kimi` subprocess for fast coding and exploration.
/// Best for: implementation tasks, quick prototyping, vibe coding.
#[allow(dead_code)]
pub struct KimiRunner {
    timeout: Duration,
    workspace: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KimiResult {
    pub stdout: String,
    pub stderr: String,
    pub files_changed: Vec<String>,
    pub exit_code: i32,
    pub tokens_used: Option<u64>,
}

impl KimiRunner {
    pub fn new(workspace: &str, timeout_secs: u64) -> Self {
        Self {
            timeout: Duration::from_secs(timeout_secs),
            workspace: workspace.to_string(),
        }
    }

    /// Execute a task via Kimi CLI.
    /// 
    /// Example: `runner.execute("Implement Day Seal feature", "typescript")`
    pub fn execute(&self, task_description: &str, context: &str) -> Result<KimiResult> {
        info!("[KimiRunner] Executing: {}", task_description);

        // Kimi CLI on Windows cannot handle emoji in prompts — strip them
        let clean_desc = strip_emoji(task_description);

        let query = format!(
            "You are a {} developer. Task: {}. Write the code, save files, run tests if applicable.",
            context, clean_desc
        );

        let output = Command::new("kimi")
            .args([
                "--prompt", &query,
                "--print",
                "--yes",
                "--work-dir", &self.workspace,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| "Failed to spawn kimi CLI. Is it installed?")?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            warn!("[KimiRunner] Non-zero exit: {}", stderr);
        }

        // Detect files changed (simple heuristic: look for file paths in output)
        let files_changed = self.extract_file_paths(&stdout);

        Ok(KimiResult {
            stdout,
            stderr,
            files_changed,
            exit_code: output.status.code().unwrap_or(-1),
            tokens_used: None, // Kimi CLI doesn't expose this directly
        })
    }

    /// Execute with specific file context (e.g., "fix this file").
    pub fn execute_on_file(&self, task: &str, file_path: &str) -> Result<KimiResult> {
        let query = format!("{}\n\nFile: {}", task, file_path);
        self.execute(&query, "general")
    }

    /// Run a review pass (Kimi is good at quick code review).
    pub fn review_code(&self, file_paths: &[String]) -> Result<KimiResult> {
        let files = file_paths.join(", ");
        let query = format!("Review these files for bugs, style issues, and improvements: {}", files);
        self.execute(&query, "code-reviewer")
    }

    fn extract_file_paths(&self, output: &str) -> Vec<String> {
        // Simple regex-like extraction: find lines that look like file paths
        output.lines()
            .filter(|line| line.contains('/') || line.contains('\\'))
            .filter(|line| line.contains('.') && !line.starts_with("http"))
            .map(|s| s.trim().to_string())
            .take(20)
            .collect()
    }
}

/// Strip emoji characters from text (Windows console compatibility for Kimi CLI).
fn strip_emoji(text: &str) -> String {
    text.chars()
        .filter(|c| !is_emoji(*c))
        .collect()
}

fn is_emoji(c: char) -> bool {
    // Basic emoji ranges: covers most common emojis used in prompts
    (c >= '\u{1F300}' && c <= '\u{1F9FF}') ||  // Misc symbols & pictographs
    (c >= '\u{2600}' && c <= '\u{26FF}') ||    // Misc symbols
    (c >= '\u{2700}' && c <= '\u{27BF}') ||    // Dingbats
    (c >= '\u{1F600}' && c <= '\u{1F64F}') ||  // Emoticons
    (c >= '\u{1F680}' && c <= '\u{1F6FF}') ||  // Transport & map
    (c >= '\u{1F1E0}' && c <= '\u{1F1FF}')     // Flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_file_paths() {
        let runner = KimiRunner::new("/tmp", 60);
        let output = "Created src/main.rs\nUpdated tests/lib.rs\nError in file";
        let paths = runner.extract_file_paths(output);
        assert_eq!(paths.len(), 2);
        assert!(paths[0].contains("main.rs"));
    }
}
