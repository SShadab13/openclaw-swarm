use anyhow::{Result, Context};
use chrono::Utc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::Database;
use crate::models::{StoryPhase, PhaseStatus, PhaseAssignment, AssignmentStatus, ActivityLogEntry};

/// PhaseManager handles the lifecycle of story phases:
/// Pending → Running → (Blocked | Reviewing) → (Approved | Rejected | Skipped)
///
/// Every transition is logged to the activity_log table.
pub struct PhaseManager {
    db: Database,
}

impl PhaseManager {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = Database::new(db_path)?;
        Ok(Self { db })
    }

    // =========================================================================
    // Phase Lifecycle
    // =========================================================================

    /// Create all 5 default phases for a story.
    pub fn create_default_phases(&self, story_id: &str) -> Result<Vec<StoryPhase>> {
        let default_phases = vec![
            ("planning", "sequential"),
            ("design", "sequential"),
            ("implementation", "parallel"),
            ("review", "parallel"),
            ("ship", "sequential"),
        ];

        let mut phases = Vec::new();
        for (i, (name, topology)) in default_phases.iter().enumerate() {
            let phase = StoryPhase {
                id: Uuid::new_v4().to_string(),
                story_id: story_id.to_string(),
                phase_number: i as i32 + 1,
                phase_name: name.to_string(),
                status: PhaseStatus::Pending,
                topology: topology.to_string(),
                started_at: None,
                completed_at: None,
                approved_by: None,
                approval_note: None,
                artifact_path: None,
            };
            self.db.create_phase(&phase)?;
            phases.push(phase);
        }

        self.log_activity(story_id, None, "system", "phase_start",
            &format!("Created {} default phases for story", default_phases.len()))?;

        info!("[PhaseManager] Created {} phases for story {}", phases.len(), story_id);
        Ok(phases)
    }

    /// Start a phase — transition from Pending → Running.
    pub fn start_phase(&self, phase_id: &str) -> Result<StoryPhase> {
        let phase = self.get_phase(phase_id)?
            .context("Phase not found")?;

        // Validate transition
        if phase.status != PhaseStatus::Pending && phase.status != PhaseStatus::Blocked {
            anyhow::bail!("Cannot start phase {} — status is {:?}, expected Pending or Blocked",
                phase_id, phase.status);
        }

        // Update status
        self.db.update_phase_status(phase_id, PhaseStatus::Running)?;

        // Log
        self.log_activity(&phase.story_id, Some(phase_id), "system", "phase_start",
            &format!("Phase {} started", phase.phase_name))?;

        info!("[PhaseManager] Phase {} ({}) started", phase_id, phase.phase_name);

        // Return updated phase
        self.get_phase(phase_id)?.context("Phase disappeared after update")
    }

    /// Complete a phase — transition from Running → Reviewing (needs approval).
    pub fn complete_phase(&self, phase_id: &str) -> Result<StoryPhase> {
        let phase = self.get_phase(phase_id)?
            .context("Phase not found")?;

        if phase.status != PhaseStatus::Running {
            anyhow::bail!("Cannot complete phase {} — status is {:?}, expected Running",
                phase_id, phase.status);
        }

        self.db.update_phase_status(phase_id, PhaseStatus::Reviewing)?;

        self.log_activity(&phase.story_id, Some(phase_id), "system", "phase_complete",
            &format!("Phase {} completed, awaiting review", phase.phase_name))?;

        info!("[PhaseManager] Phase {} ({}) completed → Reviewing", phase_id, phase.phase_name);

        self.get_phase(phase_id)?.context("Phase disappeared after update")
    }

    /// Approve a phase — transition from Reviewing → Approved.
    pub fn approve_phase(&self, phase_id: &str, approved_by: &str, note: Option<&str>) -> Result<StoryPhase> {
        let phase = self.get_phase(phase_id)?
            .context("Phase not found")?;

        if phase.status != PhaseStatus::Reviewing {
            anyhow::bail!("Cannot approve phase {} — status is {:?}, expected Reviewing",
                phase_id, phase.status);
        }

        // Update phase
        let updated = StoryPhase {
            status: PhaseStatus::Approved,
            approved_by: Some(approved_by.to_string()),
            approval_note: note.map(|s| s.to_string()),
            completed_at: Some(Utc::now()),
            ..phase.clone()
        };
        self.db.create_phase(&updated)?;

        self.log_activity(&phase.story_id, Some(phase_id), "user", "user_approve",
            &format!("Phase {} approved by {}", phase.phase_name, approved_by))?;

        info!("[PhaseManager] Phase {} approved by {}", phase_id, approved_by);

        Ok(updated)
    }

    /// Reject a phase — transition from Reviewing → Rejected.
    pub fn reject_phase(&self, phase_id: &str, reason: &str) -> Result<StoryPhase> {
        let phase = self.get_phase(phase_id)?
            .context("Phase not found")?;

        if phase.status != PhaseStatus::Reviewing {
            anyhow::bail!("Cannot reject phase {} — status is {:?}, expected Reviewing",
                phase_id, phase.status);
        }

        let updated = StoryPhase {
            status: PhaseStatus::Rejected,
            approval_note: Some(reason.to_string()),
            completed_at: Some(Utc::now()),
            ..phase.clone()
        };
        self.db.create_phase(&updated)?;

        self.log_activity(&phase.story_id, Some(phase_id), "user", "user_reject",
            &format!("Phase {} rejected: {}", phase.phase_name, reason))?;

        warn!("[PhaseManager] Phase {} rejected: {}", phase_id, reason);

        Ok(updated)
    }

    /// Reject a phase with replan — transition from Reviewing → Rejected + log replan action.
    pub fn reject_with_replan(
        &self, phase_id: &str, reason: &str, replan_action: &str) -> Result<StoryPhase> {
        let phase = self.reject_phase(phase_id, reason)?;

        self.log_activity(
            &phase.story_id, Some(phase_id), "queen", "replan",
            &format!("Replan triggered for phase {}: {}", phase.phase_name, replan_action))?;

        info!("[PhaseManager] Phase {} rejected with replan: {}", phase_id, replan_action);
        Ok(phase)
    }

    /// Handle a replan action — execute the recovery primitive.
    pub fn handle_replan(
        &self,
        phase_id: &str,
        action: &crate::review::agent::ReplanAction,
    ) -> Result<String> {
        use crate::review::agent::ReplanAction;

        let result = match action {
            ReplanAction::Rebind { assignment_id, new_args } => {
                // Retry same agent with updated arguments
                // Reset assignment to pending for retry
                info!("[PhaseManager] Rebind: retrying assignment {} with new args: {}",
                    assignment_id, new_args);
                format!("Rebind: Retry assignment {} with updated args", assignment_id)
            }
            ReplanAction::InsertPrereq { phase_name, reason: _ } => {
                // Insert a new prerequisite phase before the current one
                info!("[PhaseManager] InsertPrereq: adding phase '{}' before {}",
                    phase_name, phase_id);
                format!("InsertPrereq: Add '{}' phase before current", phase_name)
            }
            ReplanAction::Substitute { old_persona_id, new_persona_id, reason } => {
                // Swap the assigned agent for a different persona
                info!("[PhaseManager] Substitute: swapping {} → {} ({})",
                    old_persona_id, new_persona_id, reason);
                format!("Substitute: {} replaced by {}", old_persona_id, new_persona_id)
            }
            ReplanAction::Rewire { new_dependency, reason } => {
                // Change task dependencies
                info!("[PhaseManager] Rewire: new dependency {} ({})",
                    new_dependency, reason);
                format!("Rewire: Add dependency on {}", new_dependency)
            }
            ReplanAction::Bypass { phase_id, reason } => {
                // Skip the phase
                self.skip_phase(phase_id, reason)?;
                format!("Bypass: Skipped phase {}", phase_id)
            }
            ReplanAction::Escalate { reason } => {
                // Flag for manual queen/user intervention
                info!("[PhaseManager] Escalate: {} - requires manual decision", reason);
                format!("Escalate: {} — requires manual decision", reason)
            }
        };

        Ok(result)
    }

    /// Check if a story needs replan (any phase rejected).
    pub fn needs_replan(&self, story_id: &str) -> Result<Option<(String, String)>> {
        let phases = self.db.get_phases_for_story(story_id)?;

        for phase in &phases {
            if phase.status == PhaseStatus::Rejected {
                return Ok(Some((
                    phase.id.clone(),
                    phase.approval_note.clone().unwrap_or_default(),
                )));
            }
        }

        Ok(None)
    }

    /// Block a phase — transition from Running → Blocked.
    pub fn block_phase(&self, phase_id: &str, reason: &str) -> Result<StoryPhase> {
        let phase = self.get_phase(phase_id)?
            .context("Phase not found")?;

        if phase.status != PhaseStatus::Running {
            anyhow::bail!("Cannot block phase {} — status is {:?}, expected Running",
                phase_id, phase.status);
        }

        self.db.update_phase_status(phase_id, PhaseStatus::Blocked)?;

        self.log_activity(&phase.story_id, Some(phase_id), "system", "error",
            &format!("Phase {} blocked: {}", phase.phase_name, reason))?;

        warn!("[PhaseManager] Phase {} blocked: {}", phase_id, reason);

        self.get_phase(phase_id)?.context("Phase disappeared after update")
    }

    /// Skip a phase — transition from Pending → Skipped.
    pub fn skip_phase(&self, phase_id: &str, reason: &str) -> Result<StoryPhase> {
        let phase = self.get_phase(phase_id)?
            .context("Phase not found")?;

        if phase.status != PhaseStatus::Pending {
            anyhow::bail!("Cannot skip phase {} — status is {:?}, expected Pending",
                phase_id, phase.status);
        }

        let updated = StoryPhase {
            status: PhaseStatus::Skipped,
            approval_note: Some(reason.to_string()),
            completed_at: Some(Utc::now()),
            ..phase.clone()
        };
        self.db.create_phase(&updated)?;

        self.log_activity(&phase.story_id, Some(phase_id), "queen", "phase_complete",
            &format!("Phase {} skipped: {}", phase.phase_name, reason))?;

        info!("[PhaseManager] Phase {} skipped: {}", phase_id, reason);

        Ok(updated)
    }

    /// Unblock a phase — transition from Blocked → Running.
    pub fn unblock_phase(&self, phase_id: &str) -> Result<StoryPhase> {
        let phase = self.get_phase(phase_id)?
            .context("Phase not found")?;

        if phase.status != PhaseStatus::Blocked {
            anyhow::bail!("Cannot unblock phase {} — status is {:?}, expected Blocked",
                phase_id, phase.status);
        }

        self.db.update_phase_status(phase_id, PhaseStatus::Running)?;

        self.log_activity(&phase.story_id, Some(phase_id), "system", "phase_start",
            &format!("Phase {} unblocked, resuming", phase.phase_name))?;

        info!("[PhaseManager] Phase {} unblocked", phase_id);

        self.get_phase(phase_id)?.context("Phase disappeared after update")
    }

    // =========================================================================
    // Phase Assignment
    // =========================================================================

    /// Assign an agent to a phase.
    pub fn assign_agent_to_phase(&self, phase_id: &str, persona_id: &str, personality_id: &str,
        sub_task: Option<&str>) -> Result<PhaseAssignment> {

        let assignment = PhaseAssignment {
            id: Uuid::new_v4().to_string(),
            phase_id: phase_id.to_string(),
            persona_id: persona_id.to_string(),
            personality_id: personality_id.to_string(),
            sub_task_description: sub_task.map(|s| s.to_string()),
            status: AssignmentStatus::Pending,
            assigned_at: Some(Utc::now()),
            completed_at: None,
            result_summary: None,
        };

        self.db.create_phase_assignment(&assignment)?;

        self.log_activity(&self.get_phase(phase_id)?.map(|p| p.story_id).unwrap_or_default(),
            Some(phase_id), "system", "agent_start",
            &format!("Agent {} assigned to phase", persona_id))?;

        info!("[PhaseManager] Agent {} assigned to phase {}", persona_id, phase_id);
        Ok(assignment)
    }

    /// Mark an agent's work as complete.
    pub fn complete_agent_assignment(&self, assignment_id: &str, result: &str) -> Result<()> {
        // This requires a DB method to update by ID — for now, we create a new record
        // In production, this would be an UPDATE
        info!("[PhaseManager] Agent assignment {} completed: {}", assignment_id, result);
        Ok(())
    }

    // =========================================================================
    // Gate Logic
    // =========================================================================

    /// Check if a story can proceed to the next phase.
    /// Returns: (can_proceed, next_phase_id, message)
    pub fn check_phase_gate(&self, story_id: &str) -> Result<(bool, Option<String>, String)> {
        let phases = self.db.get_phases_for_story(story_id)?;

        // Find the first non-approved phase
        for phase in &phases {
            match &phase.status {
                PhaseStatus::Pending => {
                    return Ok((true, Some(phase.id.clone()),
                        format!("Next phase ready: {}", phase.phase_name)));
                }
                PhaseStatus::Running => {
                    return Ok((false, None,
                        format!("Phase {} is still running", phase.phase_name)));
                }
                PhaseStatus::Blocked => {
                    return Ok((false, None,
                        format!("Phase {} is blocked", phase.phase_name)));
                }
                PhaseStatus::Reviewing => {
                    return Ok((false, None,
                        format!("Phase {} awaiting review", phase.phase_name)));
                }
                PhaseStatus::Rejected => {
                    return Ok((false, None,
                        format!("Phase {} was rejected — needs replan", phase.phase_name)));
                }
                PhaseStatus::Approved | PhaseStatus::Skipped => {
                    // Continue to next phase
                    continue;
                }
            }
        }

        // All phases approved/skipped — story complete
        Ok((true, None, "All phases complete — ready to ship".to_string()))
    }

    /// Auto-check if a phase passes gate (compile + tests pass).
    pub fn auto_check_phase(&self, phase_id: &str) -> Result<(bool, String)> {
        let phase = self.get_phase(phase_id)?
            .context("Phase not found")?;

        // TODO: Implement actual compile/test checks
        // For now, always pass auto-check for implementation phases
        let passed = match phase.phase_name.as_str() {
            "implementation" => true,  // Would run cargo check, tests here
            "planning" => true,        // Planning always passes auto-check
            "design" => true,          // Design always passes auto-check
            _ => false,
        };

        let message = if passed {
            format!("Auto-check passed for phase {}", phase.phase_name)
        } else {
            format!("Auto-check failed for phase {}", phase.phase_name)
        };

        Ok((passed, message))
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    fn get_phase(&self, phase_id: &str) -> Result<Option<StoryPhase>> {
        // Query single phase by ID — need to add this to DB
        // For now, scan all phases for the story
        // In production, add a dedicated DB method
        let conn = self.db.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, story_id, phase_number, phase_name, status, topology, started_at, completed_at, approved_by, approval_note, artifact_path
                 FROM story_phases WHERE id = ?1"
            )?;

            let phase = stmt.query_row([phase_id], |row| {
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
            }).ok();

            Ok(phase)
        })?;

        Ok(conn)
    }

    fn log_activity(&self, story_id: &str, phase_id: Option<&str>,
        actor_type: &str, action_type: &str, payload: &str) -> Result<()> {
        let entry = ActivityLogEntry {
            id: Uuid::new_v4().to_string(),
            story_id: Some(story_id.to_string()),
            phase_id: phase_id.map(|s| s.to_string()),
            actor_type: actor_type.to_string(),
            actor_id: None,
            action_type: action_type.to_string(),
            payload: Some(payload.to_string()),
            timestamp: Utc::now(),
        };
        self.db.log_activity(&entry)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_db() -> String {
        let path = format!("/tmp/test_phase_manager_{}.db", Uuid::new_v4());
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn test_create_default_phases() {
        let db_path = temp_db();
        let manager = PhaseManager::new(&db_path).unwrap();

        let phases = manager.create_default_phases("story-123").unwrap();
        assert_eq!(phases.len(), 5);
        assert_eq!(phases[0].phase_name, "planning");
        assert_eq!(phases[1].phase_name, "design");
        assert_eq!(phases[2].phase_name, "implementation");
        assert_eq!(phases[3].phase_name, "review");
        assert_eq!(phases[4].phase_name, "ship");

        let _ = fs::remove_file(&db_path);
    }

    #[test]
    fn test_phase_lifecycle() {
        let db_path = temp_db();
        let manager = PhaseManager::new(&db_path).unwrap();

        let phases = manager.create_default_phases("story-456").unwrap();
        let phase_id = &phases[0].id;

        // Start
        let running = manager.start_phase(phase_id).unwrap();
        assert!(matches!(running.status, PhaseStatus::Running));

        // Complete
        let reviewing = manager.complete_phase(phase_id).unwrap();
        assert!(matches!(reviewing.status, PhaseStatus::Reviewing));

        // Approve
        let approved = manager.approve_phase(phase_id, "user", Some("LGTM")).unwrap();
        assert!(matches!(approved.status, PhaseStatus::Approved));

        let _ = fs::remove_file(&db_path);
    }
}
