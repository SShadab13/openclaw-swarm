use anyhow::Result;

/// ContextCompiler generates the 5-Section YAML header that every agent
/// receives at every phase. Implements the "Tiered Phase Briefing" design.
///
/// Based on: Google ADK Context Engineering + Circle of Competence mental model
pub struct ContextCompiler;

/// Compiled context briefing for an agent.
#[derive(Debug, Clone)]
pub struct AgentContext {
    pub goal: GoalSection,
    pub circle_of_competence: CircleOfCompetenceSection,
    pub role: RoleSection,
    pub expectations: ExpectationsSection,
    pub relevant_history: Vec<HistoryEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct GoalSection {
    pub epic: String,
    pub story: String,
    pub task: String,
    pub success_criteria: String,
}

#[derive(Debug, Clone, Default)]
pub struct CircleOfCompetenceSection {
    pub knows: Vec<String>,
    pub doesnt_know: Vec<String>,
    pub will_ask: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RoleSection {
    pub phase: String,
    pub topology: String,
    pub peers: Vec<PeerInfo>,
    pub reports_to: String,
}

#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub persona_id: String,
    pub doing: String,
}

#[derive(Debug, Clone, Default)]
pub struct ExpectationsSection {
    pub output: String,
    pub tests: String,
    pub documentation: String,
    pub commit_message: String,
    pub naming_conventions: NamingConventions,
    pub undefined_items_policy: String,
}

#[derive(Debug, Clone, Default)]
pub struct NamingConventions {
    pub functions: String,
    pub tables: String,
    pub components: String,
    pub variables: String,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub phase: String,
    pub artifact: String,
    pub summary: String,
}

impl ContextCompiler {
    pub fn new() -> Self {
        Self
    }

    /// Compile a full context briefing for an agent.
    pub fn compile(
        &self,
        epic_name: &str,
        story_name: &str,
        task: &str,
        success_criteria: &str,
        phase_name: &str,
        topology: &str,
        peers: &[PeerInfo],
        reports_to: &str,
        naming: &NamingConventions,
        history: &[HistoryEntry],
        agent_capabilities: &[String],
        all_peer_capabilities: &[(String, Vec<String>)], // (persona_id, capabilities)
    ) -> Result<AgentContext> {
        // Build Circle of Competence
        let mut knows = Vec::new();
        let mut doesnt_know = Vec::new();
        let will_ask = vec![
            "Clarify ambiguous requirements before coding".to_string(),
            "Flag if success criteria are underspecified".to_string(),
            "Request review if solution exceeds 100 lines".to_string(),
            "Ask leader before defining new names/signature".to_string(),
        ];

        // What this agent knows (from its capabilities)
        for cap in agent_capabilities {
            knows.push(format!("{}", cap));
        }

        // What other agents handle (cross-reference peer capabilities)
        for (peer_id, peer_caps) in all_peer_capabilities {
            if peer_id != &peers.first().map(|p| p.persona_id.clone()).unwrap_or_default() {
                for cap in peer_caps {
                    doesnt_know.push(format!("{} (handled by {})", cap, peer_id));
                }
            }
        }

        // Default expectations
        let expectations = ExpectationsSection {
            output: format!("{}", task),
            tests: "At least 3 edge cases tested".to_string(),
            documentation: "JSDoc with params/returns".to_string(),
            commit_message: format!("feat: {}", story_name.to_lowercase()),
            naming_conventions: naming.clone(),
            undefined_items_policy: "ASK_LEADER_BEFORE_DEFINING".to_string(),
        };

        Ok(AgentContext {
            goal: GoalSection {
                epic: epic_name.to_string(),
                story: story_name.to_string(),
                task: task.to_string(),
                success_criteria: success_criteria.to_string(),
            },
            circle_of_competence: CircleOfCompetenceSection {
                knows,
                doesnt_know,
                will_ask,
            },
            role: RoleSection {
                phase: phase_name.to_string(),
                topology: topology.to_string(),
                peers: peers.to_vec(),
                reports_to: reports_to.to_string(),
            },
            expectations,
            relevant_history: history.to_vec(),
        })
    }

