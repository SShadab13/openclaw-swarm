use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, Level};
use tracing_subscriber;
use openclaw_swarm::queen::Queen;
use openclaw_swarm::coordinator::Coordinator;
use openclaw_swarm::sandbox::Sandbox;
use openclaw_swarm::models::{Letter, DiaryEntry};
use chrono::Utc;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "openclaw-swarm")]
#[command(about = "Agent swarm engine with personality-driven MxN matrix")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(long, default_value = "./openclaw-swarm.db")]
    db: String,
    
    #[arg(long, default_value = "./scripts/swarm_knowledge.db")]
    knowledge_db: String,
    
    #[arg(long, default_value = "./personas")]
    personas_dir: String,
    
    #[arg(long, default_value = "./personalities")]
    personalities_dir: String,
    
    #[arg(long, default_value = "./workspace")]
    workspace: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the swarm database and directories
    Init,
    
    /// Create a new task and auto-assign swarm
    Task {
        #[arg(short, long)]
        name: String,
        
        #[arg(short, long)]
        description: String,
        
        #[arg(short, long, default_value = "main")]
        base_branch: String,
        
        #[arg(short, long, default_value = "sdlc_feature")]
        task_type: String,
    },
    
    /// Start a task (swarm begins working)
    Start {
        #[arg(short, long)]
        task_id: String,
    },
    
    /// Send a letter between agents (or broadcast)
    Letter {
        #[arg(short, long)]
        task_id: String,
        
        #[arg(short, long)]
        from: String,
        
        #[arg(short, long)]
        to: Option<String>,
        
        #[arg(short, long)]
        content: String,
        
        #[arg(short, long, default_value = "focused")]
        mood: String,
    },
    
    /// Write a diary entry
    Diary {
        #[arg(short, long)]
        task_id: String,
        
        #[arg(short, long)]
        persona: String,
        
        #[arg(short, long)]
        personality: String,
        
        #[arg(short, long)]
        entry: String,
        
        #[arg(short, long, default_value = "reflective")]
        mood: String,
    },
    
    /// Check swarm status and mood
    Status {
        #[arg(short, long)]
        task_id: String,
    },
    
    /// List all active tasks
    List,
    
    /// Ship a task (merge to main)
    Ship {
        #[arg(short, long)]
        task_id: String,
    },
    
    /// Reassign personality to an agent mid-task (Queen's command)
    Reassign {
        #[arg(short, long)]
        task_id: String,
        
        #[arg(short, long)]
        persona: String,
        
        #[arg(short, long)]
        personality: String,
        
        #[arg(short, long)]
        mood: String,
        
        #[arg(short, long)]
        reason: String,
    },
    
    /// Run the TUI dashboard
    Dashboard,
    
    /// Run execution loop until task completion
    Run {
        #[arg(short, long)]
        task_id: String,
    },
    
    /// Dispatch a task to a specific persona (testing)
    Dispatch {
        #[arg(short, long)]
        persona: String,
        
        #[arg(short, long)]
        task: String,
    },
    
    /// Start the web dashboard (real-time monitoring at http://localhost:8080)
    Serve {
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    /// Generate markdown schema documentation for a BigQuery dataset
    BqDoc {
        /// Dataset reference: "project.dataset" or "project:dataset"
        #[arg(long)]
        dataset: String,

        /// Output markdown file path
        #[arg(long)]
        out: String,

        /// Service-account JSON key path (falls back to BQ_CREDENTIALS_PATH env var)
        #[arg(long)]
        credentials: Option<String>,
    },

    /// Snapshot a dataset's schemas to JSON (for change monitoring)
    BqSnapshot {
        /// Dataset reference: "project.dataset"
        #[arg(long)]
        dataset: String,

        /// Output JSON file path
        #[arg(long)]
        out: String,

        /// Service-account JSON key path (falls back to BQ_CREDENTIALS_PATH env var)
        #[arg(long)]
        credentials: Option<String>,
    },

    /// Lint a BigQuery dataset: cost traps + documentation gaps (metadata-only)
    BqLint {
        /// Dataset reference: "project.dataset"
        #[arg(long)]
        dataset: String,

        /// Output markdown path (stdout if omitted)
        #[arg(long)]
        out: Option<String>,

        /// Service-account JSON key path (falls back to BQ_CREDENTIALS_PATH env var)
        #[arg(long)]
        credentials: Option<String>,
    },

    /// Diff two schema snapshots into a markdown changelog
    BqDiff {
        /// Older snapshot JSON
        #[arg(long)]
        old: String,

        /// Newer snapshot JSON
        #[arg(long)]
        new: String,

        /// Output markdown path (stdout if omitted)
        #[arg(long)]
        out: Option<String>,
    },
}

