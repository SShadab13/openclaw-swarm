use rusqlite::{Connection, params};
use anyhow::{Result, Context};
use crate::models::ErrorLog;
use chrono::Utc;
use uuid::Uuid;

/// Separate database for the Error Journal.
/// Tracks failures, root causes, and solutions for swarm learning.
pub struct ErrorJournal {
    conn: Connection,
}

impl ErrorJournal {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open error journal at {}", path))?;
        
        let journal = Self { conn };
        journal.init_tables()?;
        Ok(journal)
    }
    
    fn init_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS errors (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                persona_id TEXT NOT NULL,
                error_message TEXT NOT NULL,
                error_type TEXT NOT NULL,
                file_path TEXT,
                line_number INTEGER,
                root_cause TEXT,
                solution TEXT,
                same_symptom_different_cause INTEGER DEFAULT 0,
                occurred_at TEXT NOT NULL
            )",
            [],
        )?;
        
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS error_patterns (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                error_type TEXT NOT NULL UNIQUE,
                common_root_causes TEXT,  -- JSON array
                prevention_notes TEXT,
                first_seen TEXT NOT NULL,
                last_seen TEXT NOT NULL,
                occurrence_count INTEGER DEFAULT 1
            )",
            [],
        )?;
        
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_errors_type ON errors(error_type)",
            [],
        )?;
        
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_errors_task ON errors(task_id)",
            [],
        )?;
        
        Ok(())
    }
    
    pub fn log_error(&self, error: &ErrorLog) -> Result<()> {
        self.conn.execute(
            "INSERT INTO errors (id, task_id, persona_id, error_message, error_type,
             file_path, line_number, root_cause, solution, same_symptom_different_cause, occurred_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                error.id.to_string(),
                error.task_id,
                error.persona_id,
                error.error_message,
                error.error_type,
                error.file_path,
                error.line_number,
                error.root_cause,
                error.solution,
                error.same_symptom_different_cause as i32,
                error.occurred_at.to_rfc3339()
            ],
        )?;
        
        // Update pattern tracking
        self.update_pattern(&error.error_type, &error.root_cause)?;
        
        Ok(())
    }
    
    fn update_pattern(&self, error_type: &str, root_cause: &Option<String>) -> Result<()> {
        let existing: Option<(i32, String)> = self.conn.query_row(
            "SELECT occurrence_count, common_root_causes FROM error_patterns WHERE error_type = ?1",
            [error_type],
            |row| Ok((row.get(0)?, row.get(1)?))
        ).ok();
        
        if let Some((count, causes_json)) = existing {
            // Update existing pattern
            let mut causes: Vec<String> = serde_json::from_str(&causes_json).unwrap_or_default();
            if let Some(cause) = root_cause {
                if !causes.contains(cause) {
                    causes.push(cause.clone());
                }
            }
            
            self.conn.execute(
                "UPDATE error_patterns 
                 SET occurrence_count = ?1, 
                     last_seen = ?2,
                     common_root_causes = ?3
                 WHERE error_type = ?4",
                params![
                    count + 1,
                    Utc::now().to_rfc3339(),
                    serde_json::to_string(&causes)?,
                    error_type
                ],
            )?;
        } else {
            // Create new pattern
            let causes = root_cause.as_ref()
                .map(|c| vec![c.clone()])
                .unwrap_or_default();
            
            self.conn.execute(
                "INSERT INTO error_patterns (error_type, common_root_causes, first_seen, last_seen)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    error_type,
                    serde_json::to_string(&causes)?,
                    Utc::now().to_rfc3339(),
                    Utc::now().to_rfc3339()
                ],
            )?;
        }
        
        Ok(())
    }
    
    pub fn get_similar_errors(&self, error_type: &str) -> Result<Vec<ErrorLog>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, task_id, persona_id, error_message, error_type,
             file_path, line_number, root_cause, solution, 
             same_symptom_different_cause, occurred_at
             FROM errors WHERE error_type = ?1
             ORDER BY occurred_at DESC"
        )?;
        
        let errors = stmt.query_map([error_type], |row| {
            Ok(ErrorLog {
                id: row.get::<_, String>(0)?.parse().unwrap_or_else(|_| Uuid::new_v4()),
                task_id: row.get(1)?,
                persona_id: row.get(2)?,
                error_message: row.get(3)?,
                error_type: row.get(4)?,
                file_path: row.get(5)?,
                line_number: row.get(6)?,
                root_cause: row.get(7)?,
                solution: row.get(8)?,
                same_symptom_different_cause: row.get::<_, i32>(9)? != 0,
                occurred_at: row.get::<_, String>(10)?.parse().unwrap_or_else(|_| Utc::now()),
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
        
        Ok(errors)
    }
    
    pub fn get_pattern_stats(&self, error_type: &str) -> Result<Option<ErrorPatternStats>> {
        let result = self.conn.query_row(
            "SELECT error_type, occurrence_count, first_seen, last_seen, prevention_notes
             FROM error_patterns WHERE error_type = ?1",
            [error_type],
            |row| {
                Ok(ErrorPatternStats {
                    error_type: row.get(0)?,
                    occurrence_count: row.get(1)?,
                    first_seen: row.get(2)?,
                    last_seen: row.get(3)?,
                    prevention_notes: row.get(4)?,
                })
            }
        );
        
        Ok(result.ok())
    }
}

#[derive(Debug)]
pub struct ErrorPatternStats {
    pub error_type: String,
    pub occurrence_count: i32,
    pub first_seen: String,
    pub last_seen: String,
    pub prevention_notes: Option<String>,
}
