use rusqlite::{Connection, params};
use anyhow::{Result, Context};
use tracing::{info, warn};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;
use std::path::Path;
use crate::bridge::queue::BridgeQueue;

/// OpenClaw Runner — Dispatches tasks to the OpenClaw gateway via bridge queue.
/// 
/// Pattern: SQLite queue (durable, observable, zero extra deps)
/// 1. Swarm writes task to bridge_queue table
/// 2. OpenClaw poller reads pending tasks
/// 3. OpenClaw executes via sessions_spawn
/// 4. Results written back to bridge_queue
/// 5. Runner polls for result and returns
/// 
/// Also supports local file/exec ops for fast-path operations.
#[allow(dead_code)]
pub struct OpenClawRunner {
    gateway_url: String,
    gateway_token: String,
    timeout: Duration,
    workspace: String,
    db_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenClawResult {
    pub action: String,
    pub success: bool,
    pub output: String,
    pub files_affected: Vec<String>,
    pub urls_fetched: Vec<String>,
    pub messages_sent: Vec<String>,
}

impl OpenClawRunner {
    pub fn new(workspace: &str, timeout_secs: u64) -> Self {
        Self {
            gateway_url: "http://127.0.0.1:18789".to_string(),
            gateway_token: std::env::var("OPENCLAW_TOKEN")
                .unwrap_or_else(|_| "0124d10567d41ba2afbc196450fcc612".to_string()),
            timeout: Duration::from_secs(timeout_secs),
            workspace: workspace.to_string(),
            db_path: format!("{}/openclaw-swarm.db", workspace),
        }
    }

    /// Execute a task via the bridge queue.
    /// Enqueues the task, polls for completion, returns result.
    pub async fn execute_bridge(&self,
        task_id: &str,
        persona_id: &str,
        prompt: &str,
    ) -> Result<String> {
        let queue = BridgeQueue::new(&self.db_path)?;
        
        // Enqueue the task
        let bridge_task = queue.enqueue(
            task_id,
            persona_id,
            prompt,
            &self.workspace,
            "main", // default branch
        )?;
        
        info!("[OpenClawRunner] Enqueued bridge task {} for {}", bridge_task.id, persona_id);
        
        // Poll for completion (with timeout)
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_secs(2);
        let max_wait = self.timeout;
        
        while start.elapsed() < max_wait {
            if let Some(task) = queue.get_task(&bridge_task.id)? {
                match task.status {
                    crate::bridge::queue::BridgeStatus::Completed => {
                        return Ok(task.result.unwrap_or_else(|| "Completed (no output)".to_string()));
                    }
                    crate::bridge::queue::BridgeStatus::Failed => {
                        return Err(anyhow::anyhow!(
                            "Bridge task {} failed: {}",
                            bridge_task.id,
                            task.error.unwrap_or_default()
                        ));
                    }
                    _ => {
                        // Still pending/dispatched/running — wait and poll again
                        tokio::time::sleep(poll_interval).await;
                    }
                }
            } else {
                return Err(anyhow::anyhow!("Bridge task {} disappeared from queue", bridge_task.id));
            }
        }
        
        warn!("[OpenClawRunner] Bridge task {} timed out after {:?}", bridge_task.id, max_wait);
        Err(anyhow::anyhow!("Bridge task timed out after {:?}", max_wait))
    }

    /// Legacy: Execute a file operation (read, write, edit) directly.
    pub fn file_op(&self, op: FileOp) -> Result<OpenClawResult> {
        info!("[OpenClawRunner] File op: {:?}", op);

        let result = match &op {
            FileOp::Read { path } => self.file_read(&path),
            FileOp::Write { path, content } => self.file_write(&path, &content),
            FileOp::Edit { path, old_text, new_text } => self.file_edit(&path, &old_text, &new_text),
        }?;

        Ok(OpenClawResult {
            action: format!("{:?}", op),
            success: true,
            output: result,
            files_affected: vec![],
            urls_fetched: vec![],
            messages_sent: vec![],
        })
    }

