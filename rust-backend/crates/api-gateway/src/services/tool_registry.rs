//! Tool Registry Service
//!
//! Centralized registry for A2A Protocol tools.
//! Provides tool definitions, costs, and validation in a single place.
//!
//! ## Tool Tiers
//!
//! - **Tier 0** (Raw data): Direct database queries, lowest cost
//! - **Tier 1** (Aggregated): Computed aggregations, moderate cost
//! - **Tier 2** (Analysis): Complex analysis, higher cost (future)
//! - **Tier 3** (AI-powered): LLM-enhanced analysis, highest cost

use std::collections::HashMap;
use std::sync::LazyLock;

/// Tool tier levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolTier {
    Tier0, // Raw data
    Tier1, // Aggregated
    Tier2, // Analysis (future)
    Tier3, // AI-powered
}

impl ToolTier {
    /// Get tier number for sorting/comparison
    pub fn level(&self) -> u8 {
        match self {
            ToolTier::Tier0 => 0,
            ToolTier::Tier1 => 1,
            ToolTier::Tier2 => 2,
            ToolTier::Tier3 => 3,
        }
    }

    /// Get human-readable tier name
    pub fn name(&self) -> &'static str {
        match self {
            ToolTier::Tier0 => "Raw Data",
            ToolTier::Tier1 => "Aggregated",
            ToolTier::Tier2 => "Analysis",
            ToolTier::Tier3 => "AI-Powered",
        }
    }
}

/// Tool definition
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    /// Tool name (e.g., "getMyFeedbacks")
    pub name: &'static str,
    /// Tool tier
    pub tier: ToolTier,
    /// Cost in micro-USDC (1 USDC = 1,000,000 micro-USDC)
    pub cost_micro_usdc: i64,
    /// Human-readable cost (e.g., "0.001 USDC")
    pub cost_display: &'static str,
    /// Tool description
    pub description: &'static str,
    /// Whether the tool is enabled
    pub enabled: bool,
}

/// Static tool registry
static TOOL_REGISTRY: LazyLock<HashMap<&'static str, ToolDefinition>> = LazyLock::new(|| {
    let mut tools = HashMap::new();

    // Tier 0: Raw data queries
    tools.insert(
        "getMyFeedbacks",
        ToolDefinition {
            name: "getMyFeedbacks",
            tier: ToolTier::Tier0,
            cost_micro_usdc: 1_000, // 0.001 USDC
            cost_display: "0.001 USDC",
            description: "Get feedback records for an agent",
            enabled: true,
        },
    );

    tools.insert(
        "getAgentProfile",
        ToolDefinition {
            name: "getAgentProfile",
            tier: ToolTier::Tier0,
            cost_micro_usdc: 1_000, // 0.001 USDC
            cost_display: "0.001 USDC",
            description: "Get agent profile and metadata",
            enabled: true,
        },
    );

    // Tier 1: Aggregated queries
    tools.insert(
        "getReputationSummary",
        ToolDefinition {
            name: "getReputationSummary",
            tier: ToolTier::Tier1,
            cost_micro_usdc: 10_000, // 0.01 USDC
            cost_display: "0.01 USDC",
            description: "Get aggregated reputation statistics",
            enabled: true,
        },
    );

    tools.insert(
        "getTrend",
        ToolDefinition {
            name: "getTrend",
            tier: ToolTier::Tier1,
            cost_micro_usdc: 10_000, // 0.01 USDC
            cost_display: "0.01 USDC",
            description: "Get reputation trend over time",
            enabled: true,
        },
    );

    tools.insert(
        "getValidationHistory",
        ToolDefinition {
            name: "getValidationHistory",
            tier: ToolTier::Tier1,
            cost_micro_usdc: 10_000, // 0.01 USDC
            cost_display: "0.01 USDC",
            description: "Get validation history for an agent",
            enabled: true,
        },
    );

    // Tier 3: AI-powered (stub for now)
    tools.insert(
        "getReputationReport",
        ToolDefinition {
            name: "getReputationReport",
            tier: ToolTier::Tier3,
            cost_micro_usdc: 200_000, // 0.20 USDC
            cost_display: "0.20 USDC",
            description: "Get AI-powered reputation analysis report",
            enabled: true, // Enabled but returns stub response
        },
    );

    tools
});

/// Tool Registry
pub struct ToolRegistry;

impl ToolRegistry {
    /// Check if a tool exists and is enabled
    pub fn is_valid(tool: &str) -> bool {
        TOOL_REGISTRY.get(tool).map(|t| t.enabled).unwrap_or(false)
    }

