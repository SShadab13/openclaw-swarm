use anyhow::Result;
use chrono::Utc;
use tracing::info;
use uuid::Uuid;

use crate::db::Database;
use crate::models::{Task, TaskStatus, StoryDependency};
use crate::phases::manager::PhaseManager;

/// PlanningAgent analyzes user requests, scopes work, and produces
/// structured plans (epic/story breakdown + phase assignments).
///
/// This implements FR-3 from the phased orchestration design:
/// "Dedicated persona for requirement analysis and scoping."
pub struct PlanningAgent {
    db: Database,
    phase_manager: PhaseManager,
}

/// Sizing decision: is this an Epic or a Story?
#[derive(Debug, Clone, PartialEq)]
pub enum WorkSize {
    Epic,
    Story,
    Task,
}

/// A scoped story with its dependencies and metadata.
#[derive(Debug, Clone)]
pub struct StoryPlan {
    pub name: String,
    pub description: String,
    pub estimated_minutes: u32,
    pub files_touched_estimate: u32,
    pub tables_modified_estimate: u32,
    pub modules_touched_estimate: u32,
    pub depends_on: Vec<String>, // story names this depends on
}

/// A complete plan for an epic or story.
#[derive(Debug, Clone)]
pub struct Plan {
    pub size: WorkSize,
    pub epic_name: String,
    pub epic_description: String,
    pub stories: Vec<StoryPlan>,
    pub naming_conventions: NamingConventions,
}

/// Naming conventions to enforce across all agents in a story.
#[derive(Debug, Clone, Default)]
pub struct NamingConventions {
    pub functions: String,
    pub tables: String,
    pub components: String,
    pub variables: String,
}