/// Authenticate and fetch every table schema in a dataset.
async fn fetch_dataset_schemas(
    dataset: &str,
    credentials: Option<String>,
) -> Result<(String, String, Vec<openclaw_swarm::adapters::bq_adapter::BqTableSchema>)> {
    use openclaw_swarm::adapters::bq_adapter::{BigQueryAdapter, BqAdapterLive, BqConfig};

    let credentials_path = credentials
        .or_else(|| std::env::var("BQ_CREDENTIALS_PATH").ok())
        .unwrap_or_default();
    if credentials_path.is_empty() {
        info!("No key file given - using Application Default Credentials \
               (run `gcloud auth application-default login` once)");
    }

    let (project, dataset_id) = dataset
        .split_once(['.', ':'])
        .filter(|(p, d)| !p.is_empty() && !d.is_empty())
        .ok_or_else(|| anyhow::anyhow!(
            "Invalid dataset ref '{}': expected project.dataset", dataset))?;

    let config = BqConfig {
        project_id: project.to_string(),
        credentials_path,
        ..Default::default()
    };

    let mut adapter = BqAdapterLive::new();
    adapter.authenticate(&config).await?;

    let tables = adapter.list_tables(dataset_id).await?;
    info!("Dataset {}.{}: {} tables", project, dataset_id, tables.len());

    let mut schemas = Vec::new();
    for table in &tables {
        let table_ref = format!("{}.{}.{}", project, dataset_id, table);
        info!("Fetching schema: {}", table_ref);
        schemas.push(adapter.get_schema(&table_ref).await?);
    }
    Ok((project.to_string(), dataset_id.to_string(), schemas))
}

