use anyhow::Result;
use tracing::{info, debug, warn};
use crate::models::*;
use crate::db::Database;
use crate::queen::Queen;
use crate::coordinator::{Coordinator, StepStatus};
use crate::runners::RunnerRegistry;
use crate::sandbox::{Sandbox, SandboxRoom};
use crate::swarm_bus::{SwarmBus, LetterStream};
use crate::task_fsm::{TaskFsm, TaskState};
use crate::error_journal::ErrorJournal;
use crate::knowledge::CompositeKnowledgeSource;
use std::time::Duration;
use std::sync::Arc;
use tokio::task;
use tokio::time::sleep;

// =============================================================================
// Free functions — agent step execution (called from tokio tasks)
// =============================================================================

/// Execute one step for a single agent assignment — runs in its own tokio task.
///
/// Inter-agent communication: the agent subscribes to the task's broadcast
/// channel and reads peer letters before executing.
async fn execute_agent_step(
    db: Arc<Database>,
    runners: RunnerRegistry,
    workspace: String,
    task_id: String,
    task_description: String,
    assignment: Assignment,
    all_letters: Vec<Letter>,
    room: SandboxRoom,
    bus: Arc<tokio::sync::Mutex<SwarmBus>>,
    knowledge_graph_path: String,
) -> Result<()> {
    let persona_id = assignment.persona_id.clone();
    let personality_id = assignment.personality_id.clone();

    info!("[Agent] {} ({}) starting work on task {}",
          persona_id, personality_id, task_id);

    // Get the persona definition
    let personas = crate::persona_loader::Loader::load_personas("./personas")?;
    let persona = crate::persona_loader::Loader::get_persona_by_id(
        &personas, &persona_id)
        .ok_or_else(|| anyhow::anyhow!("Persona {} not found", persona_id))?;

    // Get the runner for this persona
    let runner = runners.runner_for_persona(&persona)?;

    // --- INTER-AGENT COMMUNICATION via pub/sub ---
    // Subscribe to task channel and drain any pending letters
    let mut _stream = {
        let bus_guard = bus.lock().await;
        if let Some(receiver) = bus_guard.subscribe(&task_id) {
            let mut s = LetterStream::new(receiver);
            s.drain_pending();
            s
        } else {
            // Fallback: read from DB if no bus channel
            let s = LetterStream::new(tokio::sync::broadcast::channel(1).1);
            for letter in &all_letters {
                if letter.from_persona != persona_id {
                    // Can't push to stream buffer directly, but we can use them below
                }
            }
            s
        }
    };

    // Build peer context from DB letters (fallback when bus is not available)
    let peer_letters: Vec<&Letter> = all_letters.iter()
        .filter(|l| l.from_persona != persona_id)
        .collect();

    // Query knowledge graph for relevant context
    let knowledge = match CompositeKnowledgeSource::from_path(&knowledge_graph_path) {
        Ok(k) => k,
        Err(e) => {
            warn!("[Agent] {} failed to load knowledge graph: {}", persona_id, e);
            CompositeKnowledgeSource::new(crate::knowledge::ObsidianVault::from_defaults(), crate::knowledge::SqliteGraph::new(":memory:")?)
        }
    };

    // Build task-specific prompt enriched with peer context and knowledge
    let prompt = build_prompt_with_context(
        &task_id, &persona_id, &assignment.reason,
        &task_description,
        &peer_letters, &assignment.mood,
        &knowledge
    );

    // Execute via runner
    debug!("[Agent] {} dispatching to {}: {} chars", persona_id, runner.name(), prompt.len());

    let result = if runner.name() == "openclaw" {
        run_openclaw_task(&runners, &workspace, &task_id, &persona_id, &prompt).await
    } else {
        runner.execute(&prompt).await
    };

    // Write letter based on result
    match result {
        Ok(output) => {
            let truncated = truncate_chars(&output, 500);
            let letter_content = if truncated.len() < output.len() {
                format!("DONE: {}...", truncated)
            } else {
                format!("DONE: {}", output)
            };

            let letter = Letter {
                id: uuid::Uuid::new_v4(),
                task_id: task_id.clone(),
                from_persona: persona_id.clone(),
                to_persona: Some("all".to_string()),
                content: letter_content,
                mood_at_send: assignment.mood.clone(),
                sent_at: chrono::Utc::now(),
            };

            // Publish to bus + write to DB
            {
                let bus_guard = bus.lock().await;
                let _ = bus_guard.publish(&task_id, letter.clone());
            }
            db.write_letter(&letter)?;

            // Commit agent work to sandbox
            let sandbox = Sandbox::new("main", &workspace);
            if let Err(e) = sandbox.commit_work(&room, &persona_id, "Step completed") {
                warn!("[Agent] {} sandbox commit failed: {}", persona_id, e);
            }

            info!("[Agent] {} completed step. Output: {} chars", persona_id, output.len());
        }
        Err(e) => {
            warn!("[Agent] {} failed: {}", persona_id, e);

            // Log to error journal
            let error_journal = ErrorJournal::new(&format!("{}/error_journal.db", workspace))?;
            let error_log = ErrorLog {
                id: uuid::Uuid::new_v4(),
                task_id: task_id.clone(),
                persona_id: persona_id.clone(),
                error_message: e.to_string(),
                error_type: classify_error(&e.to_string()),
                file_path: None,
                line_number: None,
                root_cause: None,
                solution: None,
                same_symptom_different_cause: false,
                occurred_at: chrono::Utc::now(),
            };
            let _ = error_journal.log_error(&error_log);

            // Write failure letter to Queen
            let letter = Letter {
                id: uuid::Uuid::new_v4(),
                task_id: task_id.clone(),
                from_persona: persona_id.clone(),
                to_persona: Some("queen".to_string()),
                content: format!("BLOCKING: {} failed: {}", persona_id, e),
                mood_at_send: "frustrated".to_string(),
                sent_at: chrono::Utc::now(),
            };

            {
                let bus_guard = bus.lock().await;
                let _ = bus_guard.publish(&task_id, letter.clone());
            }
            db.write_letter(&letter)?;
        }
    }

    Ok(())
}

