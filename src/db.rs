use rusqlite::{Connection, params};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use anyhow::{Result, Context};
use serde_json::json;
use crate::models::*;
use chrono::Utc;
use uuid::Uuid;

pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::builder()
            .max_size(10)
            .build(manager)
            .with_context(|| format!("Failed to create connection pool for {}", path))?;
        
        let db = Self { pool };
        db.init_tables()?;
        Ok(db)
    }
    
    pub fn with_conn<F, R>(&self, f: F) -> Result<R>
    where F: FnOnce(&Connection) -> Result<R> {
        let conn = self.pool.get()
            .with_context(|| "Failed to get DB connection from pool")?;
        f(&conn)
    }
    
    // =========================================================================
    // Schema Initialization
    // =========================================================================
    
    fn init_tables(&self) -> Result<()> {
        self.with_conn(|conn| {
            // Tasks table (v2: hierarchical support)
            conn.execute(
                "CREATE TABLE IF NOT EXISTS tasks (
                    id TEXT PRIMARY KEY,
                    parent_id TEXT,
                    task_level TEXT DEFAULT 'task' CHECK(task_level IN ('epic','story','task','subtask')),
                    story_type TEXT DEFAULT 'sdlc_feature',
                    name TEXT NOT NULL,
                    description TEXT,
                    branch TEXT NOT NULL,
                    status TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    completed_at TEXT
                )",
                [],
            )?;
            
            // Task assignments (MxN matrix)
            conn.execute(
                "CREATE TABLE IF NOT EXISTS task_agents (
                    task_id TEXT NOT NULL,
                    persona_id TEXT NOT NULL,
                    personality_id TEXT NOT NULL,
                    mood TEXT NOT NULL,
                    reason TEXT,
                    assigned_at TEXT NOT NULL,
                    PRIMARY KEY (task_id, persona_id)
                )",
                [],
            )?;
            
            // Letters (inter-agent mail)
            conn.execute(
                "CREATE TABLE IF NOT EXISTS letters (
                    id TEXT PRIMARY KEY,
                    task_id TEXT NOT NULL,
                    from_persona TEXT NOT NULL,
                    to_persona TEXT,
                    content TEXT NOT NULL,
                    mood_at_send TEXT NOT NULL,
                    sent_at TEXT NOT NULL
                )",
                [],
            )?;
            
            // Diary entries (private agent journals)
            conn.execute(
                "CREATE TABLE IF NOT EXISTS diary_entries (
                    id TEXT PRIMARY KEY,
                    task_id TEXT NOT NULL,
                    persona_id TEXT NOT NULL,
                    personality_id TEXT NOT NULL,
                    entry TEXT NOT NULL,
                    mood TEXT NOT NULL,
                    written_at TEXT NOT NULL
                )",
                [],
            )?;
            
            // -- NEW: Story phases table (Phase A)
            conn.execute(
                "CREATE TABLE IF NOT EXISTS story_phases (
                    id TEXT PRIMARY KEY,
                    story_id TEXT NOT NULL,
                    phase_number INTEGER NOT NULL,
                    phase_name TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','running','blocked','reviewing','approved','rejected','skipped')),
                    topology TEXT DEFAULT 'sequential' CHECK(topology IN ('sequential','parallel','hybrid')),
                    started_at TEXT,
                    completed_at TEXT,
                    approved_by TEXT,
                    approval_note TEXT,
                    artifact_path TEXT,
                    UNIQUE(story_id, phase_number)
                )",
                [],
            )?;
            
            // -- NEW: Phase assignments (which agents work on which phase)
            conn.execute(
                "CREATE TABLE IF NOT EXISTS phase_assignments (
                    id TEXT PRIMARY KEY,
                    phase_id TEXT NOT NULL,
                    persona_id TEXT NOT NULL,
                    personality_id TEXT NOT NULL,
                    sub_task_description TEXT,
                    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','running','completed','failed')),
                    assigned_at TEXT,
                    completed_at TEXT,
                    result_summary TEXT,
                    FOREIGN KEY (phase_id) REFERENCES story_phases(id)
                )",
                [],
            )?;
            
            // -- NEW: Activity log (unified event stream)
            conn.execute(
                "CREATE TABLE IF NOT EXISTS activity_log (
                    id TEXT PRIMARY KEY,
                    story_id TEXT,
                    phase_id TEXT,
                    actor_type TEXT NOT NULL CHECK(actor_type IN ('agent','user','system','queen')),
                    actor_id TEXT,
                    action_type TEXT NOT NULL CHECK(action_type IN ('phase_start','phase_complete','agent_start','agent_complete','letter_send','file_write','commit','test_run','review_submit','user_approve','user_reject','user_pause','error','replan')),
                    payload TEXT,
                    timestamp TEXT DEFAULT CURRENT_TIMESTAMP
                )",
                [],
            )?;
            
            // -- NEW: Artifacts table (phase outputs)
            conn.execute(
                "CREATE TABLE IF NOT EXISTS artifacts (
                    id TEXT PRIMARY KEY,
                    story_id TEXT,
                    phase_id TEXT,
                    artifact_type TEXT NOT NULL CHECK(artifact_type IN ('plan','design','code','review','summary','test_report')),
                    file_path TEXT NOT NULL,
                    created_at TEXT,
                    summary TEXT
                )",
                [],
            )?;
            
            // -- NEW: Story dependencies (for epics)
            conn.execute(
                "CREATE TABLE IF NOT EXISTS story_dependencies (
                    story_id TEXT NOT NULL,
                    depends_on_story_id TEXT NOT NULL,
                    dependency_type TEXT DEFAULT 'hard' CHECK(dependency_type IN ('hard','soft')),
                    PRIMARY KEY(story_id, depends_on_story_id)
                )",
                [],
            )?;
            
            // -- NEW: Phase metrics (wall-clock + token tracking)
            conn.execute(
                "CREATE TABLE IF NOT EXISTS phase_metrics (
                    id TEXT PRIMARY KEY,
                    phase_id TEXT NOT NULL,
                    wall_clock_seconds REAL,
                    tokens_input INTEGER,
                    tokens_output INTEGER,
                    tokens_total INTEGER,
                    agent_invocations INTEGER,
                    created_at TEXT
                )",
                [],
            )?;
            
            // -- NEW: Indexes for performance
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_id)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_phases_story ON story_phases(story_id)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_activity_story ON activity_log(story_id)",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_artifacts_phase ON artifacts(phase_id)",
                [],
            )?;
            
            Ok(())
        })
    }
    
    // =========================================================================
    // Task CRUD
    // =========================================================================
    
    pub fn create_task(&self, task: &Task) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO tasks (id, name, description, branch, status, created_at, completed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    description = excluded.description,
                    branch = excluded.branch,
                    status = excluded.status,
                    completed_at = excluded.completed_at",
                params![
                    task.id,
                    task.name,
                    task.description,
                    task.branch,
                    format!("{:?}", task.status).to_lowercase(),
                    task.created_at.to_rfc3339(),
                    task.completed_at.map(|dt| dt.to_rfc3339())
                ],
            )?;
            Ok(())
        })
    }
    
    // -- NEW: Create task with hierarchy (Phase A)
    pub fn create_task_hierarchy(&self, task: &Task, parent_id: Option<&str>, level: &str, story_type: Option<&str>) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO tasks (id, parent_id, task_level, story_type, name, description, branch, status, created_at, completed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET
                    parent_id = excluded.parent_id,
                    task_level = excluded.task_level,
                    story_type = excluded.story_type,
                    name = excluded.name,
                    description = excluded.description,
                    branch = excluded.branch,
                    status = excluded.status,
                    completed_at = excluded.completed_at",
                params![
                    task.id,
                    parent_id,
                    level,
                    story_type.unwrap_or("sdlc_feature"),
                    task.name,
                    task.description,
                    task.branch,
                    format!("{:?}", task.status).to_lowercase(),
                    task.created_at.to_rfc3339(),
                    task.completed_at.map(|dt| dt.to_rfc3339())
                ],
            )?;
            Ok(())
        })
    }
    
    pub fn get_task(&self, task_id: &str) -> Result<Option<Task>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, branch, status, created_at, completed_at
                 FROM tasks WHERE id = ?1"
            )?;
            
            let task = stmt.query_row([task_id], |row| {
                let status_str: String = row.get(4)?;
                let status = match status_str.as_str() {
                    "queued" => TaskStatus::Queued,
                    "running" => TaskStatus::Running,
                    "paused" => TaskStatus::Paused,
                    "completed" => TaskStatus::Completed,
                    "failed" => TaskStatus::Failed,
                    _ => TaskStatus::Queued,
                };
                
                Ok(Task {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    branch: row.get(3)?,
                    status,
                    created_at: row.get::<_, String>(5)?.parse().unwrap_or_else(|_| Utc::now()),
                    completed_at: row.get::<_, Option<String>>(6)?.and_then(|s| s.parse().ok()),
                })
            }).ok();
            
            Ok(task)
        })
    }
    
    // -- NEW: Get child tasks (Phase A)
    pub fn get_child_tasks(&self, parent_id: &str) -> Result<Vec<Task>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, branch, status, created_at, completed_at
                 FROM tasks WHERE parent_id = ?1 ORDER BY created_at"
            )?;
            
            let tasks = stmt.query_map([parent_id], |row| {
                let status_str: String = row.get(4)?;
                let status = match status_str.as_str() {
                    "queued" => TaskStatus::Queued,
                    "running" => TaskStatus::Running,
                    "paused" => TaskStatus::Paused,
                    "completed" => TaskStatus::Completed,
                    "failed" => TaskStatus::Failed,
                    _ => TaskStatus::Queued,
                };
                
                Ok(Task {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    branch: row.get(3)?,
                    status,
                    created_at: row.get::<_, String>(5)?.parse().unwrap_or_else(|_| Utc::now()),
                    completed_at: row.get::<_, Option<String>>(6)?.and_then(|s| s.parse().ok()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
            
            Ok(tasks)
        })
    }
    
    // -- NEW: Get stories for epic (Phase D)
    pub fn get_stories_for_epic(&self, epic_id: &str) -> Result<Vec<serde_json::Value>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, parent_id, task_level, story_type, name, description, branch, status, created_at, completed_at
                 FROM tasks WHERE parent_id = ?1 AND task_level = 'story' ORDER BY created_at"
            )?;
            
            let stories = stmt.query_map([epic_id], |row| {
                let status_str: String = row.get(7)?;
                let status = match status_str.as_str() {
                    "queued" => TaskStatus::Queued,
                    "running" => TaskStatus::Running,
                    "paused" => TaskStatus::Paused,
                    "completed" => TaskStatus::Completed,
                    "failed" => TaskStatus::Failed,
                    _ => TaskStatus::Queued,
                };
                
                let story = json!({
                    "id": row.get::<_, String>(0)?,
                    "epic_id": row.get::<_, String>(1)?,
                    "task_level": row.get::<_, String>(2)?,
                    "story_type": row.get::<_, String>(3)?,
                    "name": row.get::<_, String>(4)?,
                    "description": row.get::<_, String>(5)?,
                    "branch": row.get::<_, String>(6)?,
                    "status": format!("{:?}", status).to_lowercase(),
                    "created_at": row.get::<_, String>(8)?,
                    "completed_at": row.get::<_, Option<String>>(9)?,
                });
                
                Ok(story)
            })?
            .filter_map(|r| r.ok())
            .collect();
            
            Ok(stories)
        })
    }
    
    // =========================================================================
    // Task Agent Assignments
    // =========================================================================
    
    pub fn assign_agent(&self, assignment: &Assignment) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO task_agents (task_id, persona_id, personality_id, mood, reason, assigned_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(task_id, persona_id) DO UPDATE SET
                    personality_id = excluded.personality_id,
                    mood = excluded.mood,
                    reason = excluded.reason,
                    assigned_at = excluded.assigned_at",
                params![
                    assignment.task_id,
                    assignment.persona_id,
                    assignment.personality_id,
                    assignment.mood,
                    assignment.reason,
                    assignment.assigned_at.to_rfc3339()
                ],
            )?;
            Ok(())
        })
    }
    
    pub fn get_task_assignments(&self, task_id: &str) -> Result<Vec<Assignment>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT task_id, persona_id, personality_id, mood, reason, assigned_at
                 FROM task_agents WHERE task_id = ?1"
            )?;
            
            let assignments = stmt.query_map([task_id], |row| {
                Ok(Assignment {
                    task_id: row.get(0)?,
                    persona_id: row.get(1)?,
                    personality_id: row.get(2)?,
                    mood: row.get(3)?,
                    reason: row.get(4)?,
                    assigned_at: row.get::<_, String>(5)?.parse().unwrap_or_else(|_| Utc::now()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
            
            Ok(assignments)
        })
    }
    
    // =========================================================================
    // Letters & Diary
    // =========================================================================
    
    pub fn write_letter(&self, letter: &Letter) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO letters (id, task_id, from_persona, to_persona, content, mood_at_send, sent_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    letter.id.to_string(),
                    letter.task_id,
                    letter.from_persona,
                    letter.to_persona,
                    letter.content,
                    letter.mood_at_send,
                    letter.sent_at.to_rfc3339()
                ],
            )?;
            Ok(())
        })
    }
    
    pub fn write_diary(&self, entry: &DiaryEntry) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO diary_entries (id, task_id, persona_id, personality_id, entry, mood, written_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    entry.id.to_string(),
                    entry.task_id,
                    entry.persona_id,
                    entry.personality_id,
                    entry.entry,
                    entry.mood,
                    entry.written_at.to_rfc3339()
                ],
            )?;
            Ok(())
        })
    }
    
    pub fn get_task_letters(&self, task_id: &str) -> Result<Vec<Letter>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, task_id, from_persona, to_persona, content, mood_at_send, sent_at
                 FROM letters WHERE task_id = ?1 ORDER BY sent_at"
            )?;
            
            let letters = stmt.query_map([task_id], |row| {
                Ok(Letter {
                    id: row.get::<_, String>(0)?.parse().unwrap_or_else(|_| Uuid::new_v4()),
                    task_id: row.get(1)?,
                    from_persona: row.get(2)?,
                    to_persona: row.get(3)?,
                    content: row.get(4)?,
                    mood_at_send: row.get(5)?,
                    sent_at: row.get::<_, String>(6)?.parse().unwrap_or_else(|_| Utc::now()),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
            
            Ok(letters)
        })
    }
    
    // =========================================================================
    // -- NEW: Phase CRUD (Phase A)
    // =========================================================================
    
    pub fn create_phase(&self, phase: &StoryPhase) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO story_phases (id, story_id, phase_number, phase_name, status, topology, started_at, completed_at, approved_by, approval_note, artifact_path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(id) DO UPDATE SET
                    status = excluded.status,
                    topology = excluded.topology,
                    started_at = excluded.started_at,
                    completed_at = excluded.completed_at,
                    approved_by = excluded.approved_by,
                    approval_note = excluded.approval_note,
                    artifact_path = excluded.artifact_path",
                params![
                    phase.id,
                    phase.story_id,
                    phase.phase_number,
                    phase.phase_name,
                    format!("{:?}", phase.status).to_lowercase(),
                    phase.topology,
                    phase.started_at.map(|dt| dt.to_rfc3339()),
                    phase.completed_at.map(|dt| dt.to_rfc3339()),
                    phase.approved_by,
                    phase.approval_note,
                    phase.artifact_path,
                ],
            )?;
            Ok(())
        })
    }
    
    pub fn get_phases_for_story(&self, story_id: &str) -> Result<Vec<StoryPhase>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, story_id, phase_number, phase_name, status, topology, started_at, completed_at, approved_by, approval_note, artifact_path
                 FROM story_phases WHERE story_id = ?1 ORDER BY phase_number"
            )?;
            
            let phases = stmt.query_map([story_id], |row| {
                let status_str: String = row.get(4)?;
                let status = match status_str.as_str() {
                    "pending" => PhaseStatus::Pending,
                    "running" => PhaseStatus::Running,
                    "blocked" => PhaseStatus::Blocked,
                    "reviewing" => PhaseStatus::Reviewing,
                    "approved" => PhaseStatus::Approved,
                    "rejected" => PhaseStatus::Rejected,
                    "skipped" => PhaseStatus::Skipped,
                    _ => PhaseStatus::Pending,
                };
                
                Ok(StoryPhase {
                    id: row.get(0)?,
                    story_id: row.get(1)?,
                    phase_number: row.get(2)?,
                    phase_name: row.get(3)?,
                    status,
                    topology: row.get(5)?,
                    started_at: row.get::<_, Option<String>>(6)?.and_then(|s| s.parse().ok()),
                    completed_at: row.get::<_, Option<String>>(7)?.and_then(|s| s.parse().ok()),
                    approved_by: row.get(8)?,
                    approval_note: row.get(9)?,
                    artifact_path: row.get(10)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
            
            Ok(phases)
        })
    }
    
    pub fn update_phase_status(&self, phase_id: &str, status: PhaseStatus) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE story_phases SET status = ?1 WHERE id = ?2",
                params![format!("{:?}", status).to_lowercase(), phase_id],
            )?;
            Ok(())
        })
    }
    
    // =========================================================================
    // -- NEW: Phase Assignment CRUD (Phase A)
    // =========================================================================
    
    pub fn create_phase_assignment(&self, assignment: &PhaseAssignment) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO phase_assignments (id, phase_id, persona_id, personality_id, sub_task_description, status, assigned_at, completed_at, result_summary)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    status = excluded.status,
                    completed_at = excluded.completed_at,
                    result_summary = excluded.result_summary",
                params![
                    assignment.id,
                    assignment.phase_id,
                    assignment.persona_id,
                    assignment.personality_id,
                    assignment.sub_task_description,
                    format!("{:?}", assignment.status).to_lowercase(),
                    assignment.assigned_at.map(|dt| dt.to_rfc3339()),
                    assignment.completed_at.map(|dt| dt.to_rfc3339()),
                    assignment.result_summary,
                ],
            )?;
            Ok(())
        })
    }
    
    pub fn get_phase_assignments(&self, phase_id: &str) -> Result<Vec<PhaseAssignment>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, phase_id, persona_id, personality_id, sub_task_description, status, assigned_at, completed_at, result_summary
                 FROM phase_assignments WHERE phase_id = ?1"
            )?;
            
            let assignments = stmt.query_map([phase_id], |row| {
                let status_str: String = row.get(5)?;
                let status = match status_str.as_str() {
                    "pending" => AssignmentStatus::Pending,
                    "running" => AssignmentStatus::Running,
                    "completed" => AssignmentStatus::Completed,
                    "failed" => AssignmentStatus::Failed,
                    _ => AssignmentStatus::Pending,
                };
                
                Ok(PhaseAssignment {
                    id: row.get(0)?,
                    phase_id: row.get(1)?,
                    persona_id: row.get(2)?,
                    personality_id: row.get(3)?,
                    sub_task_description: row.get(4)?,
                    status,
                    assigned_at: row.get::<_, Option<String>>(6)?.and_then(|s| s.parse().ok()),
                    completed_at: row.get::<_, Option<String>>(7)?.and_then(|s| s.parse().ok()),
                    result_summary: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
            
            Ok(assignments)
        })
    }
    
    // =========================================================================
    // -- NEW: Activity Log (Phase A)
    // =========================================================================
    
    pub fn log_activity(&self, entry: &ActivityLogEntry) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO activity_log (id, story_id, phase_id, actor_type, actor_id, action_type, payload, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    entry.id,
                    entry.story_id,
                    entry.phase_id,
                    entry.actor_type,
                    entry.actor_id,
                    entry.action_type,
                    entry.payload,
                    entry.timestamp.to_rfc3339(),
                ],
            )?;
            Ok(())
        })
    }
    
    pub fn get_activity_for_story(&self, story_id: &str, limit: i64) -> Result<Vec<ActivityLogEntry>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, story_id, phase_id, actor_type, actor_id, action_type, payload, timestamp
                 FROM activity_log WHERE story_id = ?1 ORDER BY timestamp DESC LIMIT ?2"
            )?;
            
            let entries = stmt.query_map(params![story_id, limit], |row| {
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
        })
    }
    
    // =========================================================================
    // -- NEW: Artifact CRUD (Phase A)
    // =========================================================================
    
    pub fn create_artifact(&self, artifact: &Artifact) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO artifacts (id, story_id, phase_id, artifact_type, file_path, created_at, summary)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(id) DO UPDATE SET
                    summary = excluded.summary",
                params![
                    artifact.id,
                    artifact.story_id,
                    artifact.phase_id,
                    artifact.artifact_type,
                    artifact.file_path,
                    artifact.created_at.map(|dt| dt.to_rfc3339()),
                    artifact.summary,
                ],
            )?;
            Ok(())
        })
    }
    
    pub fn get_artifacts_for_phase(&self, phase_id: &str) -> Result<Vec<Artifact>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, story_id, phase_id, artifact_type, file_path, created_at, summary
                 FROM artifacts WHERE phase_id = ?1 ORDER BY created_at"
            )?;
            
            let artifacts = stmt.query_map([phase_id], |row| {
                Ok(Artifact {
                    id: row.get(0)?,
                    story_id: row.get(1)?,
                    phase_id: row.get(2)?,
                    artifact_type: row.get(3)?,
                    file_path: row.get(4)?,
                    created_at: row.get::<_, Option<String>>(5)?.and_then(|s| s.parse().ok()),
                    summary: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
            
            Ok(artifacts)
        })
    }
    
    // =========================================================================
    // -- NEW: Story Dependency CRUD (Phase A)
    // =========================================================================
    
    pub fn add_story_dependency(&self, dep: &StoryDependency) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO story_dependencies (story_id, depends_on_story_id, dependency_type)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(story_id, depends_on_story_id) DO UPDATE SET
                    dependency_type = excluded.dependency_type",
                params![dep.story_id, dep.depends_on_story_id, dep.dependency_type],
            )?;
            Ok(())
        })
    }
    
    pub fn get_story_dependencies(&self, story_id: &str) -> Result<Vec<StoryDependency>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT story_id, depends_on_story_id, dependency_type
                 FROM story_dependencies WHERE story_id = ?1"
            )?;
            
            let deps = stmt.query_map([story_id], |row| {
                Ok(StoryDependency {
                    story_id: row.get(0)?,
                    depends_on_story_id: row.get(1)?,
                    dependency_type: row.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
            
            Ok(deps)
        })
    }
    
    // =========================================================================
    // -- NEW: Phase Metrics CRUD (Phase A)
    // =========================================================================
    
    pub fn create_phase_metrics(&self, metrics: &PhaseMetrics) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO phase_metrics (id, phase_id, wall_clock_seconds, tokens_input, tokens_output, tokens_total, agent_invocations, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET
                    wall_clock_seconds = excluded.wall_clock_seconds,
                    tokens_input = excluded.tokens_input,
                    tokens_output = excluded.tokens_output,
                    tokens_total = excluded.tokens_total,
                    agent_invocations = excluded.agent_invocations",
                params![
                    metrics.id,
                    metrics.phase_id,
                    metrics.wall_clock_seconds,
                    metrics.tokens_input,
                    metrics.tokens_output,
                    metrics.tokens_total,
                    metrics.agent_invocations,
                    metrics.created_at.map(|dt| dt.to_rfc3339()),
                ],
            )?;
            Ok(())
        })
    }
    
    pub fn get_phase_metrics(&self, phase_id: &str) -> Result<Option<PhaseMetrics>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, phase_id, wall_clock_seconds, tokens_input, tokens_output, tokens_total, agent_invocations, created_at
                 FROM phase_metrics WHERE phase_id = ?1"
            )?;
            
            let metrics = stmt.query_row([phase_id], |row| {
                Ok(PhaseMetrics {
                    id: row.get(0)?,
                    phase_id: row.get(1)?,
                    wall_clock_seconds: row.get(2)?,
                    tokens_input: row.get(3)?,
                    tokens_output: row.get(4)?,
                    tokens_total: row.get(5)?,
                    agent_invocations: row.get(6)?,
                    created_at: row.get::<_, Option<String>>(7)?.and_then(|s| s.parse().ok()),
                })
            }).ok();
            
            Ok(metrics)
        })
    }
}
