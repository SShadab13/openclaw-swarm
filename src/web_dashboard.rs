use anyhow::Result;
use axum::{
    routing::{get, post},
    Router, Json,
    extract::{Path, State},
    response::{sse::Event, Sse, IntoResponse},
    http::StatusCode,
};
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::Stream;
use tracing::info;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::Deserialize;

use crate::db::Database;
use crate::swarm_bus::SwarmBus;
use crate::models::Letter;
use crate::queen::Queen;

/// Web Dashboard v2 — Real-time agent monitoring + file tree + task execution + threaded chat.
pub struct WebDashboard {
    db_path: String,
    workspace: String,
    personas_dir: String,
    personalities_dir: String,
    bus: Arc<Mutex<SwarmBus>>,
}

impl WebDashboard {
    pub fn new(
        db_path: &str,
        workspace: &str,
        personas_dir: &str,
        personalities_dir: &str,
        bus: Arc<Mutex<SwarmBus>>,
    ) -> Self {
        Self {
            db_path: db_path.to_string(),
            workspace: workspace.to_string(),
            personas_dir: personas_dir.to_string(),
            personalities_dir: personalities_dir.to_string(),
            bus,
        }
    }

    pub async fn run(self, port: u16) -> Result<()> {
        let app = Router::new()
            .route("/", get(index_handler))
            .route("/events", get(sse_handler))
            .route("/api/tasks", get(tasks_handler))
            .route("/api/letters/{task_id}", get(letters_handler))
            .route("/api/status", get(status_handler))
            // v2 features
            .route("/api/files", get(files_handler))
            .route("/api/file/{*path}", get(file_content_handler))
            // Phase A endpoints
            .route("/api/phases/{story_id}", get(phases_handler))
            .route("/api/activity/{story_id}", get(activity_handler))
            .route("/api/artifacts/{phase_id}", get(artifacts_handler))
            // Phase D endpoints
            .route("/api/stories/{epic_id}", get(stories_handler))
            .route("/api/dependencies/{story_id}", get(dependencies_handler))
            .route("/api/agents/{phase_id}", get(agents_handler))
            .route("/api/phase/{phase_id}/start", post(start_phase_handler))
            .route("/api/phase/{phase_id}/complete", post(complete_phase_handler))
            .route("/api/phase/{phase_id}/skip", post(skip_phase_handler))
            .route("/api/phase/{phase_id}/block", post(block_phase_handler))
            .route("/api/phase/{phase_id}/unblock", post(unblock_phase_handler))
            // Gate endpoints
            .route("/api/phase/{phase_id}/approve", post(approve_phase_handler))
            .route("/api/phase/{phase_id}/reject", post(reject_phase_handler))
            .route("/api/phase/{phase_id}/replan", post(replan_phase_handler))
            .route("/api/task/create", post(create_task_handler))
            .route("/api/task/{task_id}/start", post(start_task_handler))
            .route("/api/task/{task_id}/run", post(run_task_handler))
            .with_state(AppState {
                db_path: self.db_path,
                workspace: self.workspace,
                personas_dir: self.personas_dir,
                personalities_dir: self.personalities_dir,
                bus: self.bus,
            });

        let addr = format!("0.0.0.0:{}", port);
        info!("[WebDashboard] Starting on http://{}", addr);
        info!("[WebDashboard] Open your browser to http://localhost:{}", port);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

#[derive(Clone)]
struct AppState {
    db_path: String,
    workspace: String,
    personas_dir: String,
    personalities_dir: String,
    bus: Arc<Mutex<SwarmBus>>,
}

/// Serve the embedded HTML dashboard.
async fn index_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("Content-Type", "text/html")],
        DASHBOARD_HTML,
    )
}

// ──────────────────────────────────────────
// SSE — Live Letter Stream
// ──────────────────────────────────────────