impl PlanningAgent {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = Database::new(db_path)?;
        let phase_manager = PhaseManager::new(db_path)?;
        Ok(Self { db, phase_manager })
    }

    // =========================================================================
    // Sizing Analysis
    // =========================================================================

    /// Analyze a user request and determine if it's an Epic, Story, or Task.
    ///
    /// Decision matrix:
    /// - Files touched > 5 → Epic
    /// - DB tables modified > 2 → Epic
    /// - Cross-module (2+ modules) → Epic
    /// - New feature area → Epic
    /// - Estimated agent-hours > 30 min → Epic
    /// - Otherwise → Story (or Task if trivial)
    pub fn analyze_size(&self,
        name: &str,
        description: &str,
        files_estimate: Option<u32>,
        tables_estimate: Option<u32>,
        modules_estimate: Option<u32>,
        estimated_minutes: Option<u32>,
    ) -> Result<WorkSize> {

        let files = files_estimate.unwrap_or_else(|| self.estimate_files(description));
        let tables = tables_estimate.unwrap_or_else(|| self.estimate_tables(description));
        let modules = modules_estimate.unwrap_or_else(|| self.estimate_modules(description));
        let minutes = estimated_minutes.unwrap_or_else(|| self.estimate_duration(description));

        info!("[PlanningAgent] Sizing '{}' — files={}, tables={}, modules={}, minutes={}",
            name, files, tables, modules, minutes);

        // Decision matrix
        if files > 5 || tables > 2 || modules >= 2 || minutes > 30 {
            return Ok(WorkSize::Epic);
        }

        if files > 2 || tables > 0 || minutes > 10 {
            return Ok(WorkSize::Story);
        }

        Ok(WorkSize::Task)
    }

    /// Create a full plan (epic breakdown or single story).
    pub fn create_plan(
        &self,
        name: &str,
        description: &str,
        size: WorkSize,
    ) -> Result<Plan> {

        match size {
            WorkSize::Epic => {
                let stories = self.breakdown_epic(name, description)?;
                let naming = self.infer_naming_conventions(description);

                Ok(Plan {
                    size: WorkSize::Epic,
                    epic_name: name.to_string(),
                    epic_description: description.to_string(),
                    stories,
                    naming_conventions: naming,
                })
            }
            WorkSize::Story => {
                let story = StoryPlan {
                    name: name.to_string(),
                    description: description.to_string(),
                    estimated_minutes: self.estimate_duration(description),
                    files_touched_estimate: self.estimate_files(description),
                    tables_modified_estimate: self.estimate_tables(description),
                    modules_touched_estimate: self.estimate_modules(description),
                    depends_on: vec![],
                };
                let naming = self.infer_naming_conventions(description);

                Ok(Plan {
                    size: WorkSize::Story,
                    epic_name: name.to_string(),
                    epic_description: description.to_string(),
                    stories: vec![story],
                    naming_conventions: naming,
                })
            }
            WorkSize::Task => {
                // Single task — no breakdown needed
                let story = StoryPlan {
                    name: name.to_string(),
                    description: description.to_string(),
                    estimated_minutes: self.estimate_duration(description),
                    files_touched_estimate: self.estimate_files(description),
                    tables_modified_estimate: self.estimate_tables(description),
                    modules_touched_estimate: self.estimate_modules(description),
                    depends_on: vec![],
                };

                Ok(Plan {
                    size: WorkSize::Task,
                    epic_name: name.to_string(),
                    epic_description: description.to_string(),
                    stories: vec![story],
                    naming_conventions: NamingConventions::default(),
                })
            }
        }
    }

    // =========================================================================
    // Epic Breakdown
    // =========================================================================

    /// Break an epic into stories with dependencies.
    ///
    /// For now: rule-based keyword matching.
    /// In production: this would use the swarm's graphify knowledge
    /// to understand codebase structure and propose sensible boundaries.
    fn breakdown_epic(&self,
        epic_name: &str,
        description: &str,
    ) -> Result<Vec<StoryPlan>> {
        let mut stories = Vec::new();
        let desc_lower = description.to_lowercase();

        // Detect common feature areas
        if desc_lower.contains("friend") || desc_lower.contains("social") {
            stories.push(StoryPlan {
                name: format!("{} - Friendship System", epic_name),
                description: "Friend request, accept, list, unfriend".to_string(),
                estimated_minutes: 20,
                files_touched_estimate: 4,
                tables_modified_estimate: 2,
                modules_touched_estimate: 2,
                depends_on: vec![],
            });
        }

        if desc_lower.contains("feed") || desc_lower.contains("activity") {
            stories.push(StoryPlan {
                name: format!("{} - Activity Feed", epic_name),
                description: "Scrollable feed of friend activities".to_string(),
                estimated_minutes: 25,
                files_touched_estimate: 5,
                tables_modified_estimate: 2,
                modules_touched_estimate: 2,
                depends_on: vec![format!("{} - Friendship System", epic_name)],
            });
        }

        if desc_lower.contains("message") || desc_lower.contains("chat") {
            stories.push(StoryPlan {
                name: format!("{} - Direct Messaging", epic_name),
                description: "1:1 messaging between friends".to_string(),
                estimated_minutes: 30,
                files_touched_estimate: 5,
                tables_modified_estimate: 2,
                modules_touched_estimate: 2,
                depends_on: vec![format!("{} - Friendship System", epic_name)],
            });
        }

        if desc_lower.contains("notification") {
            stories.push(StoryPlan {
                name: format!("{} - Social Notifications", epic_name),
                description: "Push notifications for social events".to_string(),
                estimated_minutes: 20,
                files_touched_estimate: 3,
                tables_modified_estimate: 1,
                modules_touched_estimate: 1,
                depends_on: vec![
                    format!("{} - Friendship System", epic_name),
                    format!("{} - Activity Feed", epic_name),
                ],
            });
        }

        if desc_lower.contains("setting") || desc_lower.contains("privacy") {
            stories.push(StoryPlan {
                name: format!("{} - Social Settings", epic_name),
                description: "Privacy controls, visibility settings".to_string(),
                estimated_minutes: 15,
                files_touched_estimate: 3,
                tables_modified_estimate: 1,
                modules_touched_estimate: 1,
                depends_on: vec![format!("{} - Friendship System", epic_name)],
            });
        }

        // If no keywords matched, create a single default story
        if stories.is_empty() {
            stories.push(StoryPlan {
                name: format!("{} - Core Implementation", epic_name),
                description: description.to_string(),
                estimated_minutes: self.estimate_duration(description),
                files_touched_estimate: self.estimate_files(description),
                tables_modified_estimate: self.estimate_tables(description),
                modules_touched_estimate: self.estimate_modules(description),
                depends_on: vec![],
            });
        }

        info!("[PlanningAgent] Broke epic '{}' into {} stories", epic_name, stories.len());
        Ok(stories)
    }

    // =========================================================================
    // Persistence: Create Tasks in DB
    // =========================================================================

    /// Persist a plan to the database.
    /// Creates the epic (or story) task, child stories, and default phases.
    pub fn persist_plan(&self,
        plan: &Plan,
        branch_prefix: &str,
    ) -> Result<Vec<Task>> {
        let mut created_tasks = Vec::new();

        // Create epic (or parent story)
        let epic_task = self.create_task(
            &plan.epic_name,
            &plan.epic_description,
            branch_prefix,
            None,
            &plan.size,
        )?;
        created_tasks.push(epic_task.clone());

        // Create child stories
        for story_plan in &plan.stories {
            let story_task = self.create_task(
                &story_plan.name,
                &story_plan.description,
                &format!("{}/{}", branch_prefix, story_plan.name.to_lowercase().replace(" ", "-")),
                Some(&epic_task.id),
                &WorkSize::Story,
            )?;

            // Create default phases for this story
            self.phase_manager.create_default_phases(&story_task.id)?;

            created_tasks.push(story_task);
        }

        // Store dependencies
        for (i, story_plan) in plan.stories.iter().enumerate() {
            let story_id = &created_tasks[i + 1].id; // +1 because epic is index 0

            for dep_name in &story_plan.depends_on {
                // Find the story this depends on
                if let Some(dep_task) = created_tasks.iter().find(|t| t.name == *dep_name) {
                    let dep = StoryDependency {
                        story_id: story_id.clone(),
                        depends_on_story_id: dep_task.id.clone(),
                        dependency_type: "hard".to_string(),
                    };
                    self.db.add_story_dependency(&dep)?;
                }
            }
        }

        info!("[PlanningAgent] Persisted plan: {} tasks created", created_tasks.len());
        Ok(created_tasks)
    }

    fn create_task(
        &self,
        name: &str,
        description: &str,
        branch: &str,
        parent_id: Option<&str>,
        size: &WorkSize,
    ) -> Result<Task> {
        let task_level = match size {
            WorkSize::Epic => "epic",
            WorkSize::Story => "story",
            WorkSize::Task => "task",
        };

        let task = Task {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            branch: branch.to_string(),
            status: TaskStatus::Queued,
            created_at: Utc::now(),
            completed_at: None,
        };

        self.db.create_task_hierarchy(
            &task,
            parent_id,
            task_level,
            Some("sdlc_feature"),
        )?;

        Ok(task)
    }

    // =========================================================================
    // Estimation Helpers (rule-based)
    // =========================================================================

    fn estimate_files(&self, description: &str) -> u32 {
        let lower = description.to_lowercase();
        let count = lower.matches("file").count()
            + lower.matches("component").count()
            + lower.matches("screen").count()
            + lower.matches("module").count();
        std::cmp::max(count as u32, 2)
    }

    fn estimate_tables(&self, description: &str) -> u32 {
        let lower = description.to_lowercase();
        let count = lower.matches("table").count()
            + lower.matches("schema").count()
            + lower.matches("migration").count()
            + lower.matches("database").count();
        count as u32
    }

    fn estimate_modules(&self, description: &str) -> u32 {
        let lower = description.to_lowercase();
        let count = lower.matches("module").count()
            + lower.matches("service").count()
            + lower.matches("provider").count()
            + lower.matches("tab").count();
        std::cmp::max(count as u32, 1)
    }

    fn estimate_duration(&self, description: &str) -> u32 {
        let lower = description.to_lowercase();
        let base = if lower.contains("simple") || lower.contains("quick") || lower.contains("typo") {
            5
        } else if lower.contains("complex") || lower.contains("rebuild") {
            45
        } else {
            10
        };

        let modifiers = lower.matches("and").count() as u32
            + lower.matches("also").count() as u32
            + lower.matches("plus").count() as u32;

        base + modifiers * 5
    }

    fn infer_naming_conventions(&self,
        _description: &str,
    ) -> NamingConventions {
        // For now: default to MouminA conventions
        // In production: this would read from project config or graphify
        NamingConventions {
            functions: "camelCase".to_string(),
            tables: "snake_case".to_string(),
            components: "PascalCase".to_string(),
            variables: "camelCase".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_db() -> String {
        let path = format!("/tmp/test_planning_{}.db", Uuid::new_v4());
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn test_size_analysis() {
        let db_path = temp_db();
        let agent = PlanningAgent::new(&db_path).unwrap();

        // Small task
        let size = agent.analyze_size(
            "Fix typo", "Fix a typo in the header", None, None, None, None
        ).unwrap();
        assert_eq!(size, WorkSize::Task);

        // Epic (many files)
        let size = agent.analyze_size(
            "Social Tab Rebuild",
            "Rebuild the entire Social tab with friends, feed, messaging, notifications",
            Some(8), Some(4), Some(3), Some(120),
        ).unwrap();
        assert_eq!(size, WorkSize::Epic);

        let _ = fs::remove_file(&db_path);
    }

    #[test]
    fn test_epic_breakdown() {
        let db_path = temp_db();
        let agent = PlanningAgent::new(&db_path).unwrap();

        let plan = agent.create_plan(
            "Social Tab Rebuild",
            "Rebuild the entire Social tab with friends, feed, messaging, notifications",
            WorkSize::Epic,
        ).unwrap();

        assert_eq!(plan.size, WorkSize::Epic);
        assert!(plan.stories.len() >= 3); // Should detect friends, feed, messaging, notifications

        let _ = fs::remove_file(&db_path);
    }
}
