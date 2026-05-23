/// TopologySelector determines execution topology per phase.
///
/// Not all phases should use parallel agents:
/// - Planning: Sequential (1 agent) — coherent thought chain
/// - Design: Sequential → Parallel (2 agents debate) — architecture exploration
/// - Implementation: Parallel (N agents) — independent files/modules
/// - Review: Parallel (3 agents) — independent review dimensions
/// - Ship: Sequential (Queen) — single merge point
pub struct TopologySelector;

#[derive(Debug, Clone, PartialEq)]
pub enum Topology {
    Sequential,
    Parallel,
    Hybrid,
}

impl TopologySelector {
    pub fn new() -> Self {
        Self
    }

    /// Select the best topology for a given phase.
    pub fn select(phase_name: &str) -> Topology {
        match phase_name.to_lowercase().as_str() {
            "planning" => Topology::Sequential,
            "design" => Topology::Hybrid,     // Sequential debate → parallel exploration
            "implementation" => Topology::Parallel,
            "review" => Topology::Parallel,
            "ship" => Topology::Sequential,
            _ => Topology::Sequential,
        }
    }

    /// Get the recommended agent count for a topology + phase combination.
    pub fn agent_count(phase_name: &str) -> usize {
        match phase_name.to_lowercase().as_str() {
            "planning" => 1,
            "design" => 2,
            "implementation" => 4,
            "review" => 3,
            "ship" => 1,
            _ => 1,
        }
    }

    /// Describe why this topology was chosen (for UI display).
    pub fn rationale(phase_name: &str) -> &'static str {
        match phase_name.to_lowercase().as_str() {
            "planning" => "Sequential: Planning needs a coherent thought chain. One agent scopes, defines phases, and creates the plan.",
            "design" => "Hybrid: Architecture benefits from parallel exploration (2 agents debate approaches), then sequential synthesis.",
            "implementation" => "Parallel: Independent files/modules can be built simultaneously by different agents.",
            "review" => "Parallel: Independent review dimensions (simplicity, bugs, conventions) checked simultaneously.",
            "ship" => "Sequential: Single merge point. Queen coordinates final integration.",
            _ => "Sequential: Default conservative topology.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topology_selection() {
        assert_eq!(TopologySelector::select("planning"), Topology::Sequential);
        assert_eq!(TopologySelector::select("design"), Topology::Hybrid);
        assert_eq!(TopologySelector::select("implementation"), Topology::Parallel);
        assert_eq!(TopologySelector::select("review"), Topology::Parallel);
        assert_eq!(TopologySelector::select("ship"), Topology::Sequential);
    }

    #[test]
    fn test_agent_counts() {
        assert_eq!(TopologySelector::agent_count("planning"), 1);
        assert_eq!(TopologySelector::agent_count("design"), 2);
        assert_eq!(TopologySelector::agent_count("implementation"), 4);
        assert_eq!(TopologySelector::agent_count("review"), 3);
        assert_eq!(TopologySelector::agent_count("ship"), 1);
    }
}