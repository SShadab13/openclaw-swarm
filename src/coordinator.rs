use anyhow::Result;
use tracing::debug;
use crate::models::*;
use crate::db::Database;

/// The Coordinator is the Queen's Chamberlain/Majordomo.
/// It manages task execution flow, checks agent health, and reports to the Queen.
pub struct Coordinator {
    db: Database,
}

impl Coordinator {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = Database::new(db_path)?;
        Ok(Self { db })
    }
    
    /// Execute a task step: check all agents, collect their work, resolve conflicts.
    pub fn execute_step(&self, task_id: &str) -> Result<StepResult> {
        let assignments = self.db.get_task_assignments(task_id)?;
        let letters = self.db.get_task_letters(task_id)?;
        
        debug!("Task {}: {} agents, {} letters", task_id, assignments.len(), letters.len());
        
        // Check for blocking letters (complaints, conflicts)
        let blocking_issues: Vec<&Letter> = letters.iter()
            .filter(|l| l.content.contains("BLOCKING") || l.content.contains("BROKEN"))
            .collect();
        
        if !blocking_issues.is_empty() {
            return Ok(StepResult {
                status: StepStatus::Blocked,
                message: format!("{} blocking issues found", blocking_issues.len()),
                action_required: Some("Queen intervention needed".to_string()),
            });
        }
        
        // Check for completed work (all agents report DONE)
        let done_count = letters.iter()
            .filter(|l| l.content.contains("DONE") || l.content.contains("COMPLETE"))
            .count();
        
        if done_count >= assignments.len() {
            return Ok(StepResult {
                status: StepStatus::ReadyToMerge,
                message: "All agents report completion".to_string(),
                action_required: Some("Queen approves merge".to_string()),
            });
        }
        
        Ok(StepResult {
            status: StepStatus::InProgress,
            message: format!("{}/{} agents complete", done_count, assignments.len()),
            action_required: None,
        })
    }
    
    /// Collect all diary entries for a task review.
    pub fn collect_diary(&self, _task_id: &str) -> Result<Vec<DiaryEntry>> {
        // In a real implementation, this queries the db
        // For now, return empty (placeholder)
        Ok(Vec::new())
    }
    
    /// Summarize the swarm's emotional state for the Queen.
    pub fn swarm_mood_report(&self, task_id: &str) -> Result<String> {
        let assignments = self.db.get_task_assignments(task_id)?;
        
        let moods: Vec<String> = assignments.iter().map(|a| a.mood.clone()).collect();
        
        let report = format!(
            "Swarm mood report for {}:\n  Agents: {}\n  Moods: {}\n  Status: {}",
            task_id,
            assignments.len(),
            moods.join(", "),
            if moods.iter().any(|m| m == "angry" || m == "frustrated") {
                "⚠️ TENSION DETECTED"
            } else {
                "✅ Stable"
            }
        );
        
        Ok(report)
    }
}

#[derive(Debug)]
pub struct StepResult {
    pub status: StepStatus,
    pub message: String,
    pub action_required: Option<String>,
}

#[derive(Debug)]
pub enum StepStatus {
    InProgress,
    Blocked,
    ReadyToMerge,
    Failed,
}
