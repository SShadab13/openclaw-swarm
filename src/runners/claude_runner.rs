use std::process::{Command, Stdio};
use std::time::Duration;
use anyhow::{Result, Context};
use tracing::{info, warn};
use serde::{Deserialize, Serialize};

/// Claude Code CLI Runner — Spawns `claude` subprocess for deep analysis,
/// bug hunting, refactoring, and code review.
///
/// Best for: architecture review, finding subtle bugs, complex refactors.
pub struct ClaudeRunner {
    timeout: Duration,
    workspace: String,
    permission_mode: String,
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
            // Safe default: agent edits are auto-accepted but destructive
            // actions still gate. Bypass requires explicit opt-in below.
            permission_mode: "acceptEdits".to_string(),
        }
    }

    /// Explicit opt-in to run without any permission gate.
    /// Only use inside a throwaway sandbox — never on a client workspace.
    pub fn with_bypass_permissions(mut self) -> Self {
        self.permission_mode = "bypassPermissions".to_string();
        self
    }

    /// Execute a task via Claude Code CLI.
    ///
    /// Uses `--print --output-format json` for structured headless output;
    /// falls back to raw stdout if the CLI returns non-JSON.
    /// Enforces `self.timeout` — the subprocess is killed on expiry.
    pub fn execute(&self, instructions: &str) -> Result<ClaudeResult> {
        info!("[ClaudeRunner] Executing: {}", instructions);

        let mut cmd = Command::new("claude");
        cmd.args([
            "--print",
            "--output-format", "json",
            "--permission-mode", &self.permission_mode,
            instructions,
        ])
        .current_dir(&self.workspace);

        let output = run_with_timeout(cmd, self.timeout)
            .with_context(|| "Failed to run Claude Code CLI. Is it installed?")?;

        let raw_stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            warn!("[ClaudeRunner] Non-zero exit: {}", stderr);
        }

        // Structured result text if JSON parsed, else raw stdout as-is
        let stdout = parse_claude_json(&raw_stdout).unwrap_or(raw_stdout);

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

/// Extract the `result` text from `claude --output-format json` output.
/// Returns None when stdout is not the expected JSON shape.
fn parse_claude_json(stdout: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    v.get("result")?.as_str().map(|s| s.to_string())
}

/// Run a command, killing it if it exceeds `timeout`.
/// Pipes are drained on threads so large output cannot deadlock the child.
fn run_with_timeout(mut cmd: Command, timeout: Duration) -> Result<std::process::Output> {
    use std::io::Read;
    use std::time::Instant;

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| "Failed to spawn subprocess")?;

    let mut stdout_pipe = child.stdout.take().expect("stdout piped");
    let mut stderr_pipe = child.stderr.take().expect("stderr piped");
    let stdout_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout_pipe.read_to_end(&mut buf);
        buf
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr_pipe.read_to_end(&mut buf);
        buf
    });

    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let stdout = stdout_thread.join().unwrap_or_default();
            let stderr = stderr_thread.join().unwrap_or_default();
            return Ok(std::process::Output { status, stdout, stderr });
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("Subprocess timed out after {:?} and was killed", timeout);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_permission_mode_is_not_bypass() {
        let runner = ClaudeRunner::new(".", 60);
        assert_eq!(runner.permission_mode, "acceptEdits");

        let bypass = ClaudeRunner::new(".", 60).with_bypass_permissions();
        assert_eq!(bypass.permission_mode, "bypassPermissions");
    }

    #[test]
    fn test_parse_claude_json_valid() {
        let out = r#"{"type":"result","subtype":"success","is_error":false,"result":"All checks passed."}"#;
        assert_eq!(parse_claude_json(out), Some("All checks passed.".to_string()));
    }

    #[test]
    fn test_parse_claude_json_invalid_falls_back() {
        assert_eq!(parse_claude_json("plain text output"), None);
        assert_eq!(parse_claude_json(r#"{"no_result_field":1}"#), None);
    }

    #[test]
    fn test_run_with_timeout_quick_command_succeeds() {
        #[cfg(windows)]
        let mut cmd = Command::new("cmd");
        #[cfg(windows)]
        cmd.args(["/C", "echo hello"]);
        #[cfg(unix)]
        let mut cmd = Command::new("sh");
        #[cfg(unix)]
        cmd.args(["-c", "echo hello"]);

        let out = run_with_timeout(cmd, Duration::from_secs(10)).unwrap();
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains("hello"));
    }

    #[test]
    fn test_run_with_timeout_kills_hung_command() {
        #[cfg(windows)]
        let mut cmd = Command::new("ping");
        #[cfg(windows)]
        cmd.args(["-n", "30", "127.0.0.1"]);
        #[cfg(unix)]
        let mut cmd = Command::new("sleep");
        #[cfg(unix)]
        cmd.arg("30");

        let start = std::time::Instant::now();
        let result = run_with_timeout(cmd, Duration::from_secs(1));
        assert!(result.is_err(), "Expected timeout error");
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "Kill should happen near the 1s deadline, not after the command finishes"
        );
    }
}