async fn sse_handler(State(state): State<AppState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, mut rx) = broadcast::channel::<Letter>(100);
    
    let bus = state.bus.clone();
    tokio::spawn(async move {
        loop {
            let task_ids = {
                let bus_guard = bus.lock().await;
                bus_guard.list_channels()
            };
            
            for task_id in task_ids {
                let bus_clone = bus.clone();
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    let bus_guard = bus_clone.lock().await;
                    if let Some(mut rx) = bus_guard.subscribe(&task_id) {
                        drop(bus_guard);
                        while let Ok(letter) = rx.recv().await {
                            let _ = tx_clone.send(letter);
                        }
                    }
                });
            }
            
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    let stream = async_stream::stream! {
        while let Ok(letter) = rx.recv().await {
            let data = json!({
                "id": letter.id.to_string(),
                "task_id": letter.task_id,
                "from": letter.from_persona,
                "to": letter.to_persona,
                "content": letter.content,
                "mood": letter.mood_at_send,
                "sent_at": letter.sent_at.to_rfc3339(),
            });
            yield Ok(Event::default().data(data.to_string()));
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

// ──────────────────────────────────────────
// API: Tasks, Letters, Status
// ──────────────────────────────────────────

async fn tasks_handler(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = Database::new(&state.db_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tasks = db.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, branch, status, created_at FROM tasks ORDER BY created_at DESC"
        ).map_err(|e| anyhow::anyhow!("{}", e))?;
        
        let tasks: Result<Vec<_>, _> = stmt.query_map([], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "description": row.get::<_, String>(2)?,
                "branch": row.get::<_, String>(3)?,
                "status": row.get::<_, String>(4)?,
                "created_at": row.get::<_, String>(5)?,
            }))
        })?.collect();
        
        tasks.map_err(|e| anyhow::anyhow!("{}", e))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(json!({ "tasks": tasks })))
}

async fn letters_handler(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = Database::new(&state.db_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let letters = db.get_task_letters(&task_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let letters_json: Vec<_> = letters.into_iter().map(|l| json!({
        "id": l.id.to_string(),
        "from": l.from_persona,
        "to": l.to_persona,
        "content": l.content,
        "mood": l.mood_at_send,
        "sent_at": l.sent_at.to_rfc3339(),
    })).collect();
    
    Ok(Json(json!({ "letters": letters_json })))
}

async fn status_handler(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = Database::new(&state.db_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let task_count: i64 = db.with_conn(|conn| {
        conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
            .map_err(|e| anyhow::anyhow!("{}", e))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let letter_count: i64 = db.with_conn(|conn| {
        conn.query_row("SELECT COUNT(*) FROM letters", [], |row| row.get(0))
            .map_err(|e| anyhow::anyhow!("{}", e))
    }).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(json!({
        "tasks_total": task_count,
        "letters_total": letter_count,
        "dashboard_version": "v0.2-web",
        "status": "operational",
    })))
}

// ──────────────────────────────────────────
// v2 FEATURE 1: File Tree Browser
// ──────────────────────────────────────────

async fn files_handler(State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let tree = build_file_tree(&state.workspace)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "files": tree })))
}

fn build_file_tree(root: &str) -> Result<Vec<serde_json::Value>> {
    let mut entries = Vec::new();
    let path = std::path::Path::new(root);
    if !path.exists() {
        return Ok(entries);
    }
    
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path_str = entry.path().to_string_lossy().to_string();
        let is_dir = entry.file_type()?.is_dir();
        
        // Skip .git and target directories
        if name.starts_with('.') || name == "target" {
            continue;
        }
        
        let mut item = json!({
            "name": name,
            "path": path_str,
            "is_dir": is_dir,
        });
        
        if is_dir {
            let children = build_file_tree(&path_str)?;
            item["children"] = json!(children);
        }
        
        entries.push(item);
    }
    
    Ok(entries)
}

