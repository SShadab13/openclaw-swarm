use anyhow::{Result, Context};
use tracing::{info, warn};
use std::process::Command;
use std::path::Path;

/// Auto-map the codebase using graphify after significant changes.
///
/// Called after sandbox commits to keep the knowledge graph in sync
/// with the actual code. Non-blocking — logs warnings but never fails
/// the main workflow if graphify is unavailable.
pub struct GraphifyMapper;

impl GraphifyMapper {
    /// Run graphify on the workspace and update the knowledge graph.
    pub fn map_workspace(workspace: &str) -> Result<()> {
        let workspace_path = Path::new(workspace);
        if !workspace_path.exists() {
            anyhow::bail!("Workspace path does not exist: {}", workspace);
        }

        // Check if graphify is installed
        let graphify_available = Command::new("graphify")
            .arg("--help")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !graphify_available {
            warn!("[Graphify] graphify CLI not found. Skipping auto-map.");
            return Ok(());
        }

        info!("[Graphify] Running auto-map on {}", workspace);

        // Run graphify update (re-extract code files and update the graph)
        let output = Command::new("graphify")
            .args([
                "update",
                ".",
                "--no-viz",
            ])
            .current_dir(workspace_path)
            .output()
            .with_context(|| "Failed to run graphify")?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            info!("[Graphify] Map complete. {}", stdout.lines().next().unwrap_or("OK"));
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("[Graphify] Map failed: {}", stderr);
        }

        Ok(())
    }

    /// Quick map — just update nodes/edges without full visualization.
    /// Called frequently (after every commit).
    pub fn quick_map(workspace: &str) -> Result<()> {
        // For now, same as full map but could be optimized
        Self::map_workspace(workspace)
    }

    /// Check if graphify output exists and is recent.
    pub fn is_fresh(workspace: &str, max_age_minutes: u64) -> bool {
        let graphify_out = Path::new(workspace).join("graphify-out").join("graph.json");
        if !graphify_out.exists() {
            return false;
        }

        if let Ok(metadata) = std::fs::metadata(&graphify_out) {
            if let Ok(modified) = metadata.modified() {
                let age = std::time::SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or_default();
                return age.as_secs() < max_age_minutes * 60;
            }
        }

        false
    }
}
