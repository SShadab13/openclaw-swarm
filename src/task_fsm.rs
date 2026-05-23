use anyhow::Result;
use chrono::Utc;
use tracing::{info, warn};
use crate::db::Database;

/// Task state machine with granular lifecycle tracking.
///
/// States:
///   Queued → Running → Review → ReadyToMerge → Merging → Merged
///                ↓
///            Blocked → Paused
///                ↓
///              Failed
///
/// Every transition is logged in task_steps table.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Queued,
    Running,
    Blocked,
    Paused,
    Review,         // All agents done, waiting for coordinator check
    ReadyToMerge,   // Coordinator confirms completion
    Merging,        // Sandbox ship in progress
    Merged,         // Successfully shipped
    Failed,         // Terminal failure
}

impl TaskState {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskState::Queued => "queued",
            TaskState::Running => "running",
            TaskState::Blocked => "blocked",
            TaskState::Paused => "paused",
            TaskState::Review => "review",
            TaskState::ReadyToMerge => "ready_to_merge",
            TaskState::Merging => "merging",
            TaskState::Merged => "merged",
            TaskState::Failed => "failed",
        }
    }

    /// Check if this transition is valid.
    pub fn can_transition_to(&self, next: &TaskState) -> bool {
        match (self, next) {
            (TaskState::Queued, TaskState::Running) => true,
            (TaskState::Running, TaskState::Blocked) => true,
            (TaskState::Running, TaskState::Review) => true,
            (TaskState::Running, TaskState::Paused) => true,
            (TaskState::Running, TaskState::Failed) => true,
            (TaskState::Blocked, TaskState::Running) => true,
            (TaskState::Blocked, TaskState::Paused) => true,
            (TaskState::Blocked, TaskState::Failed) => true,
            (TaskState::Paused, TaskState::Running) => true,
            (TaskState::Paused, TaskState::Failed) => true,
            (TaskState::Review, TaskState::ReadyToMerge) => true,
            (TaskState::Review, TaskState::Running) => true, // send back for more work
            (TaskState::Review, TaskState::Blocked) => true,
            (TaskState::ReadyToMerge, TaskState::Merging) => true,
            (TaskState::ReadyToMerge, TaskState::Running) => true, // Queen rejects merge
            (TaskState::Merging, TaskState::Merged) => true,
            (TaskState::Merging, TaskState::Failed) => true,
            (TaskState::Failed, TaskState::Queued) => true, // Retry
            _ => false,
        }
    }
}

/// The TaskFsm manages state transitions and logs every change.
pub struct TaskFsm {
    db: Database,
}

impl TaskFsm {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = Database::new(db_path)?;
        Ok(Self { db })
    }

    /// Transition a task to a new state. Validates the transition first.
    pub fn transition(
        &self,
        task_id: &str,
        from: TaskState,
        to: TaskState,
        reason: &str,
        triggered_by: &str,
    ) -> Result<()> {
        if !from.can_transition_to(&to) {
            warn!(
                "Invalid state transition for {}: {:?} → {:?}",
                task_id, from, to
            );
            anyhow::bail!(
                "Invalid transition: {:?} → {:?} for task {}",
                from, to, task_id
            );
        }

        self.log_transition(task_id, &from, &to, reason, triggered_by)?;

        info!(
            "Task {}: {:?} → {:?} (by: {}, reason: {})",
            task_id, from, to, triggered_by, reason
        );

        Ok(())
    }

    /// Log a state transition in the task_steps table.
    fn log_transition(
        &self,
        task_id: &str,
        from: &TaskState,
        to: &TaskState,
        reason: &str,
        triggered_by: &str,
    ) -> Result<()> {
        self.db.with_conn(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS task_steps (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    task_id TEXT NOT NULL,
                    from_state TEXT NOT NULL,
                    to_state TEXT NOT NULL,
                    reason TEXT,
                    triggered_by TEXT NOT NULL,
                    occurred_at TEXT NOT NULL
                )",
                [],
            )?;

            conn.execute(
                "INSERT INTO task_steps (task_id, from_state, to_state, reason, triggered_by, occurred_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    task_id,
                    from.as_str(),
                    to.as_str(),
                    reason,
                    triggered_by,
                    Utc::now().to_rfc3339()
                ],
            )?;

            // Also update the task's current status
            conn.execute(
                "UPDATE tasks SET status = ?1 WHERE id = ?2",
                rusqlite::params![to.as_str(), task_id],
            )?;

            Ok(())
        })
    }

    /// Get the current state of a task from the DB.
    pub fn get_state(&self, task_id: &str) -> Result<Option<TaskState>> {
        self.db.with_conn(|conn| {
            let result: Option<String> = conn
                .query_row(
                    "SELECT status FROM tasks WHERE id = ?1",
                    [task_id],
                    |row| row.get(0),
                )
                .ok();

            Ok(result.and_then(|s| match s.as_str() {
                "queued" => Some(TaskState::Queued),
                "running" => Some(TaskState::Running),
                "blocked" => Some(TaskState::Blocked),
                "paused" => Some(TaskState::Paused),
                "review" => Some(TaskState::Review),
                "ready_to_merge" => Some(TaskState::ReadyToMerge),
                "merging" => Some(TaskState::Merging),
                "merged" => Some(TaskState::Merged),
                "failed" => Some(TaskState::Failed),
                _ => None,
            }))
        })
    }

    /// Get transition history for a task.
    pub fn get_history(&self, task_id: &str) -> Result<Vec<TransitionRecord>> {
        self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT from_state, to_state, reason, triggered_by, occurred_at
                 FROM task_steps WHERE task_id = ?1 ORDER BY occurred_at"
            )?;

            let records = stmt
                .query_map([task_id], |row| {
                    Ok(TransitionRecord {
                        from_state: row.get(0)?,
                        to_state: row.get(1)?,
                        reason: row.get(2)?,
                        triggered_by: row.get(3)?,
                        occurred_at: row.get(4)?,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(records)
        })
    }
}

#[derive(Debug, Clone)]
pub struct TransitionRecord {
    pub from_state: String,
    pub to_state: String,
    pub reason: String,
    pub triggered_by: String,
    pub occurred_at: String,
}
