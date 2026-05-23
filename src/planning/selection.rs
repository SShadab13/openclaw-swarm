use anyhow::Result;
use std::collections::HashMap;

/// AgentSelection implements "Queen Knows Her Court" — semantic agent
/// selection via persona capability matching + symbolic keyword triggers.
///
/// NOT vector embeddings. Deterministic, explainable, fast, zero API deps.
///
/// Algorithm:
/// 1. Phase gate: does persona work on this phase?
/// 2. Symbolic trigger: keyword matching from capabilities
/// 3. Task pattern matching: sample tasks
/// 4. Score sort, take top N for phase topology
pub struct AgentSelector;

/// A persona with declared capabilities and phase preferences.
#[derive(Debug, Clone)]
pub struct PersonaCapabilities {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<String>,
    pub sample_tasks: Vec<String>,
    pub phases_work_on: Vec<String>,
    pub phases_dont_work_on: Vec<String>,
    pub personality_id: String,
}

/// Selected agent with relevance score and reason.
#[derive(Debug, Clone)]
pub struct SelectedAgent {
    pub persona_id: String,
    pub personality_id: String,
    pub name: String,
    pub score: i32,
    pub reason: String,
    pub sub_task: String,
}

impl AgentSelector {
    pub fn new() -> Self {
        Self
    }

    /// Select agents for a phase based on task description and phase name.
    ///
    /// Returns agents sorted by relevance (highest first).
    pub fn select_agents(
        &self,
        task_description: &str,
        phase_name: &str,
        available_personas: &[PersonaCapabilities],
    ) -> Result<Vec<SelectedAgent>> {
        let task_lower = task_description.to_lowercase();
        let phase_lower = phase_name.to_lowercase();
        let mut candidates: Vec<(SelectedAgent, i32)> = Vec::new();

        for persona in available_personas {
            // Gate 1: Does this persona work on this phase?
            if !persona.phases_work_on.is_empty() && !persona.phases_work_on.contains(&phase_lower) {
                continue;
            }
            if persona.phases_dont_work_on.contains(&phase_lower) {
                continue;
            }

            // Gate 2: Symbolic trigger — keyword matching from capabilities
            let mut capability_score = 0;
            let mut matched_caps = Vec::new();

            for cap in &persona.capabilities {
                let cap_lower = cap.to_lowercase();
                if task_lower.contains(&cap_lower) {
                    capability_score += 1;
                    matched_caps.push(cap.clone());
                }
            }

            // Gate 3: Task pattern matching
            let mut task_match = false;
            for sample in &persona.sample_tasks {
                let sample_lower = sample.to_lowercase();
                if task_lower.contains(&sample_lower) || sample_lower.contains(&task_lower) {
                    task_match = true;
                    break;
                }
            }

            let total_score = capability_score + if task_match { 2 } else { 0 };

            if total_score > 0 || task_match {
                let reason = if task_match {
                    format!("Direct task match: '{}'", persona.sample_tasks.first().unwrap_or(&String::new()))
                } else if !matched_caps.is_empty() {
                    format!("Capability match: {}", matched_caps.join(", "))
                } else {
                    "Phase-compatible (no direct match)".to_string()
                };

                let sub_task = self.infer_sub_task(&task_lower, &persona.capabilities);

                candidates.push((SelectedAgent {
                    persona_id: persona.id.clone(),
                    personality_id: persona.personality_id.clone(),
                    name: persona.name.clone(),
                    score: total_score,
                    reason,
                    sub_task,
                }, total_score));
            }
        }

        // Sort by relevance score (descending)
        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        // Limit by topology
        let max_agents = Self::get_topology_max_agents(phase_name);
        let selected: Vec<SelectedAgent> = candidates.into_iter()
            .take(max_agents)
            .map(|(agent, _)| agent)
            .collect();

        Ok(selected)
    }

    /// Get the maximum number of agents for a given phase.
    pub fn get_topology_max_agents(phase_name: &str) -> usize {
        match phase_name.to_lowercase().as_str() {
            "planning" => 1,
            "design" => 2,
            "implementation" => 4,
            "review" => 3,
            "ship" => 1,
            _ => 1,
        }
    }

    /// Infer a sub-task description from the main task and agent capabilities.
    fn infer_sub_task(&self, _task_lower: &str, capabilities: &[String]) -> String {
        let cap_map: HashMap<&str, &str> = [
            ("schema", "Database schema design and migration"),
            ("migration", "Database schema design and migration"),
            ("backend", "Backend business logic and services"),
            ("service", "Backend business logic and services"),
            ("frontend", "UI components and screens"),
            ("ui", "UI components and screens"),
            ("component", "UI components and screens"),
            ("api", "API contracts and endpoints"),
            ("endpoint", "API contracts and endpoints"),
            ("test", "Test cases and edge case validation"),
            ("review", "Code review and quality audit"),
            ("security", "Security audit and vulnerability scan"),
            ("performance", "Performance analysis and optimization"),
            ("documentation", "Documentation and inline comments"),
            ("deploy", "Deployment and CI/CD pipeline"),
        ].iter().cloned().collect();

        for cap in capabilities {
            let cap_lower = cap.to_lowercase();
            for (keyword, task_desc) in &cap_map {
                if cap_lower.contains(keyword) {
                    return task_desc.to_string();
                }
            }
        }

        "General implementation".to_string()
    }
}