    /// Get tool definition
    pub fn get(tool: &str) -> Option<&'static ToolDefinition> {
        TOOL_REGISTRY.get(tool)
    }

    /// Get cost in micro-USDC for a tool
    pub fn get_cost_micro_usdc(tool: &str) -> i64 {
        TOOL_REGISTRY
            .get(tool)
            .map(|t| t.cost_micro_usdc)
            .unwrap_or(0)
    }

    /// Get human-readable cost string
    pub fn get_cost_display(tool: &str) -> Option<&'static str> {
        TOOL_REGISTRY.get(tool).map(|t| t.cost_display)
    }

    /// Get tool tier
    pub fn get_tier(tool: &str) -> Option<ToolTier> {
        TOOL_REGISTRY.get(tool).map(|t| t.tier)
    }

    /// List all enabled tools
    pub fn list_enabled() -> Vec<&'static ToolDefinition> {
        TOOL_REGISTRY.values().filter(|t| t.enabled).collect()
    }

    /// List tools by tier
    pub fn list_by_tier(tier: ToolTier) -> Vec<&'static ToolDefinition> {
        TOOL_REGISTRY
            .values()
            .filter(|t| t.enabled && t.tier == tier)
            .collect()
    }

    /// Get all tool names (for validation error messages)
    pub fn tool_names() -> Vec<&'static str> {
        TOOL_REGISTRY
            .values()
            .filter(|t| t.enabled)
            .map(|t| t.name)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry_is_valid() {
        assert!(ToolRegistry::is_valid("getMyFeedbacks"));
        assert!(ToolRegistry::is_valid("getAgentProfile"));
        assert!(ToolRegistry::is_valid("getReputationSummary"));
        assert!(ToolRegistry::is_valid("getTrend"));
        assert!(ToolRegistry::is_valid("getValidationHistory"));
        assert!(ToolRegistry::is_valid("getReputationReport"));
        assert!(!ToolRegistry::is_valid("unknownTool"));
        assert!(!ToolRegistry::is_valid(""));
    }

    #[test]
    fn test_tool_registry_get_cost() {
        // Tier 0: 0.001 USDC = 1,000 micro-USDC
        assert_eq!(ToolRegistry::get_cost_micro_usdc("getMyFeedbacks"), 1_000);
        assert_eq!(ToolRegistry::get_cost_micro_usdc("getAgentProfile"), 1_000);

        // Tier 1: 0.01 USDC = 10,000 micro-USDC
        assert_eq!(
            ToolRegistry::get_cost_micro_usdc("getReputationSummary"),
            10_000
        );
        assert_eq!(ToolRegistry::get_cost_micro_usdc("getTrend"), 10_000);
        assert_eq!(
            ToolRegistry::get_cost_micro_usdc("getValidationHistory"),
            10_000
        );

        // Tier 3: 0.20 USDC = 200,000 micro-USDC
        assert_eq!(
            ToolRegistry::get_cost_micro_usdc("getReputationReport"),
            200_000
        );

        // Unknown returns 0
        assert_eq!(ToolRegistry::get_cost_micro_usdc("unknownTool"), 0);
    }

    #[test]
    fn test_tool_registry_get_tier() {
        assert_eq!(
            ToolRegistry::get_tier("getMyFeedbacks"),
            Some(ToolTier::Tier0)
        );
        assert_eq!(
            ToolRegistry::get_tier("getReputationSummary"),
            Some(ToolTier::Tier1)
        );
        assert_eq!(
            ToolRegistry::get_tier("getReputationReport"),
            Some(ToolTier::Tier3)
        );
        assert_eq!(ToolRegistry::get_tier("unknownTool"), None);
    }

    #[test]
    fn test_tool_tier_levels() {
        assert_eq!(ToolTier::Tier0.level(), 0);
        assert_eq!(ToolTier::Tier1.level(), 1);
        assert_eq!(ToolTier::Tier2.level(), 2);
        assert_eq!(ToolTier::Tier3.level(), 3);
    }

    #[test]
    fn test_list_enabled_tools() {
        let tools = ToolRegistry::list_enabled();
        assert!(!tools.is_empty());
        assert!(tools.iter().all(|t| t.enabled));
    }

    #[test]
    fn test_list_by_tier() {
        let tier0_tools = ToolRegistry::list_by_tier(ToolTier::Tier0);
        assert_eq!(tier0_tools.len(), 2); // getMyFeedbacks, getAgentProfile

        let tier1_tools = ToolRegistry::list_by_tier(ToolTier::Tier1);
        assert_eq!(tier1_tools.len(), 3); // getReputationSummary, getTrend, getValidationHistory

        let tier3_tools = ToolRegistry::list_by_tier(ToolTier::Tier3);
        assert_eq!(tier3_tools.len(), 1); // getReputationReport
    }

    #[test]
    fn test_cost_display() {
        assert_eq!(
            ToolRegistry::get_cost_display("getMyFeedbacks"),
            Some("0.001 USDC")
        );
        assert_eq!(
            ToolRegistry::get_cost_display("getReputationSummary"),
            Some("0.01 USDC")
        );
        assert_eq!(
            ToolRegistry::get_cost_display("getReputationReport"),
            Some("0.20 USDC")
        );
        assert_eq!(ToolRegistry::get_cost_display("unknownTool"), None);
    }
}
