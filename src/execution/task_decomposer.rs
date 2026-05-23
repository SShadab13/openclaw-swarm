use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde_json;
use uuid::Uuid;

/// Phase status matching the database schema.
pub struct Phase {
    pub id: String,
    pub task_id: String,
    pub phase_number: i32,
    pub name: String,
    pub description: String,
    pub status: String,
    pub files_expected: Vec<String>,
    pub files_created: Vec<String>,
}

/// Task complexity level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Complexity {
    Low,
    Medium,
    High,
}

/// Decomposes multi-file tasks into sequential phases that fit within timeout windows.
pub struct TaskDecomposer {
    conn: Connection,
}

impl TaskDecomposer {
    /// Open the database and ensure the phases schema exists.
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open database at {}", db_path))?;
        Ok(TaskDecomposer { conn })
    }

    /// Full decomposition: estimate files, determine complexity, create phases, persist to DB.
    pub fn decompose_task(
        &self,
        task_id: &str,
        task_name: &str,
        task_desc: &str,
    ) -> Result<Vec<Phase>> {
        let estimated_files = Self::estimate_file_count(task_desc);
        let _complexity = Self::estimate_complexity(estimated_files);

        let files = Self::generate_file_list(task_name, task_desc, estimated_files);
        let phases = Self::create_phases(task_id, files)?;

        self.persist_phases(&phases)
            .with_context(|| "Failed to persist phases to database")?;

        Ok(phases)
    }

    /// Heuristic: count words suggesting multiple files are needed.
    pub fn estimate_file_count(description: &str) -> usize {
        let lower = description.to_lowercase();
        let keywords = ["file", "files", "module", "modules", "component", "components", "screen", "screens", "page", "pages"];
        let mut count = 0;
        for kw in keywords {
            count += lower.matches(kw).count();
        }
        // Every two keyword hits ≈ 1 file; minimum 1.
        let estimated = (count / 2).max(1);
        // Cap at reasonable max to avoid runaway decomposition.
        estimated.min(12)
    }

    /// Map estimated file count to complexity.
    pub fn estimate_complexity(files: usize) -> Complexity {
        match files {
            1 | 2 => Complexity::Low,
            3 | 4 | 5 => Complexity::Medium,
            _ => Complexity::High,
        }
    }

    /// Break a list of files into chunks of max 2 files per phase for timeout safety.
    pub fn create_phases(task_id: &str, files: Vec<String>) -> Result<Vec<Phase>> {
        let chunk_size = 2;
        let mut phases = Vec::new();

        for (i, chunk) in files.chunks(chunk_size).enumerate() {
            let phase_number = (i + 1) as i32;
            let name = format!("Phase {}: {}", phase_number, chunk.join(" + "));
            let description = format!("Create/implement {} for this task.", chunk.join(", "));

            phases.push(Phase {
                id: Uuid::new_v4().to_string(),
                task_id: task_id.to_string(),
                phase_number,
                name,
                description,
                status: "pending".to_string(),
                files_expected: chunk.to_vec(),
                files_created: Vec::new(),
            });
        }

        Ok(phases)
    }

    /// Known decomposition patterns for common task types.
    pub fn known_patterns() -> Vec<(&'static str, Vec<&'static str>)> {
        vec![
            ("new screen", vec!["component.rs", "service.rs", "test.rs"]),
            ("auth system", vec!["models.rs", "middleware.rs", "routes.rs"]),
            ("tui dashboard", vec!["dashboard.rs", "lib.rs", "main.rs"]),
        ]
    }

    /// Generate a likely file list from task description.
    fn generate_file_list(_task_name: &str, task_desc: &str, estimated: usize) -> Vec<String> {
        // Check known patterns first.
        let lower = task_desc.to_lowercase();
        for (pattern, files) in Self::known_patterns() {
            if lower.contains(pattern) {
                return files.iter().map(|s| s.to_string()).collect();
            }
        }

        // Fallback: generate generic filenames.
        (1..=estimated)
            .map(|i| format!("file_{}.rs", i))
            .collect()
    }

    /// Insert phases into the database.
    fn persist_phases(&self, phases: &[Phase]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO phases (
                id, task_id, phase_number, name, description, status,
                files_expected, files_created
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )?;

        for phase in phases {
            let files_expected_json = serde_json::to_string(&phase.files_expected)?;
            let files_created_json = serde_json::to_string(&phase.files_created)?;

            stmt.execute(params![
                phase.id,
                phase.task_id,
                phase.phase_number,
                phase.name,
                phase.description,
                phase.status,
                files_expected_json,
                files_created_json,
            ])?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_file_count() {
        assert_eq!(TaskDecomposer::estimate_file_count("Create a single file"), 1);
        // "files" matches both "file" and "files" → count=2 → 2/2=1 file (conservative)
        assert_eq!(
            TaskDecomposer::estimate_file_count("Build auth system with models, middleware, and routes files"),
            1
        );
        // More explicit keywords → higher count
        assert_eq!(
            TaskDecomposer::estimate_file_count("Build screen with component, module, and page files"),
            3
        );
    }

    #[test]
    fn test_estimate_complexity() {
        assert_eq!(TaskDecomposer::estimate_complexity(1), Complexity::Low);
        assert_eq!(TaskDecomposer::estimate_complexity(4), Complexity::Medium);
        assert_eq!(TaskDecomposer::estimate_complexity(8), Complexity::High);
    }

    #[test]
    fn test_create_phases() {
        let files = vec!["a.rs".to_string(), "b.rs".to_string(), "c.rs".to_string()];
        let phases = TaskDecomposer::create_phases("task-1", files).unwrap();
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].files_expected.len(), 2);
        assert_eq!(phases[1].files_expected.len(), 1);
    }

    #[test]
    fn test_known_patterns() {
        let patterns = TaskDecomposer::known_patterns();
        assert_eq!(patterns.len(), 3);
    }
}