/// Auto-classify an error message into a type.
fn classify_error(error: &str) -> String {
    let lower = error.to_lowercase();
    if lower.contains("not found") || lower.contains("does not exist") {
        "NotFound".to_string()
    } else if lower.contains("permission") || lower.contains("access") {
        "PermissionDenied".to_string()
    } else if lower.contains("type mismatch") || lower.contains("expected") {
        "TypeMismatch".to_string()
    } else if lower.contains("timeout") || lower.contains("timed out") {
        "Timeout".to_string()
    } else if lower.contains("connection") || lower.contains("network") {
        "NetworkError".to_string()
    } else if lower.contains("parse") || lower.contains("invalid") {
        "ParseError".to_string()
    } else {
        "Unknown".to_string()
    }
}

/// Build a task-specific prompt for a persona, enriched with peer letters.
fn build_prompt_with_context(
    task_id: &str,
    persona_id: &str,
    reason: &str,
    task_description: &str,
    peer_letters: &[&Letter],
    mood: &str,
    knowledge: &CompositeKnowledgeSource,
) -> String {
    let mut peer_context = String::new();
    if !peer_letters.is_empty() {
        peer_context.push_str("📬 Letters from other agents in this task:\n");
        for letter in peer_letters.iter().take(5) {
            let to = letter.to_persona.as_deref().unwrap_or("all");
            let content_preview = if letter.content.len() > 200 {
                format!("{}...", &letter.content[..200])
            } else {
                letter.content.clone()
            };
            peer_context.push_str(&format!(
                "  - {} → {} ({}): {}\n",
                letter.from_persona, to, letter.mood_at_send, content_preview
            ));
        }
        peer_context.push('\n');
    } else {
        peer_context.push_str("📬 No letters from other agents yet. You are first.\n\n");
    }

    // Query knowledge graph for relevant context
    let knowledge_context = match knowledge.read(task_description) {
        Ok(chunks) if !chunks.is_empty() => {
            let mut ctx = String::from("🧠 Relevant knowledge from the vault:\n");
            for chunk in chunks.iter().take(5) {
                let preview = if chunk.content.len() > 300 {
                    format!("{}...", &chunk.content[..300])
                } else {
                    chunk.content.clone()
                };
                ctx.push_str(&format!(
                    "  - {} (relevance {:.2}): {}\n",
                    chunk.source, chunk.relevance, preview
                ));
            }
            ctx.push('\n');
            ctx
        }
        Ok(_) => "🧠 No relevant vault knowledge found.\n\n".to_string(),
        Err(e) => {
            warn!("Failed to query knowledge graph: {}", e);
            "🧠 Vault knowledge unavailable.\n\n".to_string()
        }
    };

    format!(
        "You are the '{}' persona in the OpenClaw Swarm.\n\
        Task ID: {}\n\
        Your role: {}\n\
        Your current mood: {}\n\
        \n\
        TASK DESCRIPTION: {}\n\
        \n\
        {}\
        {}\n        Your assignment: {}\n\
        \n\
        Execute this task using your skills. Write code to files when appropriate.\n\
        Report DONE when complete.\n\
        If blocked by something another agent is responsible for, report BLOCKING with reason.\n\
        If you need to ask another agent a question, mention their persona name.\n\
        Keep your response concise (max 1000 chars) to save tokens.",
        persona_id, task_id, persona_id, mood, task_description, peer_context, knowledge_context, reason
    )
}

