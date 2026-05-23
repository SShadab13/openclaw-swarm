pub mod kimi_runner;
pub mod claude_runner;
pub mod openclaw_runner;

use std::collections::HashMap;
use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use crate::models::Persona;
use crate::runners::kimi_runner::KimiRunner;
use crate::runners::claude_runner::ClaudeRunner;
use crate::runners::openclaw_runner::OpenClawRunner;

/// The RunnerRegistry maps personas to their preferred CLI tools.
/// Each persona has a primary runner and a fallback.
pub struct RunnerRegistry {
#[allow(dead_code)]
    workspace: String,
    runners: HashMap<String, Box<dyn Runner + Send + Sync>>,
}

#[async_trait]
pub trait Runner: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, task: &str) -> Result<String>;
    fn is_available(&self) -> bool;
}

impl RunnerRegistry {
    pub fn new(workspace: &str) -> Self {
        let mut runners: HashMap<String, Box<dyn Runner + Send + Sync>> = HashMap::new();

        runners.insert(
            "kimi".to_string(),
            Box::new(KimiAdapter::new(workspace, 120)),
        );
        runners.insert(
            "claude".to_string(),
            Box::new(ClaudeAdapter::new(workspace, 300)),
        );
        runners.insert(
            "openclaw".to_string(),
            Box::new(OpenClawAdapter::new(workspace, 60)),
        );

        Self {
            workspace: workspace.to_string(),
            runners,
        }
    }

    pub fn runner_for_persona(&self, persona: &Persona) -> Result<&(dyn Runner + Send + Sync)> {
        let preferred = if persona.skills.contains(&"security".to_string())
            || persona.skills.contains(&"testing".to_string())
        {
            "claude"
        } else if persona.skills.contains(&"mcp_servers".to_string())
            || persona.skills.contains(&"agent_swarm_design".to_string())
            || persona.skills.contains(&"multi_agent_coordination".to_string())
        {
            "openclaw"
        } else {
            "kimi"
        };

        if let Some(runner) = self.runners.get(preferred) {
            if runner.is_available() {
                return Ok(runner.as_ref());
            }
        }

        for (name, runner) in &self.runners {
            if runner.is_available() {
                info!("Fallback: using {} for {}", name, persona.name);
                return Ok(runner.as_ref());
            }
        }

        anyhow::bail!("No runner available for persona {}", persona.name)
    }

    pub fn get(&self, name: &str) -> Option<&(dyn Runner + Send + Sync)> {
        self.runners.get(name).map(|r| r.as_ref())
    }
}

struct KimiAdapter {
    runner: KimiRunner,
}

impl KimiAdapter {
    fn new(workspace: &str, timeout: u64) -> Self {
        Self {
            runner: KimiRunner::new(workspace, timeout),
        }
    }
}

#[async_trait]
impl Runner for KimiAdapter {
    fn name(&self) -> &str {
        "kimi"
    }

    async fn execute(&self, task: &str) -> Result<String> {
        let result = self.runner.execute(task, "general")?;
        Ok(format!("[Kimi] {}\n{}", result.exit_code, result.stdout))
    }

    fn is_available(&self) -> bool {
        std::process::Command::new("kimi")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

struct ClaudeAdapter {
    runner: ClaudeRunner,
}

impl ClaudeAdapter {
    fn new(workspace: &str, timeout: u64) -> Self {
        Self {
            runner: ClaudeRunner::new(workspace, timeout),
        }
    }
}

#[async_trait]
impl Runner for ClaudeAdapter {
    fn name(&self) -> &str {
        "claude"
    }

    async fn execute(&self, task: &str) -> Result<String> {
        let result = self.runner.execute(task)?;
        Ok(format!(
            "[Claude] Found {} bugs. {}\n{}",
            result.bugs_found.len(),
            result.analysis_summary.unwrap_or_default(),
            result.stdout
        ))
    }

    fn is_available(&self) -> bool {
        std::process::Command::new("claude")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

struct OpenClawAdapter {
    runner: OpenClawRunner,
}

impl OpenClawAdapter {
    fn new(workspace: &str, timeout: u64) -> Self {
        Self {
            runner: OpenClawRunner::new(workspace, timeout),
        }
    }
}

#[async_trait]
impl Runner for OpenClawAdapter {
    fn name(&self) -> &str {
        "openclaw"
    }

    async fn execute(&self, task: &str) -> Result<String> {
        // Use bridge queue for full OpenClaw execution
        let task_id = extract_task_id(task).unwrap_or_else(|| "unknown".to_string());
        let persona_id = extract_persona_id(task).unwrap_or_else(|| "openclaw".to_string());
        
        self.runner.execute_bridge(&task_id, &persona_id, task).await
    }

    fn is_available(&self) -> bool {
        let gateway_port = std::env::var("OPENCLAW_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(18679u16);
        let url = format!("http://127.0.0.1:{}/status", gateway_port);
        
        reqwest::blocking::get(&url)
            .map(|r| r.status().is_success())
            .unwrap_or_else(|_| {
                reqwest::blocking::get("http://127.0.0.1:18789/status")
                    .map(|r| r.status().is_success())
                    .unwrap_or(false)
            })
    }
}

/// Extract task_id from prompt text (heuristic: looks for "Task ID: xxx")
fn extract_task_id(task: &str) -> Option<String> {
    task.lines()
        .find(|l| l.contains("Task ID:"))
        .and_then(|l| l.split("Task ID:").nth(1))
        .map(|s| s.trim().to_string())
}

/// Extract persona_id from prompt text (heuristic: first line after "You are the '")
fn extract_persona_id(task: &str) -> Option<String> {
    task.lines()
        .find(|l| l.contains("You are the '"))
        .and_then(|l| {
            let start = l.find("'")? + 1;
            let end = l[start..].find("'")?;
            Some(l[start..start+end].to_string())
        })
}
