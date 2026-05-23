use anyhow::{Result, Context};
use std::process::Command;
use tracing::{info, warn, error};
use crate::models::*;

/// The Sandbox manages isolated execution environments.
/// Each task gets a branch + VM context where agents build behind closed doors.
pub struct Sandbox {
    base_branch: String,
    workspace_path: String,
}

impl Sandbox {
    pub fn new(base_branch: &str, workspace_path: &str) -> Self {
        Self {
            base_branch: base_branch.to_string(),
            workspace_path: workspace_path.to_string(),
        }
    }
    
    /// Create a new sandbox room (Git branch) for a task.
    pub fn create_room(&self, task: &Task) -> Result<SandboxRoom> {
        let branch_name = format!("swarm/{}/{}", 
            task.name.to_lowercase().replace(" ", "-"),
            task.id[..8].to_string()
        );
        
        // Create branch from base
        let output = Command::new("git")
            .args(["checkout", "-b", &branch_name, &self.base_branch])
            .current_dir(&self.workspace_path)
            .output()
            .with_context(|| "Failed to create git branch")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Git branch creation warning: {}", stderr);
            // Branch might already exist, that's ok
        }
        
        info!("Sandbox room created: branch {}", branch_name);
        
        Ok(SandboxRoom {
            task_id: task.id.clone(),
            branch: branch_name,
            path: self.workspace_path.clone(),
            status: SandboxStatus::Ready,
        })
    }
    
    /// Run a command in the sandbox (agent execution).
    pub fn execute_in_room(&self, room: &SandboxRoom, command: &str, args: &[&str]) -> Result<SandboxResult> {
        info!("Executing in sandbox {}: {} {}", room.branch, command, args.join(" "));
        
        let output = Command::new(command)
            .args(args)
            .current_dir(&room.path)
            .output()
            .with_context(|| format!("Failed to execute {} in sandbox", command))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        
        if !output.status.success() {
            error!("Sandbox execution failed: {}", stderr);
            return Ok(SandboxResult {
                success: false,
                stdout,
                stderr,
                exit_code: output.status.code().unwrap_or(-1),
            });
        }
        
        Ok(SandboxResult {
            success: true,
            stdout,
            stderr,
            exit_code: 0,
        })
    }
    
    /// Commit agent work to the sandbox branch.
    pub fn commit_work(&self, room: &SandboxRoom, agent_name: &str, message: &str) -> Result<()> {
        let full_message = format!("[{}] {}", agent_name, message);
        
        let _output = Command::new("git")
            .args(["add", "."])
            .current_dir(&room.path)
            .output()
            .with_context(|| "Git add failed")?;
        
        // Retry commit up to 3 times with backoff (handles concurrent index.lock)
        let mut last_err = None;
        for attempt in 1..=3 {
            let output = Command::new("git")
                .args(["commit", "-m", &full_message, "--allow-empty"])
                .current_dir(&room.path)
                .output();
            
            match output {
                Ok(o) if o.status.success() => {
                    info!("Agent {} committed to {}", agent_name, room.branch);
                    
                    // Auto-map codebase after commit (non-blocking)
                    let _ = crate::graphify_mapper::GraphifyMapper::quick_map(
                        &self.workspace_path);
                    
                    return Ok(());
                }
                Ok(o) => {
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    if stderr.contains("index.lock") || stderr.contains("cannot lock ref") {
                        warn!("Git lock conflict (attempt {}/3), retrying...", attempt);
                        std::thread::sleep(std::time::Duration::from_millis(500 * attempt as u64));
                        last_err = Some(stderr.to_string());
                    } else if stderr.contains("nothing to commit") {
                        return Ok(());
                    } else {
                        warn!("Git commit warning: {}", stderr);
                        return Ok(());
                    }
                }
                Err(e) => {
                    last_err = Some(e.to_string());
                    std::thread::sleep(std::time::Duration::from_millis(500 * attempt as u64));
                }
            }
        }
        
        Err(anyhow::anyhow!("Git commit failed after 3 retries: {:?}", last_err))
    }
    
    /// Merge the sandbox branch to main (ship it).
    pub fn ship(&self, room: &SandboxRoom) -> Result<()> {
        // Checkout main
        Command::new("git")
            .args(["checkout", &self.base_branch])
            .current_dir(&room.path)
            .output()
            .with_context(|| "Failed to checkout main")?;
        
        // Merge the swarm branch
        let output = Command::new("git")
            .args(["merge", &room.branch, "--no-ff", "-m", 
                  &format!("Merge swarm branch: {}", room.branch)])
            .current_dir(&room.path)
            .output()
            .with_context(|| "Failed to merge branch")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Merge failed: {}", stderr);
            return Err(anyhow::anyhow!("Merge conflict in sandbox {}: {}", room.branch, stderr));
        }
        
        info!("🚀 SHIPPED: {} merged to {}", room.branch, self.base_branch);
        
        Ok(())
    }
    
    /// Clean up sandbox (delete branch after merge).
    pub fn close_room(&self, room: &SandboxRoom) -> Result<()> {
        Command::new("git")
            .args(["branch", "-D", &room.branch])
            .current_dir(&room.path)
            .output()
            .with_context(|| "Failed to delete branch")?;
        
        info!("Sandbox room closed: {}", room.branch);
        Ok(())
    }
}

#[derive(Debug)]
pub struct SandboxRoom {
    pub task_id: String,
    pub branch: String,
    pub path: String,
    pub status: SandboxStatus,
}

#[derive(Debug, Clone)]
pub enum SandboxStatus {
    Ready,
    Building,
    Testing,
    ReadyToMerge,
    Merged,
    Failed,
}

#[derive(Debug)]
pub struct SandboxResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}