    /// Legacy: Execute a shell command in the workspace.
    pub fn exec_command(&self, command: &str, args: &[&str]) -> Result<OpenClawResult> {
        info!("[OpenClawRunner] Exec: {} {}", command, args.join(" "));

        let output = Command::new(command)
            .args(args)
            .current_dir(&self.workspace)
            .output()
            .with_context(|| format!("Failed to execute: {} {}", command, args.join(" ")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(OpenClawResult {
            action: format!("exec: {} {}", command, args.join(" ")),
            success: output.status.success(),
            output: format!("{stdout}\n{stderr}"),
            files_affected: vec![],
            urls_fetched: vec![],
            messages_sent: vec![],
        })
    }

    /// Memory search — query the swarm knowledge base.
    pub fn memory_search(&self, query: &str) -> Result<OpenClawResult> {
        info!("[OpenClawRunner] Memory search: {}", query);

        let db_path = Path::new(&self.workspace).join("scripts").join("swarm_knowledge.db");
        
        let output = if db_path.exists() {
            let conn = Connection::open(&db_path)
                .with_context(|| format!("Failed to open knowledge DB at {}", db_path.display()))?;
            
            let mut results = vec![];
            results.push(format!("Query: '{}'", query));
            
            let chunks: Vec<(String, f64)> = conn.prepare(
                "SELECT content, rank FROM chunks_fts WHERE chunks_fts MATCH ?1 ORDER BY rank LIMIT 5"
            ).and_then(|mut stmt| {
                stmt.query_map(params![query], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
                })?.collect::<Result<Vec<_>, _>>()
            }).unwrap_or_default();
            
            if !chunks.is_empty() {
                results.push("--- Top Knowledge Chunks ---".to_string());
                for (i, (content, rank)) in chunks.iter().enumerate() {
                    let preview = if content.len() > 200 { &content[..200] } else { content };
                    results.push(format!("{}. [score: {:.3}] {}", i+1, rank, preview));
                }
            }
            
            let pattern = format!("%{}%", query);
            let concepts: Vec<(String, Option<String>)> = conn.prepare(
                "SELECT name, definition FROM concepts WHERE name LIKE ?1 OR definition LIKE ?1 LIMIT 5"
            ).and_then(|mut stmt| {
                stmt.query_map(params![&pattern], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
                })?.collect::<Result<Vec<_>, _>>()
            }).unwrap_or_default();
            
            if !concepts.is_empty() {
                results.push("\n--- Related Concepts ---".to_string());
                for (name, def) in concepts.iter().take(5) {
                    let d = def.as_deref().unwrap_or("");
                    let preview = if d.len() > 100 { &d[..100] } else { d };
                    results.push(format!("- {}: {}", name, preview));
                }
            }
            
            if chunks.is_empty() && concepts.is_empty() {
                results.push("No results found. Try a different query.".to_string());
            }
            
            results.join("\n")
        } else {
            format!("Knowledge DB not found at {}. Run ingest.py first.", db_path.display())
        };

        Ok(OpenClawResult {
            action: format!("memory_search: {}", query),
            success: true,
            output,
            files_affected: vec![],
            urls_fetched: vec![],
            messages_sent: vec![],
        })
    }

    // Internal helpers
    fn file_read(&self, path: &str) -> Result<String> {
        let full_path = Path::new(&self.workspace).join(path);
        std::fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read {}", full_path.display()))
    }

    fn file_write(&self, path: &str, content: &str) -> Result<String> {
        let full_path = Path::new(&self.workspace).join(path);
        std::fs::write(&full_path, content)
            .with_context(|| format!("Failed to write {}", full_path.display()))?;
        Ok(format!("Wrote {} bytes to {}", content.len(), full_path.display()))
    }

    fn file_edit(&self, path: &str, old_text: &str, new_text: &str) -> Result<String> {
        let full_path = Path::new(&self.workspace).join(path);
        let content = std::fs::read_to_string(&full_path)?;
        let new_content = content.replace(old_text, new_text);
        std::fs::write(&full_path, new_content)?;
        Ok(format!("Edited {}", full_path.display()))
    }
}

#[derive(Debug, Clone)]
pub enum FileOp {
    Read { path: String },
    Write { path: String, content: String },
    Edit { path: String, old_text: String, new_text: String },
}

#[derive(Debug, Clone)]
pub enum BrowserAction {
    Navigate { url: String },
    Screenshot { full_page: bool },
    Click { ref_: String },
}
