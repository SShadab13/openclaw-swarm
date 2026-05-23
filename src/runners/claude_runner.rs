use std::process::{Command, Stdio};
use std::time::Duration;
use anyhow::{Result, Context};
use tracing::{info, warn};
use serde::{Deserialize, Serialize};

/// Claude Code CLI Runner — Spawns `claude` subprocess for deep analysis,
/// bug hunting, refactoring, and code review.
/// 
/// Best for: architecture review, finding subtle bugs, complex refactors.
#[allow(dead_code)]
pub struct ClaudeRunner {
    timeout: Duration,
    workspace: String,
    permission_mode: String, // "bypassPermissions" for headless
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeResult {
    pub stdout: String,
    pub stderr: String,
    pub files_changed: Vec<String>,
    pub bugs_found: Vec<String>,
    pub exit_code: i32,
    pub analysis_summary: Option<String>,
}

impl ClaudeRunner {
    pub fn new(workspace: &str, timeout_secs: u64) -> Self {
        Self {
            timeout: Duration::from_secs(timeout_secs),
            workspace: workspace.to_string(),
            permission_mode: "bypassPermissions".to_string(),
        }
    }

    /// Execute a task via Claude Code CLI.
    /// 
    /// Uses `--print` for headless output and `--permission-mode bypassPermissions`
    /// for non-interactive execution.
    pub fn execute(&self, instructions: &str) -> Result<ClaudeResult> {
        info!("[ClaudeRunner] Executing: {}", instructions);

        let output = Command::new("claude")
            .args([
                "--print",
                "--permission-mode", &self.permission_mode,
                instructions,
            ])
            .current_dir(&self.workspace)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| "Failed to spawn Claude Code CLI. Is it installed?")?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            warn!("[ClaudeRunner] Non-zero exit: {}", stderr);
        }

        let files_changed = self.extract_file_paths(&stdout);
        let bugs_found = self.extract_bugs(&stdout);
        let analysis_summary = self.extract_summary(&stdout);

        Ok(ClaudeResult {
            stdout,
            stderr,
            files_changed,
            bugs_found,
            exit_code: output.status.code().unwrap_or(-1),
            analysis_summary,
        })
    }

    /// Specialized: Bug hunt mode.
    /// Claude Code found 7 real bugs in services/xp.ts — this is its strength.
    pub fn bug_hunt(&self, target_file: &str) -> Result<ClaudeResult> {
        let instructions = format!(
            "Find ALL bugs in {}. Check for: type mismatches, uninitialized variables, \\
             off-by-one errors, dead code, and logic errors. Report each bug with line number and fix.",
            target_file
        );
        self.execute(&instructions)
    }

    /// Specialized: Architecture review.
    pub fn review_architecture(&self, files: &[String]) -> Result<ClaudeResult> {
        let file_list = files.join(", ");
        let instructions = format!(
            "Review the architecture of these files: {}. Check for: tight coupling, \\
             missing abstractions, single responsibility violations, and scalability issues.",
            file_list
        );
        self.execute(&instructions)
    }

    /// Specialized: Refactor with tests.
    pub fn refactor_with_tests(&self, target_file: &str) -> Result<ClaudeResult> {
        let instructions = format!(
            "Refactor {} for clarity and maintainability. Add comprehensive tests. \\
             Ensure all existing tests pass. Report what you changed and why.",
            target_file
        );
        self.execute(&instructions)
    }

    fn extract_file_paths(&self, output: &str) -> Vec<String> {
        output.lines()
            .filter(|line| {
                (line.contains("/") || line.contains("\\"))
                    && line.contains('.')
                    && !line.starts_with("http")
            })
            .map(|s| s.trim().to_string())
            .take(30)
            .collect()
    }

    fn extract_bugs(&self, output: &str) -> Vec<String> {
        output.lines()
            .filter(|line| {
                line.to_lowercase().contains("bug")
                    || line.to_lowercase().contains("error")
                    || line.to_lowercase().contains("issue")
                    || line.to_lowercase().contains("crash")
            })
            .map(|s| s.trim().to_string())
            .take(20)
            .collect()
    }

    fn extract_summary(&self, output: &str) -> Option<String> {
        // Look for a summary paragraph near the end
        let lines: Vec<&str> = output.lines().collect();
        let last_10 = lines.iter().rev().take(10).cloned().collect::<Vec<_>>();
        let summary = last_10.join("\n");
        
        if summary.len() > 50 {
            Some(summary)
        } else {
            None
        }
    }
}
