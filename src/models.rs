use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A Persona is an agent archetype with specific skills and a base role.
/// The Queen assigns a Personality (soul) to a Persona for each task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub id: String,
    pub name: String,
    pub role: String,
    pub skills: Vec<String>,
    pub base_personality: String,
    pub voice: String,
    pub mood_default: String,
    pub caveman_level: CavemanLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CavemanLevel {
    Low,      // Verbose output
    Medium,   // Balanced
    High,     // Compressed tokens
}

/// A Personality is a "soul" that gives character to any Persona.
/// Personalities have moods, speech patterns, and emotional states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Personality {
    pub id: String,
    pub name: String,
    pub description: String,
    pub triggers: Vec<String>,
    pub speech_patterns: Vec<String>,
    pub token_cost: TokenCost,
    pub mood_states: Vec<String>, // e.g., ["angry", "happy", "confused", "moody"]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenCost {
    Low,    // Caveman compatible
    Medium, // Slightly verbose
    High,   // Verbose but worth it
}

/// The MxN assignment: Queen maps Persona + Personality + Mood for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub task_id: String,
    pub persona_id: String,
    pub personality_id: String,
    pub mood: String,
    pub reason: String,
    pub assigned_at: DateTime<Utc>,
}

/// A Task represents a swarm mission (e.g., "Build Day Seal feature").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    pub branch: String,  // Git branch = sandbox room
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
}

/// A Letter is inter-agent communication within a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Letter {
    pub id: Uuid,
    pub task_id: String,
    pub from_persona: String,
    pub to_persona: Option<String>, // None = broadcast to all
    pub content: String,
    pub mood_at_send: String,
    pub sent_at: DateTime<Utc>,
}

/// A Diary entry is an agent's private reflection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryEntry {
    pub id: Uuid,
    pub task_id: String,
    pub persona_id: String,
    pub personality_id: String,
    pub entry: String,
    pub mood: String,
    pub written_at: DateTime<Utc>,
}

/// An ErrorLog entry records failures and their solutions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorLog {
    pub id: Uuid,
    pub task_id: String,
    pub persona_id: String,
    pub error_message: String,
    pub error_type: String,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub root_cause: Option<String>,
    pub solution: Option<String>,
    pub same_symptom_different_cause: bool,
    pub occurred_at: DateTime<Utc>,
}

/// A Phase represents a stage within a Story (Planning, Design, Implementation, Review, Ship).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryPhase {
    pub id: String,
    pub story_id: String,
    pub phase_number: i32,
    pub phase_name: String,
    pub status: PhaseStatus,
    pub topology: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub approved_by: Option<String>,
    pub approval_note: Option<String>,
    pub artifact_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    Pending,
    Running,
    Blocked,
    Reviewing,
    Approved,
    Rejected,
    Skipped,
}

/// Assignment of an agent to a specific phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseAssignment {
    pub id: String,
    pub phase_id: String,
    pub persona_id: String,
    pub personality_id: String,
    pub sub_task_description: Option<String>,
    pub status: AssignmentStatus,
    pub assigned_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Unified activity log entry for the event stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLogEntry {
    pub id: String,
    pub story_id: Option<String>,
    pub phase_id: Option<String>,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub action_type: String,
    pub payload: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Artifact produced by a phase (plan, design, code, review, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub story_id: Option<String>,
    pub phase_id: Option<String>,
    pub artifact_type: String,
    pub file_path: String,
    pub created_at: Option<DateTime<Utc>>,
    pub summary: Option<String>,
}

/// Dependency between stories within an epic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryDependency {
    pub story_id: String,
    pub depends_on_story_id: String,
    pub dependency_type: String,
}

/// Metrics tracked per phase execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseMetrics {
    pub id: String,
    pub phase_id: String,
    pub wall_clock_seconds: Option<f64>,
    pub tokens_input: Option<i64>,
    pub tokens_output: Option<i64>,
    pub tokens_total: Option<i64>,
    pub agent_invocations: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
}

/// The Queen's state for a running swarm.
#[derive(Debug)]
pub struct SwarmState {
    pub task: Task,
    pub assignments: Vec<Assignment>,
    pub letters: Vec<Letter>,
    pub diary: Vec<DiaryEntry>,
    pub errors: Vec<ErrorLog>,
}

/// Caveman compression: reduce token usage in agent communication.
pub fn caveman_compress(text: &str) -> String {
    let mut result = text.to_string();
    
    // Remove articles
    for word in &[" a ", " an ", " the "] {
        result = result.replace(word, " ");
    }
    
    // Remove filler words
    for word in &[" just ", " really ", " basically ", " actually ", " simply ", " essentially "] {
        result = result.replace(word, " ");
    }
    
    // Remove hedging
    for phrase in &["it might be worth", "you could consider", "it would be good to"] {
        result = result.replace(phrase, "consider");
    }
    
    // Shorten phrases
    result = result.replace("in order to", "to");
    result = result.replace("make sure to", "ensure");
    result = result.replace("the reason is because", "because");
    
    // Clean up multiple spaces
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }
    
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_caveman_compress() {
        let input = "You should always make sure to run the test suite before pushing any changes to the main branch.";
        let output = caveman_compress(input);
        assert!(!output.contains("the"));
        assert!(!output.contains("make sure to"));
    }
}