async fn file_content_handler(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    // Decode URL-encoded path
    let decoded = urlencoding::decode(&path).map_err(|_| StatusCode::BAD_REQUEST)?;
    let full_path = std::path::Path::new(&state.workspace).join(decoded.as_ref());
    
    // Security: must be within workspace
    let canonical = full_path.canonicalize().map_err(|_| StatusCode::NOT_FOUND)?;
    let workspace_canonical = std::path::Path::new(&state.workspace).canonicalize()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    if !canonical.starts_with(&workspace_canonical) {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let content = std::fs::read_to_string(&canonical)
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    Ok((
        StatusCode::OK,
        [("Content-Type", "text/plain; charset=utf-8")],
        content,
    ))
}

// ──────────────────────────────────────────
// v2 FEATURE 2: Live Task Execution
// ──────────────────────────────────────────

#[derive(Deserialize)]
struct CreateTaskRequest {
    name: String,
    description: String,
    #[serde(default = "default_task_type")]
    task_type: String,
}

fn default_task_type() -> String { "sdlc_feature".to_string() }

async fn create_task_handler(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut queen = Queen::new(
        &state.db_path,
        &state.personas_dir,
        &state.personalities_dir,
        &state.workspace,
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let branch = format!("swarm/{}/{}", 
        req.name.to_lowercase().replace(" ", "-"),
        uuid::Uuid::new_v4().to_string()[..8].to_string()
    );
    
    let task = queen.create_task(&req.name, &req.description, &branch)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    queen.auto_assign_swarm(&task.id, &req.task_type)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(json!({
        "id": task.id,
        "name": task.name,
        "branch": task.branch,
        "status": "queued",
    })))
}

async fn start_task_handler(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut queen = Queen::new(
        &state.db_path,
        &state.personas_dir,
        &state.personalities_dir,
        &state.workspace,
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    queen.start_task(&task_id)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(json!({ "status": "started", "task_id": task_id })))
}

async fn run_task_handler(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let task_id_clone = task_id.clone();
    // This runs the execution loop — takes time, so we spawn it
    tokio::spawn(async move {
        let loop_exec = crate::execution_loop::ExecutionLoop::new(
            &state.db_path,
            &state.personas_dir,
            &state.personalities_dir,
            &state.workspace,
        );
        
        if let Ok(exec) = loop_exec {
            let _ = exec.run_until_done(&task_id_clone, 50).await;
        }
    });
    
    Ok(Json(json!({ "status": "running", "task_id": task_id })))
}

// ──────────────────────────────────────────
// Phase A: Phase, Activity, Artifact Endpoints
// ──────────────────────────────────────────

async fn phases_handler(
    State(state): State<AppState>,
    Path(story_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = Database::new(&state.db_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let phases = db.get_phases_for_story(&story_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let phases_json: Vec<_> = phases.into_iter().map(|p| json!({
        "id": p.id,
        "phase_number": p.phase_number,
        "phase_name": p.phase_name,
        "status": format!("{:?}", p.status).to_lowercase(),
        "topology": p.topology,
        "started_at": p.started_at.map(|dt| dt.to_rfc3339()),
        "completed_at": p.completed_at.map(|dt| dt.to_rfc3339()),
        "approved_by": p.approved_by,
        "approval_note": p.approval_note,
        "artifact_path": p.artifact_path,
    })).collect();
    
    Ok(Json(json!({ "phases": phases_json })))
}

async fn activity_handler(
    State(state): State<AppState>,
    Path(story_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = Database::new(&state.db_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let activity = db.get_activity_for_story(&story_id, 100).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let activity_json: Vec<_> = activity.into_iter().map(|a| json!({
        "id": a.id,
        "actor_type": a.actor_type,
        "actor_id": a.actor_id,
        "action_type": a.action_type,
        "payload": a.payload,
        "timestamp": a.timestamp.to_rfc3339(),
    })).collect();
    
    Ok(Json(json!({ "activity": activity_json })))
}

async fn artifacts_handler(
    State(state): State<AppState>,
    Path(phase_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = Database::new(&state.db_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let artifacts = db.get_artifacts_for_phase(&phase_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let artifacts_json: Vec<_> = artifacts.into_iter().map(|a| json!({
        "id": a.id,
        "artifact_type": a.artifact_type,
        "file_path": a.file_path,
        "created_at": a.created_at.map(|dt| dt.to_rfc3339()),
        "summary": a.summary,
    })).collect();
    
    Ok(Json(json!({ "artifacts": artifacts_json })))
}

// ──────────────────────────────────────────
// Phase C: Gate API Endpoints
// ──────────────────────────────────────────

async fn approve_phase_handler(
    State(state): State<AppState>,
    Path(phase_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::phases::manager::PhaseManager;

    let manager = PhaseManager::new(&state.db_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let phase = manager.approve_phase(&phase_id, "user", None)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(json!({
        "phase_id": phase.id,
        "phase_name": phase.phase_name,
        "status": format!("{:?}", phase.status).to_lowercase(),
        "approved_by": phase.approved_by,
    })))
}

async fn reject_phase_handler(
    State(state): State<AppState>,
    Path(phase_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::phases::manager::PhaseManager;

    let manager = PhaseManager::new(&state.db_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let phase = manager.reject_phase(&phase_id, "User rejected via dashboard")
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(json!({
        "phase_id": phase.id,
        "phase_name": phase.phase_name,
        "status": format!("{:?}", phase.status).to_lowercase(),
    })))
}

async fn replan_phase_handler(
    State(state): State<AppState>,
    Path(phase_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::phases::manager::PhaseManager;

    let manager = PhaseManager::new(&state.db_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let phase = manager.reject_with_replan(
        &phase_id,
        "Issues found during review — replan required",
        "Replan triggered from dashboard",
    ).map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(json!({
        "phase_id": phase.id,
        "phase_name": phase.phase_name,
        "status": format!("{:?}", phase.status).to_lowercase(),
        "replan": true,
    })))
}

// ──────────────────────────────────────────
// Phase D: Dashboard v2.0 API Endpoints
// ──────────────────────────────────────────

async fn stories_handler(
    State(state): State<AppState>,
    Path(epic_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = Database::new(&state.db_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let stories = db.get_stories_for_epic(&epic_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "stories": stories })))
}

async fn dependencies_handler(
    State(state): State<AppState>,
    Path(story_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = Database::new(&state.db_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let deps = db.get_story_dependencies(&story_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let deps_json: Vec<_> = deps.into_iter().map(|d| json!({
        "story_id": d.story_id,
        "depends_on_story_id": d.depends_on_story_id,
        "dependency_type": d.dependency_type,
    })).collect();
    
    Ok(Json(json!({ "dependencies": deps_json })))
}

async fn agents_handler(
    State(state): State<AppState>,
    Path(phase_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let db = Database::new(&state.db_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let assignments = db.get_phase_assignments(&phase_id).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let agents_json: Vec<_> = assignments.into_iter().map(|a| json!({
        "phase_id": a.phase_id,
        "persona_id": a.persona_id,
        "personality_id": a.personality_id,
        "sub_task_description": a.sub_task_description,
        "status": format!("{:?}", a.status).to_lowercase(),
        "assigned_at": a.assigned_at.map(|dt| dt.to_rfc3339()),
        "completed_at": a.completed_at.map(|dt| dt.to_rfc3339()),
        "result_summary": a.result_summary,
    })).collect();
    
    Ok(Json(json!({ "agents": agents_json })))
}

async fn start_phase_handler(
    State(state): State<AppState>,
    Path(phase_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::phases::manager::PhaseManager;

    let manager = PhaseManager::new(&state.db_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let phase = manager.start_phase(&phase_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(json!({
        "phase_id": phase.id,
        "phase_name": phase.phase_name,
        "status": format!("{:?}", phase.status).to_lowercase(),
        "started_at": phase.started_at.map(|dt| dt.to_rfc3339()),
    })))
}

async fn complete_phase_handler(
    State(state): State<AppState>,
    Path(phase_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::phases::manager::PhaseManager;

    let manager = PhaseManager::new(&state.db_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let phase = manager.complete_phase(&phase_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(json!({
        "phase_id": phase.id,
        "phase_name": phase.phase_name,
        "status": format!("{:?}", phase.status).to_lowercase(),
        "completed_at": phase.completed_at.map(|dt| dt.to_rfc3339()),
    })))
}

async fn skip_phase_handler(
    State(state): State<AppState>,
    Path(phase_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::phases::manager::PhaseManager;

    let manager = PhaseManager::new(&state.db_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let phase = manager.skip_phase(&phase_id, "Skipped from dashboard")
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(json!({
        "phase_id": phase.id,
        "phase_name": phase.phase_name,
        "status": format!("{:?}", phase.status).to_lowercase(),
    })))
}

async fn block_phase_handler(
    State(state): State<AppState>,
    Path(phase_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::phases::manager::PhaseManager;

    let manager = PhaseManager::new(&state.db_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let phase = manager.block_phase(&phase_id, "Blocked from dashboard")
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(json!({
        "phase_id": phase.id,
        "phase_name": phase.phase_name,
        "status": format!("{:?}", phase.status).to_lowercase(),
    })))
}

async fn unblock_phase_handler(
    State(state): State<AppState>,
    Path(phase_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use crate::phases::manager::PhaseManager;

    let manager = PhaseManager::new(&state.db_path)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let phase = manager.unblock_phase(&phase_id)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(json!({
        "phase_id": phase.id,
        "phase_name": phase.phase_name,
        "status": format!("{:?}", phase.status).to_lowercase(),
    })))
}

// ──────────────────────────────────────────
// Embedded HTML Dashboard — loaded from external file for syntax highlighting
// ──────────────────────────────────────────

const DASHBOARD_HTML: &str = include_str!("dashboard.html");