    /// Render the context as a YAML string (for agent prompt prepending).
    pub fn render_yaml(&self, ctx: &AgentContext) -> String {
        let mut yaml = String::new();

        yaml.push_str("---\n");
        yaml.push_str("# Agent Context Briefing\n");
        yaml.push_str("# This header is prepended to your task prompt.\n");
        yaml.push_str("# Follow these constraints. Ask if anything is ambiguous.\n");
        yaml.push_str("---\n\n");

        // Section 1: Goal
        yaml.push_str("agent_context:\n");
        yaml.push_str("  goal:\n");
        yaml.push_str(&format!("    epic: \"{}\"\n", escape_yaml(&ctx.goal.epic)));
        yaml.push_str(&format!("    story: \"{}\"\n", escape_yaml(&ctx.goal.story)));
        yaml.push_str(&format!("    task: \"{}\"\n", escape_yaml(&ctx.goal.task)));
        yaml.push_str(&format!("    success_criteria: \"{}\"\n", escape_yaml(&ctx.goal.success_criteria)));
        yaml.push('\n');

        // Section 2: Circle of Competence
        yaml.push_str("  circle_of_competence:\n");
        yaml.push_str("    knows:\n");
        for item in &ctx.circle_of_competence.knows {
            yaml.push_str(&format!("      - \"{}\"\n", escape_yaml(item)));
        }
        yaml.push_str("    doesnt_know:\n");
        for item in &ctx.circle_of_competence.doesnt_know {
            yaml.push_str(&format!("      - \"{}\"\n", escape_yaml(item)));
        }
        yaml.push_str("    will_ask:\n");
        for item in &ctx.circle_of_competence.will_ask {
            yaml.push_str(&format!("      - \"{}\"\n", escape_yaml(item)));
        }
        yaml.push('\n');

        // Section 3: Role
        yaml.push_str("  role_in_this_phase:\n");
        yaml.push_str(&format!("    phase: \"{}\"\n", escape_yaml(&ctx.role.phase)));
        yaml.push_str(&format!("    topology: \"{}\"\n", escape_yaml(&ctx.role.topology)));
        yaml.push_str("    peers:\n");
        for peer in &ctx.role.peers {
            yaml.push_str(&format!("      - persona: \"{}\"\n", escape_yaml(&peer.persona_id)));
            yaml.push_str(&format!("        doing: \"{}\"\n", escape_yaml(&peer.doing)));
        }
        yaml.push_str(&format!("    reports_to: \"{}\"\n", escape_yaml(&ctx.role.reports_to)));
        yaml.push('\n');

        // Section 4: Expectations
        yaml.push_str("  expectations:\n");
        yaml.push_str(&format!("    output: \"{}\"\n", escape_yaml(&ctx.expectations.output)));
        yaml.push_str(&format!("    tests: \"{}\"\n", escape_yaml(&ctx.expectations.tests)));
        yaml.push_str(&format!("    documentation: \"{}\"\n", escape_yaml(&ctx.expectations.documentation)));
        yaml.push_str(&format!("    commit_message: \"{}\"\n", escape_yaml(&ctx.expectations.commit_message)));
        yaml.push_str("    naming_conventions:\n");
        yaml.push_str(&format!("      functions: \"{}\"\n", escape_yaml(&ctx.expectations.naming_conventions.functions)));
        yaml.push_str(&format!("      tables: \"{}\"\n", escape_yaml(&ctx.expectations.naming_conventions.tables)));
        yaml.push_str(&format!("      components: \"{}\"\n", escape_yaml(&ctx.expectations.naming_conventions.components)));
        yaml.push_str(&format!("      variables: \"{}\"\n", escape_yaml(&ctx.expectations.naming_conventions.variables)));
        yaml.push_str(&format!("    undefined_items_policy: \"{}\"\n", escape_yaml(&ctx.expectations.undefined_items_policy)));
        yaml.push('\n');

        // Section 5: Relevant History
        yaml.push_str("  relevant_history:\n");
        for entry in &ctx.relevant_history {
            yaml.push_str(&format!("    - phase: \"{}\"\n", escape_yaml(&entry.phase)));
            yaml.push_str(&format!("      artifact: \"{}\"\n", escape_yaml(&entry.artifact)));
            yaml.push_str(&format!("      summary: \"{}\"\n", escape_yaml(&entry.summary)));
        }
        yaml.push('\n');

        yaml.push_str("# END CONTEXT BRIEFING\n");
        yaml.push_str("# Proceed with your assigned task below.\n");
        yaml.push_str("---\n\n");

        yaml
    }
}

fn escape_yaml(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_context() {
        let compiler = ContextCompiler::new();
        let ctx = compiler.compile(
            "Social Tab Rebuild",
            "Implement Day Seal",
            "Write calculateDaySeal() in services/xp.ts",
            "Function detects completion, awards 50 XP, updates streak",
            "implementation",
            "parallel",
            &[PeerInfo { persona_id: "coder_a".to_string(), doing: "DB migration".to_string() }],
            "coordinator",
            &NamingConventions {
                functions: "camelCase".to_string(),
                tables: "snake_case".to_string(),
                components: "PascalCase".to_string(),
                variables: "camelCase".to_string(),
            },
            &[HistoryEntry {
                phase: "Design".to_string(),
                artifact: "designs/day-seal-design.md".to_string(),
                summary: "calculateDaySeal() returns boolean".to_string(),
            }],
            &["business_logic".to_string(), "services".to_string()],
            &[("coder_a".to_string(), vec!["schema".to_string(), "migration".to_string()])],
        ).unwrap();

        assert_eq!(ctx.goal.epic, "Social Tab Rebuild");
        assert_eq!(ctx.role.phase, "implementation");
        assert_eq!(ctx.expectations.undefined_items_policy, "ASK_LEADER_BEFORE_DEFINING");
        assert!(!ctx.circle_of_competence.will_ask.is_empty());
    }

    #[test]
    fn test_render_yaml() {
        let compiler = ContextCompiler::new();
        let ctx = compiler.compile(
            "Test Epic", "Test Story", "Test task", "Criteria",
            "planning", "sequential",
            &[], "queen",
            &NamingConventions::default(),
            &[],
            &[],
            &[],
        ).unwrap();

        let yaml = compiler.render_yaml(&ctx);
        assert!(yaml.contains("agent_context:"));
        assert!(yaml.contains("goal:"));
        assert!(yaml.contains("circle_of_competence:"));
        assert!(yaml.contains("role_in_this_phase:"));
        assert!(yaml.contains("expectations:"));
        assert!(yaml.contains("relevant_history:"));
        assert!(yaml.contains("ASK_LEADER_BEFORE_DEFINING"));
    }
}