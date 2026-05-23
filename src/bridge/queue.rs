use rusqlite::{Connection, params};
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use chrono::Utc;
use uuid::Uuid;

/// A task request queued for OpenClaw execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeTask {
    pub id: String,
    pub task_id: String,
    pub persona_id: String,
    pub prompt: String,
    pub workspace: String,
    pub branch: String,
    pub status: BridgeStatus,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeStatus {
    Pending,
    Dispatched,
    Running,
    Completed,
    Failed,
}

impl BridgeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BridgeStatus::Pending => "pending",
            BridgeStatus::Dispatched => "dispatched",
            BridgeStatus::Running => "running",
            BridgeStatus::Completed => "completed",
            BridgeStatus::Failed => "failed",
        }
    }
}

/// SQLite-backed queue for OpenClaw bridge tasks.
pub struct BridgeQueue {
    conn: Connection,
}

impl BridgeQueue {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open bridge queue DB at {}", db_path))?;
        let queue = Self { conn };
        queue.init_tables()?;
        Ok(queue)
    }

    fn init_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS bridge_queue (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                persona_id TEXT NOT NULL,
                prompt TEXT NOT NULL,
                workspace TEXT NOT NULL,
                branch TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT,
                result TEXT,
                error TEXT
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_bridge_status ON bridge_queue(status)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_bridge_task ON bridge_queue(task_id)",
            [],
        )?;

        Ok(())
    }

    /// Enqueue a new task for OpenClaw execution.
    pub fn enqueue(
        &self,
        task_id: &str,
        persona_id: &str,
        prompt: &str,
        workspace: &str,
        branch: &str,
    ) -> Result<BridgeTask> {
        let task = BridgeTask {
            id: Uuid::new_v4().to_string(),
            task_id: task_id.to_string(),
            persona_id: persona_id.to_string(),
            prompt: prompt.to_string(),
            workspace: workspace.to_string(),
            branch: branch.to_string(),
            status: BridgeStatus::Pending,
            created_at: Utc::now().to_rfc3339(),
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
        };

        self.conn.execute(
            "INSERT INTO bridge_queue (id, task_id, persona_id, prompt, workspace, branch, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                task.id, task.task_id, task.persona_id, task.prompt,
                task.workspace, task.branch, task.status.as_str(), task.created_at
            ],
        )?;

        Ok(task)
    }

    /// Get all pending tasks (for the poller to consume).
    pub fn get_pending(&self, limit: usize) -> Result<Vec<BridgeTask>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, persona_id, prompt, workspace, branch, status,
                    created_at, started_at, completed_at, result, error
             FROM bridge_queue WHERE status = 'pending' ORDER BY created_at LIMIT ?1"
        )?;

        let tasks = stmt
            .query_map([limit as i64], |row| {
                Ok(BridgeTask {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    persona_id: row.get(2)?,
                    prompt: row.get(3)?,
                    workspace: row.get(4)?,
                    branch: row.get(5)?,
                    status: match row.get::<_, String>(6)?.as_str() {
                        "pending" => BridgeStatus::Pending,
                        "dispatched" => BridgeStatus::Dispatched,
                        "running" => BridgeStatus::Running,
                        "completed" => BridgeStatus::Completed,
                        "failed" => BridgeStatus::Failed,
                        _ => BridgeStatus::Pending,
                    },
                    created_at: row.get(7)?,
                    started_at: row.get(8)?,
                    completed_at: row.get(9)?,
                    result: row.get(10)?,
                    error: row.get(11)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(tasks)
    }

    /// Mark a task as dispatched (OpenClaw picked it up).
    pub fn mark_dispatched(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE bridge_queue SET status = 'dispatched', started_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    /// Mark a task as completed with result.
    pub fn mark_completed(&self, id: &str, result: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE bridge_queue SET status = 'completed', completed_at = ?1, result = ?2 WHERE id = ?3",
            params![Utc::now().to_rfc3339(), result, id],
        )?;
        Ok(())
    }

    /// Mark a task as failed with error.
    pub fn mark_failed(&self, id: &str, error: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE bridge_queue SET status = 'failed', completed_at = ?1, error = ?2 WHERE id = ?3",
            params![Utc::now().to_rfc3339(), error, id],
        )?;
        Ok(())
    }

    /// Get a task by ID.
    pub fn get_task(&self, id: &str) -> Result<Option<BridgeTask>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, persona_id, prompt, workspace, branch, status,
                    created_at, started_at, completed_at, result, error
             FROM bridge_queue WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(BridgeTask {
                id: row.get(0)?,
                task_id: row.get(1)?,
                persona_id: row.get(2)?,
                prompt: row.get(3)?,
                workspace: row.get(4)?,
                branch: row.get(5)?,
                status: match row.get::<_, String>(6)?.as_str() {
                    "pending" => BridgeStatus::Pending,
                    "dispatched" => BridgeStatus::Dispatched,
                    "running" => BridgeStatus::Running,
                    "completed" => BridgeStatus::Completed,
                    "failed" => BridgeStatus::Failed,
                    _ => BridgeStatus::Pending,
                },
                created_at: row.get(7)?,
                started_at: row.get(8)?,
                completed_at: row.get(9)?,
                result: row.get(10)?,
                error: row.get(11)?,
            }))
        } else {
            Ok(None)
        }
    }
}
