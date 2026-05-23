use anyhow::Result;
use tracing::info;
use crate::models::*;
use crate::runners::RunnerRegistry;
use crate::db::Database;
use crate::persona_loader::Loader;
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;

/// The Queen is the sovereign agent that assigns personalities to personas,
/// creates tasks, and orchestrates the swarm.
#[allow(dead_code)]
pub struct Queen {
    db: Database,
    personas: Vec<Persona>,
    personalities: Vec<Personality>,
    active_tasks: HashMap<String, SwarmState>,
    runners: RunnerRegistry,
}

impl Queen {
    pub fn new(db_path: &str, personas_dir: &str, personalities_dir: &str, workspace: &str) -> Result<Self> {
        let db = Database::new(db_path)?;
        let personas = Loader::load_personas(personas_dir)?;
        let personalities = Loader::load_personalities(personalities_dir)?;
        let runners = RunnerRegistry::new(workspace);
        
        info!("Queen initialized with {} personas, {} personalities, runners: [kimi, claude, openclaw]", 
              personas.len(), personalities.len());
        
        Ok(Self {
            db,
            personas,
            personalities,
            active_tasks: HashMap::new(),
            runners,
        })
    }
    
    /// Create a new task and assign the appropriate swarm.
    pub fn create_task(&mut self, name: &str, description: &str, branch: &str) -> Result<Task> {
        let task = Task {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            branch: branch.to_string(),
            status: TaskStatus::Queued,
            created_at: Utc::now(),
            completed_at: None,
        };
        
        self.db.create_task(&task)?;
        info!("Task created: {} ({}) on branch {}", task.name, task.id, task.branch);
        
        Ok(task)
    }
    
    /// Assign a swarm to a task. The Queen decides which persona gets which personality.
    pub fn assign_swarm(&mut self, task_id: &str, assignments: Vec<(String, String, String, String)>) -> Result<()> {
        // assignments: Vec<(persona_id, personality_id, mood, reason)>
        for (persona_id, personality_id, mood, reason) in assignments {
            let assignment = Assignment {
                task_id: task_id.to_string(),
                persona_id,
                personality_id,
                mood,
                reason,
                assigned_at: Utc::now(),
            };
            
            self.db.assign_agent(&assignment)?;
            info!("Assigned {} with {} mood '{}' to task {}", 
                  assignment.persona_id, assignment.personality_id, assignment.mood, task_id);
        }
        
        Ok(())
    }
    
    /// Auto-assign a default swarm based on task type.
    pub fn auto_assign_swarm(&mut self, task_id: &str, task_type: &str) -> Result<()> {
        let assignments = match task_type {
            "sdlc_feature" => vec![
                ("architect".to_string(), "meticulous".to_string(), "calm".to_string(), "Schema needs precision".to_string()),
                ("coder".to_string(), "tsundere".to_string(), "frustrated".to_string(), "Deadline tight, needs pressure".to_string()),
                ("tester".to_string(), "sadist_cheerful".to_string(), "excited".to_string(), "Loves breaking tsundere's code".to_string()),
                ("frontend".to_string(), "honest".to_string(), "confused".to_string(), "Needs clarity on API contract".to_string()),
                ("devops".to_string(), "meticulous".to_string(), "paranoid".to_string(), "Production deployment".to_string()),
            ],
            "bug_fix" => vec![
                ("coder".to_string(), "honest".to_string(), "focused".to_string(), "Root cause analysis".to_string()),
                ("tester".to_string(), "sadist_cheerful".to_string(), "delighted".to_string(), "Regression test needed".to_string()),
                ("architect".to_string(), "meticulous".to_string(), "calm".to_string(), "Check for systemic issues".to_string()),
            ],
            "architecture" => vec![
                ("architect".to_string(), "meticulous".to_string(), "contemplative".to_string(), "Design phase".to_string()),
                ("mlops".to_string(), "honest".to_string(), "curious".to_string(), "Model serving requirements".to_string()),
                ("devops".to_string(), "confused".to_string(), "questioning".to_string(), "Infra implications".to_string()),
            ],
            _ => vec![
                ("coder".to_string(), "honest".to_string(), "focused".to_string(), "Default assignment".to_string()),
                ("tester".to_string(), "sadist_cheerful".to_string(), "excited".to_string(), "Default testing".to_string()),
            ],
        };
        
        self.assign_swarm(task_id, assignments)
    }
    
    /// Start a task: move from Queued to Running.
    pub fn start_task(&mut self, task_id: &str) -> Result<()> {
        let assignments = self.db.get_task_assignments(task_id)?;
        let letters = self.db.get_task_letters(task_id)?;
        
        let swarm_state = SwarmState {
            task: Task {
                id: task_id.to_string(),
                name: "Unknown".to_string(),
                description: "".to_string(),
                branch: "".to_string(),
                status: TaskStatus::Running,
                created_at: Utc::now(),
                completed_at: None,
            },
            assignments,
            letters,
            diary: Vec::new(),
            errors: Vec::new(),
        };
        
        self.active_tasks.insert(task_id.to_string(), swarm_state);
        info!("Task {} started. Swarm active.", task_id);
        
        Ok(())
    }
    
    /// The Queen reads a letter from one agent and routes it.
    pub fn route_letter(&self, letter: &Letter) -> Result<()> {
        info!("Letter from {} in task {}", letter.from_persona, letter.task_id);
        
        if let Some(to) = &letter.to_persona {
            info!("  → Routed to {}", to);
        } else {
            info!("  → Broadcast to all agents in task");
        }
        
        Ok(())
    }
    
    /// Dispatch a task to the appropriate runner for a persona.
    pub async fn dispatch_task(&self,
        persona_id: &str,
        task: &str,
    ) -> Result<String> {
        let persona = Loader::get_persona_by_id(&self.personas, persona_id)
            .ok_or_else(|| anyhow::anyhow!("Persona {} not found", persona_id))?;
        
        let runner = self.runners.runner_for_persona(&persona)?;
        info!("[Queen] Dispatching to {} via {}: {}", persona_id, runner.name(), task);
        
        let result = runner.execute(task).await?;
        Ok(result)
    }
    
    /// Dispatch with a specific runner override (Queen's prerogative).
    pub async fn dispatch_with_runner(&self,
        runner_name: &str,
        task: &str,
    ) -> Result<String> {
        let runner = self.runners.get(runner_name)
            .ok_or_else(|| anyhow::anyhow!("Runner {} not found", runner_name))?;
        
        info!("[Queen] Direct dispatch via {}: {}", runner_name, task);
        let result = runner.execute(task).await?;
        Ok(result)
    }
    pub fn list_active_tasks(&self) -> Vec<&SwarmState> {
        self.active_tasks.values().collect()
    }
    
    /// Reassign personality mid-task (Queen's prerogative).
    pub fn reassign_personality(&mut self, task_id: &str, persona_id: &str, new_personality_id: &str, new_mood: &str, reason: &str) -> Result<()> {
        let assignment = Assignment {
            task_id: task_id.to_string(),
            persona_id: persona_id.to_string(),
            personality_id: new_personality_id.to_string(),
            mood: new_mood.to_string(),
            reason: reason.to_string(),
            assigned_at: Utc::now(),
        };
        
        self.db.assign_agent(&assignment)?;
        info!("Reassigned {} in task {}: now {} with mood '{}' - {}", 
              persona_id, task_id, new_personality_id, new_mood, reason);
        
        Ok(())
    }
}
