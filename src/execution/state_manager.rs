use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde_json;

/// Represents a phase in the task decomposition.
#[derive(Debug, Clone)]
pub struct Phase {
    pub id: String,
    pub task_id: String,
    pub phase_number: i32,
    pub name: String,
    pub description: String,
    pub status: String,
    pub assigned_agent: String,
    pub files_expected: Vec<String>,
    pub files_created: Vec<String>,
    pub started_at: String,
    pub completed_at: String,
    pub error_output: String,
    pub retry_count: i32,
    pub handoff_letter: String,
}

impl Phase {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        let files_expected_json: String = row.get(7)?;
        let files_created_json: String = row.get(8)?;
        let files_expected: Vec<String> = serde_json::from_str(&files_expected_json).unwrap_or_default();
        let files_created: Vec<String> = serde_json::from_str(&files_created_json).unwrap_or_default();

        Ok(Phase {
            id: row.get(0)?,
            task_id: row.get(1)?,
            phase_number: row.get(2)?,
            name: row.get(3)?,
            description: row.get(4)?,
            status: row.get(5)?,
            assigned_agent: row.get(6)?,
            files_expected,
            files_created,
            started_at: row.get(9)?,
            completed_at: row.get(10)?,
            error_output: row.get(11)?,
            retry_count: row.get(12)?,
            handoff_letter: row.get(13)?,
        })
    }
}

/// Persists phase results to DB and loads them for next phase.
pub struct StateManager {
    conn: Connection,
}

impl StateManager {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open database at {}", db_path))?;
        Ok(StateManager { conn })
    }

    /// Insert a new phase.
    pub fn create_phase(&self, phase: &Phase) -> Result<()> {
        let files_expected_json = serde_json::to_string(&phase.files_expected)?;
        let files_created_json = serde_json::to_string(&phase.files_created)?;

        self.conn.execute(
            "INSERT INTO phases (id, task_id, phase_number, name, description, status,
                assigned_agent, files_expected, files_created, started_at, completed_at,
                error_output, retry_count, handoff_letter)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                phase.id, phase.task_id, phase.phase_number, phase.name,
                phase.description, phase.status, phase.assigned_agent,
                files_expected_json, files_created_json,
                phase.started_at, phase.completed_at,
                phase.error_output, phase.retry_count, phase.handoff_letter,
            ],
        )?;
        Ok(())
    }

    /// Get a single phase by ID.
    pub fn get_phase(&self, phase_id: &str) -> Result<Option<Phase>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, phase_number, name, description, status,
                    assigned_agent, files_expected, files_created,
                    started_at, completed_at, error_output, retry_count, handoff_letter
             FROM phases WHERE id = ?1"
        )?;
        let mut rows = stmt.query(params![phase_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Phase::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// Get all phases for a task, ordered by phase_number.
    pub fn get_phases_for_task(&self, task_id: &str) -> Result<Vec<Phase>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, phase_number, name, description, status,
                    assigned_agent, files_expected, files_created,
                    started_at, completed_at, error_output, retry_count, handoff_letter
             FROM phases WHERE task_id = ?1 ORDER BY phase_number"
        )?;
        let rows = stmt.query_map(params![task_id], |row| {
            Phase::from_row(row)
        })?;

        let mut phases = Vec::new();
        for row in rows {
            phases.push(row?);
        }
        Ok(phases)
    }

    /// Update phase status.
    pub fn update_phase_status(&self, phase_id: &str, status: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut stmt = self.conn.prepare(
            "UPDATE phases SET status = ?1, completed_at = ?2 WHERE id = ?3"
        )?;
        stmt.execute(params![status, now, phase_id])?;
        Ok(())
    }

    /// Update files_created for a phase.
    pub fn update_phase_files_created(&self, phase_id: &str, files: Vec<String>) -> Result<()> {
        let files_json = serde_json::to_string(&files)?;
        self.conn.execute(
            "UPDATE phases SET files_created = ?1 WHERE id = ?2",
            params![files_json, phase_id],
        )?;
        Ok(())
    }

    /// Update error output for a phase.
    pub fn update_phase_error(&self, phase_id: &str, error: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE phases SET error_output = ?1, status = 'failed' WHERE id = ?2",
            params![error, phase_id],
        )?;
        Ok(())
    }

    /// Get the last completed phase for a task.
    pub fn get_last_completed_phase(&self, task_id: &str) -> Result<Option<Phase>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, phase_number, name, description, status,
                    assigned_agent, files_expected, files_created,
                    started_at, completed_at, error_output, retry_count, handoff_letter
             FROM phases WHERE task_id = ?1 AND status = 'done'
             ORDER BY phase_number DESC LIMIT 1"
        )?;
        let mut rows = stmt.query(params![task_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Phase::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// Get all pending phases for a task.
    pub fn get_pending_phases(&self, task_id: &str) -> Result<Vec<Phase>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, phase_number, name, description, status,
                    assigned_agent, files_expected, files_created,
                    started_at, completed_at, error_output, retry_count, handoff_letter
             FROM phases WHERE task_id = ?1 AND status = 'pending'
             ORDER BY phase_number"
        )?;
        let rows = stmt.query_map(params![task_id], |row| {
            Phase::from_row(row)
        })?;

        let mut phases = Vec::new();
        for row in rows {
            phases.push(row?);
        }
        Ok(phases)
    }

    /// Get the next pending phase for a task.
    pub fn get_next_pending_phase(&self, task_id: &str) -> Result<Option<Phase>> {
        let phases = self.get_pending_phases(task_id)?;
        Ok(phases.into_iter().next())
    }

    /// Save an execution log entry.
    pub fn save_execution_log(
        &self,
        phase_id: &str,
        status: &str,
        files_written: Vec<String>,
        output: &str,
        error: &str,
    ) -> Result<()> {
        let id = uuid::Uuid::new_v4().to_string();
        let files_json = serde_json::to_string(&files_written)?;
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO phase_execution_log (id, phase_id, attempt_number, started_at,
                completed_at, status, files_written, output_summary, error_text)
             VALUES (?1, ?2,
                (SELECT COALESCE(MAX(attempt_number), 0) + 1 FROM phase_execution_log WHERE phase_id = ?3),
                ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, phase_id, phase_id, now, now, status, files_json, output, error],
        )?;
        Ok(())
    }

    /// Increment retry count for a phase.
    pub fn increment_retry_count(&self, phase_id: &str) -> Result<i32> {
        self.conn.execute(
            "UPDATE phases SET retry_count = retry_count + 1 WHERE id = ?1",
            params![phase_id],
        )?;
        self.get_retry_count(phase_id)
    }

    /// Get current retry count for a phase.
    pub fn get_retry_count(&self, phase_id: &str) -> Result<i32> {
        let mut stmt = self.conn.prepare("SELECT retry_count FROM phases WHERE id = ?1")?;
        let mut rows = stmt.query(params![phase_id])?;
        if let Some(row) = rows.next()? {
            let count: i32 = row.get(0)?;
            Ok(count)
        } else {
            Ok(0)
        }
    }
}