fn write_with_parents(path: &str, content: &str) -> Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init => {
            info!("Initializing OpenClaw Swarm...");
            std::fs::create_dir_all(&cli.personas_dir)?;
            std::fs::create_dir_all(&cli.personalities_dir)?;
            std::fs::create_dir_all(&cli.workspace)?;
            info!("Directories created: personas, personalities, workspace");
            info!("Database path: {}", cli.db);
            info!("Ready. Add YAML files to personas/ and personalities/");
        }
        
        Commands::Task { name, description, base_branch: _, task_type } => {
            let mut queen = Queen::new(&cli.db, &cli.personas_dir, &cli.personalities_dir, &cli.workspace)?;
            
            let branch = format!("swarm/{}/{}", 
                name.to_lowercase().replace(" ", "-"),
                Uuid::new_v4().to_string()[..8].to_string()
            );
            
            let task = queen.create_task(&name, &description, &branch)?;
            queen.auto_assign_swarm(&task.id, &task_type)?;
            
            info!("Task {} created with {} swarm", task.id, task_type);
            info!("Branch: {}", branch);
            info!("Run: openclaw-swarm start --task-id {}", task.id);
        }
        
        Commands::Start { task_id } => {
            let mut queen = Queen::new(&cli.db, &cli.personas_dir, &cli.personalities_dir, &cli.workspace)?;
            queen.start_task(&task_id)?;
            info!("Task {} started. Swarm is active.", task_id);
        }
        
        Commands::Letter { task_id, from, to, content, mood } => {
            let queen = Queen::new(&cli.db, &cli.personas_dir, &cli.personalities_dir, &cli.workspace)?;
            
            let letter = Letter {
                id: Uuid::new_v4(),
                task_id,
                from_persona: from,
                to_persona: to,
                content,
                mood_at_send: mood,
                sent_at: Utc::now(),
            };
            
            queen.route_letter(&letter)?;
            info!("Letter sent from {}.", letter.from_persona);
        }
        
        Commands::Diary { task_id, persona, personality, entry, mood } => {
            let _queen = Queen::new(&cli.db, &cli.personas_dir, &cli.personalities_dir, &cli.workspace)?;
            
            let diary = DiaryEntry {
                id: Uuid::new_v4(),
                task_id,
                persona_id: persona,
                personality_id: personality,
                entry,
                mood,
                written_at: Utc::now(),
            };
            
            info!("Diary entry recorded for {} ({}) at {}", 
                  diary.persona_id, diary.personality_id, diary.written_at);
        }
        
        Commands::Status { task_id } => {
            let coordinator = Coordinator::new(&cli.db)?;
            let step = coordinator.execute_step(&task_id)?;
            let mood = coordinator.swarm_mood_report(&task_id)?;
            
            info!("{}", mood);
            info!("Step status: {:?}", step.status);
            info!("Message: {}", step.message);
            if let Some(action) = step.action_required {
                info!("Action required: {}", action);
            }
        }
        
        Commands::List => {
            let queen = Queen::new(&cli.db, &cli.personas_dir, &cli.personalities_dir, &cli.workspace)?;
            let tasks = queen.list_active_tasks();
            
            if tasks.is_empty() {
                info!("No active tasks.");
            } else {
                info!("Active tasks: {}", tasks.len());
                for task in tasks {
                    info!("  - {} ({} agents)", 
                          task.task.name, 
                          task.assignments.len());
                }
            }
        }
        
        Commands::Ship { task_id } => {
            let queen = Queen::new(&cli.db, &cli.personas_dir, &cli.personalities_dir, &cli.workspace)?;
            let coordinator = Coordinator::new(&cli.db)?;
            
            let step = coordinator.execute_step(&task_id)?;
            
            match step.status {
                openclaw_swarm::coordinator::StepStatus::ReadyToMerge => {
                    let sandbox = Sandbox::new("main", &cli.workspace);
                    let tasks = queen.list_active_tasks();
                    if let Some(swarm) = tasks.iter().find(|t| t.task.id == task_id) {
                        let room = openclaw_swarm::sandbox::SandboxRoom {
                            task_id: swarm.task.id.clone(),
                            branch: swarm.task.branch.clone(),
                            path: cli.workspace.clone(),
                            status: openclaw_swarm::sandbox::SandboxStatus::ReadyToMerge,
                        };
                        sandbox.ship(&room)?;
                        sandbox.close_room(&room)?;
                        info!("Task {} SHIPPED to main!", task_id);
                    }
                }
                _ => {
                    info!("Cannot ship. Status: {:?} - {}", step.status, step.message);
                }
            }
        }
        
        Commands::Reassign { task_id, persona, personality, mood, reason } => {
            let mut queen = Queen::new(&cli.db, &cli.personas_dir, &cli.personalities_dir, &cli.workspace)?;
            queen.reassign_personality(&task_id, &persona, &personality, &mood, &reason)?;
            info!("Reassignment complete. Queen has spoken.");
        }
        
        Commands::Dashboard => {
            info!("Starting TUI dashboard...");
            openclaw_swarm::dashboard::run_dashboard(&cli.db, &cli.knowledge_db)?;
        }
        
        Commands::Run {
            task_id,
        } => {
            info!("Running execution loop for task {}...", task_id);
            let loop_exec = openclaw_swarm::execution_loop::ExecutionLoop::new(
                &cli.db, &cli.personas_dir, &cli.personalities_dir, &cli.workspace)?;
            loop_exec.run_until_done(&task_id, 50).await?;
            info!("Execution loop completed for task {}", task_id);
        }
        
        Commands::Dispatch { persona, task } => {
            let queen = Queen::new(&cli.db, &cli.personas_dir, &cli.personalities_dir, &cli.workspace)?;
            let result = queen.dispatch_task(&persona, &task).await?;
            info!("Dispatch result:\n{}", result);
        }
        
        Commands::Serve { port } => {
            info!("Starting web dashboard on http://localhost:{} ...", port);
            let bus = std::sync::Arc::new(tokio::sync::Mutex::new(
                openclaw_swarm::swarm_bus::SwarmBus::new()
            ));
            let dashboard = openclaw_swarm::web_dashboard::WebDashboard::new(
                &cli.db,
                &cli.workspace,
                &cli.personas_dir,
                &cli.personalities_dir,
                bus
            );
            dashboard.run(port).await?;
        }

        Commands::BqDoc { dataset, out, credentials } => {
            let (project, dataset_id, schemas) =
                fetch_dataset_schemas(&dataset, credentials).await?;
            let doc = openclaw_swarm::bq_doc::render_dataset_doc(&project, &dataset_id, &schemas);
            write_with_parents(&out, &doc)?;
            info!("Schema doc written: {} ({} tables, {} bytes)", out, schemas.len(), doc.len());
        }

        Commands::BqSnapshot { dataset, out, credentials } => {
            let (_, _, schemas) = fetch_dataset_schemas(&dataset, credentials).await?;
            let json = serde_json::to_string_pretty(&schemas)?;
            write_with_parents(&out, &json)?;
            info!("Snapshot written: {} ({} tables)", out, schemas.len());
        }

        Commands::BqLint { dataset, out, credentials } => {
            let (project, dataset_id, schemas) =
                fetch_dataset_schemas(&dataset, credentials).await?;
            let findings = openclaw_swarm::bq_lint::lint_schemas(&schemas);
            let report = openclaw_swarm::bq_lint::render_lint_report(
                &project, &dataset_id, schemas.len(), &findings);
            match out {
                Some(path) => {
                    write_with_parents(&path, &report)?;
                    info!("Lint report written: {} ({} findings)", path, findings.len());
                }
                None => println!("{}", report),
            }
        }

        Commands::BqDiff { old, new, out } => {
            use openclaw_swarm::adapters::bq_adapter::BqTableSchema;
            let old_schemas: Vec<BqTableSchema> =
                serde_json::from_str(&std::fs::read_to_string(&old)?)?;
            let new_schemas: Vec<BqTableSchema> =
                serde_json::from_str(&std::fs::read_to_string(&new)?)?;
            let diff = openclaw_swarm::bq_doc::render_schema_diff(&old_schemas, &new_schemas);
            match out {
                Some(path) => {
                    write_with_parents(&path, &diff)?;
                    info!("Changelog written: {}", path);
                }
                None => println!("{}", diff),
            }
        }
    }

    Ok(())
}
