use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Represents a discovered file in the workspace.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub relative_path: String,
    pub size: u64,
    pub modified: u64,
    pub is_rust: bool,
    pub is_test: bool,
}

/// Scans the workspace and builds context for each phase.
pub struct WorkspaceSnapshot {
    workspace_path: PathBuf,
}

impl WorkspaceSnapshot {
    pub fn new(workspace_path: &str) -> Result<Self> {
        let path = PathBuf::from(workspace_path);
        if !path.exists() {
            anyhow::bail!("Workspace path does not exist: {}", workspace_path);
        }
        Ok(WorkspaceSnapshot { workspace_path: path })
    }

    /// Scan all files in the workspace.
    pub fn scan(&self) -> Result<Vec<FileEntry>> {
        let mut entries = Vec::new();
        self.scan_dir(&self.workspace_path, &mut entries)?;
        entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(entries)
    }

    fn scan_dir(&self, dir: &Path, entries: &mut Vec<FileEntry>) -> Result<()> {
        if let Ok(reader) = fs::read_dir(dir) {
            for entry in reader.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Skip target and hidden dirs.
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    if name != "target" && !name.starts_with('.') {
                        self.scan_dir(&path, entries)?;
                    }
                } else {
                    let metadata = entry.metadata()?;
                    let relative = path.strip_prefix(&self.workspace_path)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    let is_rust = relative.ends_with(".rs");
                    let is_test = relative.contains("test") || relative.contains("spec");
                    let modified = metadata.modified()
                        .unwrap_or(SystemTime::UNIX_EPOCH)
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();

                    entries.push(FileEntry {
                        path: path.to_string_lossy().to_string(),
                        relative_path: relative.clone(),
                        size: metadata.len(),
                        modified,
                        is_rust,
                        is_test,
                    });
                }
            }
        }
        Ok(())
    }

    /// Get only Rust source files.
    pub fn get_rust_files(&self) -> Result<Vec<String>> {
        let all = self.scan()?;
        Ok(all.into_iter()
            .filter(|e| e.is_rust)
            .map(|e| e.relative_path)
            .collect())
    }

    /// Read a file's content.
    pub fn get_file_content(&self, relative_path: &str) -> Result<String> {
        let full = self.workspace_path.join(relative_path);
        fs::read_to_string(&full)
            .with_context(|| format!("Failed to read {}", full.display()))
    }

    /// Get a file's modification timestamp.
    pub fn get_file_modification_time(&self, relative_path: &str) -> Result<u64> {
        let full = self.workspace_path.join(relative_path);
        let metadata = fs::metadata(&full)?;
        let modified = metadata.modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Ok(modified)
    }

    /// Build a handoff letter for the next phase.
    pub fn build_context_for_phase(
        &self,
        task_id: &str,
        prior_files: &[String],
        current_phase_name: &str,
        files_to_create: &[String],
    ) -> Result<String> {
        let all_files = self.scan()?;
        let rust_files: Vec<_> = all_files.into_iter()
            .filter(|e| e.is_rust)
            .collect();

        let mut lines = vec![
            format!("Workspace Snapshot for Task: {}", task_id),
            "=========================================".to_string(),
            String::new(),
            format!("Phase: {}", current_phase_name),
            String::new(),
            "Previously created files:".to_string(),
        ];

        for pf in prior_files {
            let found = rust_files.iter().find(|e| e.relative_path == *pf);
            if let Some(entry) = found {
                let size_kb = entry.size / 1024;
                lines.push(format!("  - {} ({} KB)", pf, size_kb));
            } else {
                lines.push(format!("  - {} (not yet on disk)", pf));
            }
        }

        lines.push(String::new());
        lines.push("Files you need to create:".to_string());
        for f in files_to_create {
            lines.push(format!("  - {}", f));
        }

        lines.push(String::new());
        lines.push("Existing Rust files in workspace:".to_string());
        for entry in &rust_files {
            if !prior_files.contains(&entry.relative_path) {
                let size_kb = entry.size / 1024;
                lines.push(format!("  - {} ({} KB)", entry.relative_path, size_kb));
            }
        }

        lines.push(String::new());
        lines.push("Instructions:".to_string());
        lines.push("1. Do NOT recreate files that already exist.".to_string());
        lines.push("2. Import from existing modules using `use crate::...`.".to_string());
        lines.push("3. Match existing code style and naming conventions.".to_string());
        lines.push("4. Keep it simple. Minimum code that works.".to_string());

        Ok(lines.join("\n"))
    }
}