/// Built-in persona pool (12+ personas) for immediate use.
///
/// In production: these are loaded from YAML files in `personas/` directory.
pub fn default_persona_pool() -> Vec<PersonaCapabilities> {
    vec![
        PersonaCapabilities {
            id: "planning_agent".to_string(),
            name: "Planning Agent".to_string(),
            capabilities: vec![
                "scoping".to_string(),
                "sizing".to_string(),
                "requirement_analysis".to_string(),
                "phase_planning".to_string(),
            ],
            sample_tasks: vec![
                "Analyze feature request".to_string(),
                "Break epic into stories".to_string(),
                "Estimate effort".to_string(),
            ],
            phases_work_on: vec!["planning".to_string()],
            phases_dont_work_on: vec!["implementation".to_string(), "review".to_string(), "ship".to_string()],
            personality_id: "analytical".to_string(),
        },
        PersonaCapabilities {
            id: "system_agent".to_string(),
            name: "Ayan the Architect".to_string(),
            capabilities: vec![
                "architecture_design".to_string(),
                "schema_modeling".to_string(),
                "api_contracts".to_string(),
                "technology_evaluation".to_string(),
                "tradeoff_analysis".to_string(),
            ],
            sample_tasks: vec![
                "Design database schema".to_string(),
                "Create API contract".to_string(),
                "Evaluate tradeoffs".to_string(),
            ],
            phases_work_on: vec!["planning".to_string(), "design".to_string()],
            phases_dont_work_on: vec!["implementation".to_string(), "review".to_string(), "ship".to_string()],
            personality_id: "analytical".to_string(),
        },
        PersonaCapabilities {
            id: "db_architect".to_string(),
            name: "Database Architect".to_string(),
            capabilities: vec![
                "schema_design".to_string(),
                "migration".to_string(),
                "query_optimization".to_string(),
                "database".to_string(),
            ],
            sample_tasks: vec![
                "Design schema for new feature".to_string(),
                "Write migration".to_string(),
                "Optimize query".to_string(),
            ],
            phases_work_on: vec!["design".to_string(), "implementation".to_string()],
            phases_dont_work_on: vec!["review".to_string(), "ship".to_string()],
            personality_id: "precise".to_string(),
        },
        PersonaCapabilities {
            id: "api_designer".to_string(),
            name: "API Designer".to_string(),
            capabilities: vec![
                "api_contracts".to_string(),
                "rest".to_string(),
                "openapi".to_string(),
                "endpoint".to_string(),
            ],
            sample_tasks: vec![
                "Design REST API".to_string(),
                "Create OpenAPI spec".to_string(),
            ],
            phases_work_on: vec!["design".to_string()],
            phases_dont_work_on: vec!["implementation".to_string(), "review".to_string()],
            personality_id: "precise".to_string(),
        },
        PersonaCapabilities {
            id: "coder_a".to_string(),
            name: "Coder A (Backend)".to_string(),
            capabilities: vec![
                "backend".to_string(),
                "business_logic".to_string(),
                "services".to_string(),
                "data_layer".to_string(),
            ],
            sample_tasks: vec![
                "Write business logic".to_string(),
                "Implement service".to_string(),
                "Create data layer".to_string(),
            ],
            phases_work_on: vec!["implementation".to_string()],
            phases_dont_work_on: vec!["planning".to_string(), "design".to_string(), "review".to_string()],
            personality_id: "builder".to_string(),
        },
        PersonaCapabilities {
            id: "coder_b".to_string(),
            name: "Coder B (Frontend)".to_string(),
            capabilities: vec![
                "frontend".to_string(),
                "ui".to_string(),
                "components".to_string(),
                "react_native".to_string(),
                "hooks".to_string(),
            ],
            sample_tasks: vec![
                "Build UI component".to_string(),
                "Create screen".to_string(),
                "Implement hook".to_string(),
            ],
            phases_work_on: vec!["implementation".to_string()],
            phases_dont_work_on: vec!["planning".to_string(), "design".to_string(), "review".to_string()],
            personality_id: "builder".to_string(),
        },
        PersonaCapabilities {
            id: "security_reviewer".to_string(),
            name: "Security Reviewer".to_string(),
            capabilities: vec![
                "security".to_string(),
                "auth".to_string(),
                "injection".to_string(),
                "secrets".to_string(),
                "owasp".to_string(),
            ],
            sample_tasks: vec![
                "Audit auth flow".to_string(),
                "Check for injection".to_string(),
                "Review secrets handling".to_string(),
            ],
            phases_work_on: vec!["review".to_string()],
            phases_dont_work_on: vec!["planning".to_string(), "design".to_string(), "implementation".to_string(), "ship".to_string()],
            personality_id: "critical".to_string(),
        },
        PersonaCapabilities {
            id: "performance_auditor".to_string(),
            name: "Performance Auditor".to_string(),
            capabilities: vec![
                "performance".to_string(),
                "optimization".to_string(),
                "n_plus_one".to_string(),
                "query".to_string(),
                "bundle_size".to_string(),
            ],
            sample_tasks: vec![
                "Check for N+1".to_string(),
                "Optimize query".to_string(),
                "Analyze bundle size".to_string(),
            ],
            phases_work_on: vec!["review".to_string()],
            phases_dont_work_on: vec!["planning".to_string(), "design".to_string(), "implementation".to_string(), "ship".to_string()],
            personality_id: "critical".to_string(),
        },
        PersonaCapabilities {
            id: "test_engineer".to_string(),
            name: "Test Engineer".to_string(),
            capabilities: vec![
                "testing".to_string(),
                "unit_tests".to_string(),
                "integration_tests".to_string(),
                "edge_cases".to_string(),
            ],
            sample_tasks: vec![
                "Write unit tests".to_string(),
                "Add integration tests".to_string(),
                "Test edge cases".to_string(),
            ],
            phases_work_on: vec!["implementation".to_string(), "review".to_string()],
            phases_dont_work_on: vec!["planning".to_string(), "design".to_string(), "ship".to_string()],
            personality_id: "thorough".to_string(),
        },
        PersonaCapabilities {
            id: "devops".to_string(),
            name: "DevOps Engineer".to_string(),
            capabilities: vec![
                "docker".to_string(),
                "ci_cd".to_string(),
                "deployment".to_string(),
                "github_actions".to_string(),
            ],
            sample_tasks: vec![
                "Create Dockerfile".to_string(),
                "Setup CI/CD".to_string(),
                "Deploy to staging".to_string(),
            ],
            phases_work_on: vec!["ship".to_string()],
            phases_dont_work_on: vec!["planning".to_string(), "design".to_string(), "implementation".to_string(), "review".to_string()],
            personality_id: "systematic".to_string(),
        },
        PersonaCapabilities {
            id: "integration_specialist".to_string(),
            name: "Integration Specialist".to_string(),
            capabilities: vec![
                "integration".to_string(),
                "third_party_api".to_string(),
                "webhooks".to_string(),
                "adapters".to_string(),
            ],
            sample_tasks: vec![
                "Integrate third-party API".to_string(),
                "Setup webhook handler".to_string(),
                "Create adapter".to_string(),
            ],
            phases_work_on: vec!["implementation".to_string()],
            phases_dont_work_on: vec!["planning".to_string(), "design".to_string(), "review".to_string()],
            personality_id: "adapter".to_string(),
        },
        PersonaCapabilities {
            id: "documentation_writer".to_string(),
            name: "Documentation Writer".to_string(),
            capabilities: vec![
                "documentation".to_string(),
                "readme".to_string(),
                "api_docs".to_string(),
                "inline_docs".to_string(),
            ],
            sample_tasks: vec![
                "Write README".to_string(),
                "Document API".to_string(),
                "Add inline comments".to_string(),
            ],
            phases_work_on: vec!["implementation".to_string(), "ship".to_string()],
            phases_dont_work_on: vec!["planning".to_string(), "design".to_string(), "review".to_string()],
            personality_id: "clear".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_selection() {
        let selector = AgentSelector::new();
        let personas = default_persona_pool();

        // Test: DB schema design → should match System Agent + DB Architect
        let agents = selector.select_agents(
            "Design database schema for Day Seal feature",
            "design",
            &personas,
        ).unwrap();

        assert!(!agents.is_empty());
        // System Agent or DB Architect should be top
        let top = &agents[0];
        assert!(top.reason.contains("schema") || top.reason.contains("architecture"));
    }

    #[test]
    fn test_topology_limits() {
        assert_eq!(AgentSelector::get_topology_max_agents("planning"), 1);
        assert_eq!(AgentSelector::get_topology_max_agents("design"), 2);
        assert_eq!(AgentSelector::get_topology_max_agents("implementation"), 4);
        assert_eq!(AgentSelector::get_topology_max_agents("review"), 3);
        assert_eq!(AgentSelector::get_topology_max_agents("ship"), 1);
    }
}
