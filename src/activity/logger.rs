use rusqlite::params;
use anyhow::Result;
use chrono::Utc;
use tracing::debug;
use uuid::Uuid;

use crate::db::Database;
use crate::models::ActivityLogEntry;

/// ActivityLogger provides a unified event stream for the swarm.
///
/// Every action (phase start, agent assignment, letter sent, file written,
/// test run, user approval, etc.) is logged to the activity_log table.
///
/// The web dashboard reads from this table to display the real-time feed.
pub struct ActivityLogger {
    db: Database,
}

impl ActivityLogger {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = Database::new(db_path)?;
        Ok(Self { db })
    }

    // =========================================================================
    // Convenience logging methods
    // =========================================================================

    /// Log a phase lifecycle event.
    pub fn log_phase_start(&self, story_id: &str, phase_id: &str, phase_name: &str) -> Result<()> {
        self.log(
            Some(story_id),
            Some(phase_id),
            "system",
            "phase_start",
            &format!("Phase '{}' started", phase_name),
        )
    }

    pub fn log_phase_complete(&self, story_id: &str, phase_id: &str, phase_name: &str) -> Result<()> {
        self.log(
            Some(story_id),
            Some(phase_id),
            "system",
            "phase_complete",
            &format!("Phase '{}' completed", phase_name),
        )
    }

    /// Log an agent action.
    pub fn log_agent_start(&self, story_id: &str, phase_id: &str, persona_id: &str) -> Result<()> {
        self.log(
            Some(story_id),
            Some(phase_id),
            "agent",
            "agent_start",
            &format!("Agent {} started work", persona_id),
        )
    }

    pub fn log_agent_complete(&self, story_id: &str, phase_id: &str, persona_id: &str, result: &str) -> Result<()> {
        self.log(
            Some(story_id),
            Some(phase_id),
            "agent",
            "agent_complete",
            &format!("Agent {} completed: {}", persona_id, result),
        )
    }

    /// Log a letter sent between agents.
    pub fn log_letter(&self, story_id: &str, from: &str, to: Option<&str>, content_preview: &str) -> Result<()> {
        let payload = match to {
            Some(t) => format!("Letter {} → {}: {}", from, t, content_preview),
            None => format!("Broadcast {} → all: {}", from, content_preview),
        };
        self.log(
            Some(story_id),
            None,
            "agent",
            "letter_send",
            &payload,
        )
    }

    /// Log a file write.
    pub fn log_file_write(&self, story_id: &str, file_path: &str, description: &str) -> Result<()> {
        self.log(
            Some(story_id),
            None,
            "agent",
            "file_write",
            &format!("Wrote {}: {}", file_path, description),
        )
    }

    /// Log a git commit.
    pub fn log_commit(&self, story_id: &str, commit_hash: &str, message: &str) -> Result<()> {
        self.log(
            Some(story_id),
            None,
            "agent",
            "commit",
            &format!("Commit {}: {}", commit_hash, message),
        )
    }

    /// Log a test run.
    pub fn log_test_run(&self, story_id: &str, passed: bool, details: &str) -> Result<()> {
        let _action = if passed { "test_pass" } else { "test_fail" };
        self.log(
            Some(story_id),
            None,
            "system",
            "test_run",
            &format!("Tests {}: {}", if passed { "PASSED" } else { "FAILED" }, details),
        )
    }

    /// Log a review submission.
    pub fn log_review(&self, story_id: &str, phase_id: &str, reviewer: &str, findings: &str) -> Result<()> {
        self.log(
            Some(story_id),
            Some(phase_id),
            "agent",
            "review_submit",
            &format!("Reviewer {}: {}", reviewer, findings),
        )
    }

    /// Log a user action (approve/reject/pause).
    pub fn log_user_approve(&self, story_id: &str, phase_id: &str, note: Option<&str>) -> Result<()> {
        let payload = match note {
            Some(n) => format!("User approved: {}", n),
            None => "User approved".to_string(),
        };
        self.log(
            Some(story_id),
            Some(phase_id),
            "user",
            "user_approve",
            &payload,
        )
    }

    pub fn log_user_reject(&self, story_id: &str, phase_id: &str, reason: &str) -> Result<()> {
        self.log(
            Some(story_id),
            Some(phase_id),
            "user",
            "user_reject",
            &format!("User rejected: {}", reason),
        )
    }

    pub fn log_user_pause(&self, story_id: &str, phase_id: &str, reason: &str) -> Result<()> {
        self.log(
            Some(story_id),
            Some(phase_id),
            "user",
            "user_pause",
            &format!("User paused: {}", reason),
        )
    }

    /// Log an error.
    pub fn log_error(&self, story_id: &str, phase_id: Option<&str>, error: &str) -> Result<()> {
        self.log(
            Some(story_id),
            phase_id,
            "system",
            "error",
            error,
        )
    }

    /// Log a replan event.
    pub fn log_replan(&self, story_id: &str, phase_id: &str, reason: &str) -> Result<()> {
        self.log(
            Some(story_id),
            Some(phase_id),
            "queen",
            "replan",
            &format!("Replan triggered: {}", reason),
        )
    }

    // =========================================================================
    // Query methods (for dashboard)
    // =========================================================================

    /// Get recent activity for a story (newest first).
    pub fn get_activity(&self, story_id: &str, limit: i64) -> Result<Vec<ActivityLogEntry>> {
        self.db.get_activity_for_story(story_id, limit)
    }

    /// Get activity stream formatted for SSE (Server-Sent Events).
    pub fn get_activity_stream(&self, story_id: &str, since: Option<&str>) -> Result<Vec<ActivityLogEntry>> {
        self.db.with_conn(|conn| {
            let query = if let Some(ts) = since {
                let mut stmt = conn.prepare(
                    "SELECT id, story_id, phase_id, actor_type, actor_id, action_type, payload, timestamp
                     FROM activity_log
                     WHERE story_id = ?1 AND timestamp > ?2
                     ORDER BY timestamp ASC"
                )?;
                let entries = stmt.query_map(params![story_id, ts], |row| {
                    Ok(ActivityLogEntry {
                        id: row.get(0)?,
                        story_id: row.get(1)?,
                        phase_id: row.get(2)?,
                        actor_type: row.get(3)?,
                        actor_id: row.get(4)?,
                        action_type: row.get(5)?,
                        payload: row.get(6)?,
                        timestamp: row.get::<_, String>(7)?.parse().unwrap_or_else(|_| Utc::now()),
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();
                Ok(entries)
            } else {
                let mut stmt = conn.prepare(
                    "SELECT id, story_id, phase_id, actor_type, actor_id, action_type, payload, timestamp
                     FROM activity_log
                     WHERE story_id = ?1
                     ORDER BY timestamp DESC
                     LIMIT 50"
                )?;
                let entries = stmt.query_map([story_id], |row| {
                    Ok(ActivityLogEntry {
                        id: row.get(0)?,
                        story_id: row.get(1)?,
                        phase_id: row.get(2)?,
                        actor_type: row.get(3)?,
                        actor_id: row.get(4)?,
                        action_type: row.get(5)?,
                        payload: row.get(6)?,
                        timestamp: row.get::<_, String>(7)?.parse().unwrap_or_else(|_| Utc::now()),
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();
                Ok(entries)
            };
            query
        })
    }

    // =========================================================================
    // Low-level log method
    // =========================================================================

    fn log(&self, story_id: Option<&str>, phase_id: Option<&str>,
        actor_type: &str, action_type: &str, payload: &str) -> Result<()> {
        let entry = ActivityLogEntry {
            id: Uuid::new_v4().to_string(),
            story_id: story_id.map(|s| s.to_string()),
            phase_id: phase_id.map(|s| s.to_string()),
            actor_type: actor_type.to_string(),
            actor_id: None,
            action_type: action_type.to_string(),
            payload: Some(payload.to_string()),
            timestamp: Utc::now(),
        };

        debug!("[Activity] {} | {}: {}", actor_type, action_type, payload);
        self.db.log_activity(&entry)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_db() -> String {
        let path = format!("/tmp/test_activity_{}.db", Uuid::new_v4());
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn test_log_and_query() {
        let db_path = temp_db();
        let logger = ActivityLogger::new(&db_path).unwrap();

        logger.log_phase_start("story-1", "phase-1", "Planning").unwrap();
        logger.log_agent_start("story-1", "phase-1", "planning_agent").unwrap();
        logger.log_agent_complete("story-1", "phase-1", "planning_agent", "Plan created").unwrap();

        let activity = logger.get_activity("story-1", 10).unwrap();
        assert_eq!(activity.len(), 3);

        let _ = fs::remove_file(&db_path);
    }
}
