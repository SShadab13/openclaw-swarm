use anyhow::Result;
use tracing::{info, debug};

use crate::db::Database;
use crate::models::{Artifact, ActivityLogEntry};
use crate::activity::logger::ActivityLogger;

/// ReviewAgent performs 3-dimension parallel code review:
/// 1. Simplicity/DRY — "Is this the minimum code needed?"
/// 2. Bugs/Correctness — "Run edge case analysis"
/// 3. Conventions — "Match existing project patterns?"
///
/// Based on AGENTS.md mental models: Falsifiability, Occam's Razor, Inversion.
pub struct ReviewAgent {
    db: Database,
    logger: ActivityLogger,
}

/// A single finding from a reviewer.
#[derive(Debug, Clone)]
pub struct ReviewFinding {
    pub dimension: ReviewDimension,
    pub severity: Severity,
    pub file_path: String,
    pub line_number: Option<u32>,
    pub description: String,
    pub suggestion: Option<String>,
    pub principle: String, // Which mental model triggered this finding
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewDimension {
    Simplicity,
    Bugs,
    Conventions,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Severity {
    Info,      // Suggestion, not blocking
    Warning,   // Should fix, but not blocking
    Error,     // Must fix before approval
    Critical,  // Security/integrity issue, blocks immediately
}

/// Complete review report for a phase.
#[derive(Debug, Clone)]
pub struct ReviewReport {
    pub phase_id: String,
    pub story_id: String,
    pub reviewer_id: String,
    pub findings: Vec<ReviewFinding>,
    pub overall_verdict: ReviewVerdict,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReviewVerdict {
    Approve,       // No issues or only info
    ApproveWithNotes, // Warnings only
    RequestChanges,  // Errors found
    Reject,        // Critical issues
}

/// Replan recommendation when review finds issues.
#[derive(Debug, Clone)]
pub enum ReplanAction {
    Rebind {              // Retry same agent with updated args
        assignment_id: String,
        new_args: String,
    },
    InsertPrereq {        // Add a missing prerequisite phase
        phase_name: String,
        reason: String,
    },
    Substitute {          // Swap agent for different persona
        old_persona_id: String,
        new_persona_id: String,
        reason: String,
    },
    Rewire {              // Change task dependencies
        new_dependency: String,
        reason: String,
    },
    Bypass {              // Skip if downstream already satisfied
        phase_id: String,
        reason: String,
    },
    Escalate {            // Queen/user decides
        reason: String,
    },
}

impl ReviewAgent {
    pub fn new(db_path: &str) -> Result<Self> {
        let db = Database::new(db_path)?;
        let logger = ActivityLogger::new(db_path)?;
        Ok(Self { db, logger })
    }

    /// Run a 3-dimension review on a phase's artifacts.
    ///
    /// In production: this reads actual code from artifacts table,
    /// runs static analysis, and applies mental model principles.
    /// For now: rule-based review from artifact metadata.
    pub fn review_phase(
        &self,
        phase_id: &str,
        reviewer_id: &str,
    ) -> Result<ReviewReport> {
        let artifacts = self.db.get_artifacts_for_phase(phase_id)?;
        let activity = self.db.get_activity_for_story(
            &self.get_story_id_for_phase(phase_id)?, 50)?;

        let mut findings = Vec::new();

        // Dimension 1: Simplicity / DRY
        findings.extend(self.review_simplicity(&artifacts)?);

        // Dimension 2: Bugs / Correctness
        findings.extend(self.review_correctness(&artifacts, &activity)?);

        // Dimension 3: Conventions
        findings.extend(self.review_conventions(&artifacts)?);

        // Determine verdict
        let overall_verdict = self.determine_verdict(&findings);
        let summary = self.generate_summary(&findings, &overall_verdict);

        // Log review submission
        self.logger.log_review(
            &self.get_story_id_for_phase(phase_id)?,
            phase_id,
            reviewer_id,
            &summary,
        )?;

        info!("[ReviewAgent] {} findings for phase {}, verdict: {:?}",
            findings.len(), phase_id, overall_verdict);

        Ok(ReviewReport {
            phase_id: phase_id.to_string(),
            story_id: self.get_story_id_for_phase(phase_id)?,
            reviewer_id: reviewer_id.to_string(),
            findings,
            overall_verdict,
            summary,
        })
    }

    /// Generate a replan recommendation based on review findings.
    pub fn recommend_replan(&self, report: &ReviewReport) -> Option<ReplanAction> {
        let has_critical = report.findings.iter().any(|f| f.severity == Severity::Critical);
        let has_errors = report.findings.iter().any(|f| f.severity == Severity::Error);
        let has_warnings = report.findings.iter().any(|f| f.severity == Severity::Warning);

        if has_critical {
            return Some(ReplanAction::Escalate {
                reason: format!("Critical issues found: {}",
                    report.findings.iter()
                        .filter(|f| f.severity == Severity::Critical)
                        .map(|f| f.description.clone())
                        .collect::<Vec<_>>()
                        .join("; ")),
            });
        }

        if has_errors {
            // Check if errors are from a specific dimension
            let bug_errors: Vec<_> = report.findings.iter()
                .filter(|f| f.severity == Severity::Error && f.dimension == ReviewDimension::Bugs)
                .collect();

            if !bug_errors.is_empty() {
                return Some(ReplanAction::Rebind {
                    assignment_id: report.phase_id.clone(),
                    new_args: format!("Fix: {}",
                        bug_errors.iter().map(|f| f.description.clone()).collect::<Vec<_>>().join("; ")),
                });
            }

            let conv_errors: Vec<_> = report.findings.iter()
                .filter(|f| f.severity == Severity::Error && f.dimension == ReviewDimension::Conventions)
                .collect();

            if !conv_errors.is_empty() {
                return Some(ReplanAction::Rebind {
                    assignment_id: report.phase_id.clone(),
                    new_args: format!("Refactor to match conventions: {}",
                        conv_errors.iter().map(|f| f.suggestion.clone().unwrap_or_default()).collect::<Vec<_>>().join("; ")),
                });
            }
        }

        if has_warnings {
            return Some(ReplanAction::Bypass {
                phase_id: report.phase_id.clone(),
                reason: "Warnings only — address in next story".to_string(),
            });
        }

        None // No replan needed — approve
    }

    // =========================================================================
    // Review Dimensions
    // =========================================================================

    fn review_simplicity(&self, artifacts: &[Artifact]) -> Result<Vec<ReviewFinding>> {
        let mut findings = Vec::new();

        for artifact in artifacts {
            // Heuristic: code artifacts with long file paths might be over-engineered
            if artifact.artifact_type == "code" && artifact.file_path.len() > 80 {
                findings.push(ReviewFinding {
                    dimension: ReviewDimension::Simplicity,
                    severity: Severity::Info,
                    file_path: artifact.file_path.clone(),
                    line_number: None,
                    description: "File path is long — consider if this abstraction is necessary".to_string(),
                    suggestion: Some("Apply Occam's Razor: is this the simplest path?".to_string()),
                    principle: "Occam's Razor".to_string(),
                });
            }

            // Heuristic: artifacts without summaries might be too complex to explain
            if artifact.summary.is_none() {
                findings.push(ReviewFinding {
                    dimension: ReviewDimension::Simplicity,
                    severity: Severity::Info,
                    file_path: artifact.file_path.clone(),
                    line_number: None,
                    description: "No summary provided — complex code should be explainable in one paragraph".to_string(),
                    suggestion: Some("Add a one-paragraph summary of what this code does".to_string()),
                    principle: "Simplicity".to_string(),
                });
            }
        }

        debug!("[ReviewAgent] Simplicity: {} findings", findings.len());
        Ok(findings)
    }

    fn review_correctness(
        &self,
        artifacts: &[Artifact],
        activity: &[ActivityLogEntry],
    ) -> Result<Vec<ReviewFinding>> {
        let mut findings = Vec::new();

        // Heuristic: check for error events in activity log
        let errors: Vec<_> = activity.iter()
            .filter(|a| a.action_type == "error")
            .collect();

        for error in errors {
            findings.push(ReviewFinding {
                dimension: ReviewDimension::Bugs,
                severity: Severity::Error,
                file_path: error.payload.clone().unwrap_or_default(),
                line_number: None,
                description: format!("Error during execution: {}", error.payload.clone().unwrap_or_default()),
                suggestion: Some("Investigate root cause before approving".to_string()),
                principle: "Falsifiability".to_string(),
            });
        }

        // Heuristic: code artifacts should have test artifacts
        let code_count = artifacts.iter().filter(|a| a.artifact_type == "code").count();
        let test_count = artifacts.iter().filter(|a| a.artifact_type == "test_report").count();

        if code_count > 0 && test_count == 0 {
            findings.push(ReviewFinding {
                dimension: ReviewDimension::Bugs,
                severity: Severity::Warning,
                file_path: "N/A".to_string(),
                line_number: None,
                description: "Code artifacts present but no test report found".to_string(),
                suggestion: Some("Add tests before approval — 'write test that reproduces, then make it pass'".to_string()),
                principle: "Inversion".to_string(),
            });
        }

        debug!("[ReviewAgent] Correctness: {} findings", findings.len());
        Ok(findings)
    }

    fn review_conventions(&self, artifacts: &[Artifact]) -> Result<Vec<ReviewFinding>> {
        let mut findings = Vec::new();

        for artifact in artifacts {
            // Heuristic: naming convention checks
            let path = &artifact.file_path;

            // Check for inconsistent naming (snake_case vs camelCase)
            if path.contains("newFunction") || path.contains("newVariable") {
                findings.push(ReviewFinding {
                    dimension: ReviewDimension::Conventions,
                    severity: Severity::Error,
                    file_path: path.clone(),
                    line_number: None,
                    description: "Inconsistent naming: found camelCase where snake_case expected".to_string(),
                    suggestion: Some("Follow the plan's naming conventions — ask leader if unsure".to_string()),
                    principle: "Consistency".to_string(),
                });
            }

            // Check for hardcoded values that should be constants
            // (In production: actual AST analysis)
            if artifact.artifact_type == "code" && artifact.summary.as_ref().map(|s| s.contains("hardcoded")).unwrap_or(false) {
                findings.push(ReviewFinding {
                    dimension: ReviewDimension::Conventions,
                    severity: Severity::Warning,
                    file_path: path.clone(),
                    line_number: None,
                    description: "Hardcoded values detected — use constants or config".to_string(),
                    suggestion: Some("Extract magic numbers to named constants".to_string()),
                    principle: "Maintainability".to_string(),
                });
            }
        }

        debug!("[ReviewAgent] Conventions: {} findings", findings.len());
        Ok(findings)
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    fn determine_verdict(&self, findings: &[ReviewFinding]) -> ReviewVerdict {
        let has_critical = findings.iter().any(|f| f.severity == Severity::Critical);
        let has_errors = findings.iter().any(|f| f.severity == Severity::Error);
        let has_warnings = findings.iter().any(|f| f.severity == Severity::Warning);

        if has_critical {
            ReviewVerdict::Reject
        } else if has_errors {
            ReviewVerdict::RequestChanges
        } else if has_warnings {
            ReviewVerdict::ApproveWithNotes
        } else {
            ReviewVerdict::Approve
        }
    }

    fn generate_summary(&self, findings: &[ReviewFinding], verdict: &ReviewVerdict) -> String {
        let counts = [
            ("critical", findings.iter().filter(|f| f.severity == Severity::Critical).count()),
            ("error", findings.iter().filter(|f| f.severity == Severity::Error).count()),
            ("warning", findings.iter().filter(|f| f.severity == Severity::Warning).count()),
            ("info", findings.iter().filter(|f| f.severity == Severity::Info).count()),
        ];

        format!(
            "Review: {} critical, {} errors, {} warnings, {} info. Verdict: {:?}",
            counts[0].1, counts[1].1, counts[2].1, counts[3].1, verdict
        )
    }

    fn get_story_id_for_phase(&self, phase_id: &str) -> Result<String> {
        self.db.with_conn(|conn| {
            let story_id: String = conn.query_row(
                "SELECT story_id FROM story_phases WHERE id = ?1",
                [phase_id],
                |row| row.get(0),
            )?;
            Ok(story_id)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use uuid::Uuid;

    fn temp_db() -> String {
        let path = format!("/tmp/test_review_{}.db", Uuid::new_v4());
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn test_review_verdict() {
        let agent = ReviewAgent::new("/tmp/test_review_empty.db").unwrap();

        let findings = vec![
            ReviewFinding {
                dimension: ReviewDimension::Simplicity,
                severity: Severity::Info,
                file_path: "test.rs".to_string(),
                line_number: None,
                description: "Consider simplifying".to_string(),
                suggestion: None,
                principle: "Occam".to_string(),
            },
        ];

        assert_eq!(agent.determine_verdict(&findings), ReviewVerdict::Approve);

        let findings = vec![
            ReviewFinding {
                dimension: ReviewDimension::Bugs,
                severity: Severity::Error,
                file_path: "test.rs".to_string(),
                line_number: None,
                description: "Null pointer".to_string(),
                suggestion: None,
                principle: "Falsifiability".to_string(),
            },
        ];

        assert_eq!(agent.determine_verdict(&findings), ReviewVerdict::RequestChanges);
    }
}