/// Run an OpenClaw-specific task using file ops, exec, etc.
async fn run_openclaw_task(
    runners: &RunnerRegistry,
    _workspace: &str,
    _task_id: &str,
    _persona_id: &str,
    prompt: &str,
) -> Result<String> {
    let openclaw_runner = runners.get("openclaw")
        .ok_or_else(|| anyhow::anyhow!("OpenClaw runner not found"))?;
    openclaw_runner.execute(prompt).await
}

// =============================================================================
// ExecutionLoop struct + impl (async)
// =============================================================================

/// The Execution Loop makes swarm agents DO work — in PARALLEL via tokio.
///
/// Flow:
/// 1. Read task assignments from DB
/// 2. Create SwarmBus channel for the task
/// 3. Spawn each agent as a concurrent tokio task
/// 4. Collect results, write letters, check coordinator status
/// 5. Repeat until task is complete (ReadyToMerge, Blocked, or Failed)
pub struct ExecutionLoop {
    db: Arc<Database>,
    _queen: Queen,
    coordinator: Coordinator,
    #[allow(dead_code)]
    runners: RunnerRegistry,
    workspace: String,
    sandbox: Sandbox,
    bus: Arc<tokio::sync::Mutex<SwarmBus>>,
    fsm: TaskFsm,
    knowledge_graph_path: String,
}

impl ExecutionLoop {
    pub fn new(
        db_path: &str,
        personas_dir: &str,
        personalities_dir: &str,
        workspace: &str,
    ) -> Result<Self> {
        let db = Arc::new(Database::new(db_path)?);
        let queen = Queen::new(db_path, personas_dir, personalities_dir, workspace)?;
        let coordinator = Coordinator::new(db_path)?;
        let runners = RunnerRegistry::new(workspace);
        let sandbox = Sandbox::new("main", workspace);
        let bus = Arc::new(tokio::sync::Mutex::new(SwarmBus::new()));
        let fsm = TaskFsm::new(db_path)?;
        let knowledge_graph_path = std::env::var("OPENCLAW_KNOWLEDGE_GRAPH")
            .unwrap_or_else(|_| "./knowledge_graph.db".to_string());

        Ok(Self {
            db,
            _queen: queen,
            coordinator,
            runners,
            workspace: workspace.to_string(),
            sandbox,
            bus,
            fsm,
            knowledge_graph_path,
        })
    }

    /// Run a single step of execution for a task — ALL AGENTS IN PARALLEL.
    /// Returns true if task is complete (ReadyToMerge or Failed).
    pub async fn run_step(
        &self,
        task_id: &str,
        task_description: &str,
        room: &SandboxRoom,
    ) -> Result<bool> {
        let assignments = self.db.get_task_assignments(task_id)?;

        if assignments.is_empty() {
            warn!("Task {} has no assignments - cannot execute", task_id);
            return Ok(true);
        }

        // Ensure bus channel exists for this task
        {
            let mut bus_guard = self.bus.lock().await;
            if !bus_guard.has_channel(task_id) {
                bus_guard.create_task_channel(task_id, 100);
            }
        }

        // Pre-load ALL letters for fallback context
        let all_letters = self.db.get_task_letters(task_id)?;
        info!(
            "[ExecutionLoop] Task {}: {} agents, {} letters. Spawning swarm...",
            task_id, assignments.len(), all_letters.len()
        );

        // Spawn each agent as a concurrent tokio task
        let mut handles = vec![];
        for assignment in assignments {
            let db = Arc::clone(&self.db);
            let runners = RunnerRegistry::new(&self.workspace);
            let workspace = self.workspace.clone();
            let task_id = task_id.to_string();
            let task_description = task_description.to_string();
            let letters = all_letters.clone();
            let room = SandboxRoom {
                task_id: room.task_id.clone(),
                branch: room.branch.clone(),
                path: room.path.clone(),
                status: room.status.clone(),
            };
            let bus = Arc::clone(&self.bus);
            let knowledge_graph_path = self.knowledge_graph_path.clone();

            let handle = task::spawn(async move {
                execute_agent_step(db, runners, workspace, task_id, task_description, assignment, letters, room, bus, knowledge_graph_path).await
            });
            handles.push(handle);
        }

        // Wait for all agents to complete
        for handle in handles {
            let _ = handle.await;
        }

        info!("[ExecutionLoop] Task {}: all agents completed step", task_id);

        // Check coordinator status
        let step_result = self.coordinator.execute_step(task_id)?;

        match step_result.status {
            StepStatus::ReadyToMerge => {
                info!("[ExecutionLoop] Task {} ready to ship!", task_id);
                self.fsm.transition(
                    task_id,
                    TaskState::Review,
                    TaskState::ReadyToMerge,
                    "All agents report completion",
                    "coordinator",
                )?;
                Ok(true)
            }
            StepStatus::Blocked => {
                warn!(
                    "[ExecutionLoop] Task {} BLOCKED: {}",
                    task_id,
                    step_result.action_required.unwrap_or_default()
                );
                self.fsm.transition(
                    task_id,
                    TaskState::Running,
                    TaskState::Blocked,
                    &step_result.message,
                    "coordinator",
                )?;
                Ok(true)
            }
            StepStatus::Failed => {
                warn!("[ExecutionLoop] Task {} FAILED", task_id);
                self.fsm.transition(
                    task_id,
                    TaskState::Running,
                    TaskState::Failed,
                    &step_result.message,
                    "coordinator",
                )?;
                Ok(true)
            }
            StepStatus::InProgress => {
                info!(
                    "[ExecutionLoop] Task {} still in progress: {}",
                    task_id, step_result.message
                );
                Ok(false)
            }
        }
    }

    /// Run the full execution loop until completion.
    pub async fn run_until_done(
        &self,
        task_id: &str,
        max_steps: usize,
    ) -> Result<()> {
        // Transition task to Running
        self.fsm.transition(
            task_id,
            TaskState::Queued,
            TaskState::Running,
            "Starting execution loop",
            "execution_loop",
        )?;

        // Create sandbox room for this task
        let task = self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, branch, status, created_at, completed_at FROM tasks WHERE id = ?1"
            )?;
            let task_result = stmt.query_row([task_id], |row| {
                Ok(Task {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    branch: row.get(3)?,
                    status: TaskStatus::Running,
                    created_at: row.get::<_, String>(5)?.parse().unwrap_or_else(|_| chrono::Utc::now()),
                    completed_at: row.get::<_, Option<String>>(6)?.map(|s| s.parse().unwrap_or_else(|_| chrono::Utc::now())),
                })
            });
            match task_result {
                Ok(t) => Ok(Some(t)),
                Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.into()),
            }
        })?;

        let room = if let Some(ref t) = task {
            self.sandbox.create_room(&t)?
        } else {
            anyhow::bail!("Task {} not found in database", task_id);
        };

        for step in 1..=max_steps {
            info!(
                "[ExecutionLoop] === Step {}/{} for task {} ===",
                step, max_steps, task_id
            );

            let task_description = task.as_ref().map(|t| t.description.clone()).unwrap_or_default();
            let done = self.run_step(task_id, &task_description, &room).await?;
            if done {
                info!(
                    "[ExecutionLoop] Task {} completed after {} steps",
                    task_id, step
                );
                break;
            }

            // Sleep between steps to avoid hammering APIs
            sleep(Duration::from_secs(2)).await;
        }

        Ok(())
    }
}

/// Truncate to at most `max_chars` characters without splitting a UTF-8 char.
/// Plain byte slicing (`&s[..500]`) panics on multibyte boundaries.
fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::truncate_chars;

    #[test]
    fn test_truncate_chars_multibyte_no_panic() {
        let s = "✨".repeat(600); // 600 chars, 1800 bytes — byte slice at 500 would panic
        let t = truncate_chars(&s, 500);
        assert_eq!(t.chars().count(), 500);
    }

    #[test]
    fn test_truncate_chars_short_string_unchanged() {
        assert_eq!(truncate_chars("abc", 500), "abc");
        assert_eq!(truncate_chars("", 500), "");
    }
}
