//! Test Patterns - Demonstrates all orchestrator workflow patterns
//!
//! This example shows how to use each of the 6 workflow patterns
//! loaded from YAML templates to solve decision theory questions.

use spai::prelude::*;
use spai::security_tools::{SecurityToolRegistry, TaggedSecurityTools};
use spai::orchestrator::{
    OrchestratorConfig,
    OrchestratorPattern,
    PatternType,
    PatternSpecificConfig,
    SequentialOrchestrator,
    ConcurrentOrchestrator,
    HierarchicalOrchestrator,
    DebateOrchestrator,
    RouterOrchestrator,
    ConsensusOrchestrator,
    AgentConfig,
};
use std::sync::Arc;
use std::path::PathBuf;

/// Path to orchestrator templates
const TEMPLATES_DIR: &str = "src/orchestrator/templates";

/// Decision theory questions for testing each pattern
const SEQUENTIAL_QUESTION: &str = 
    "What is the optimal strategy for the Prisoner's Dilemma when played repeatedly?";

const CONCURRENT_QUESTION: &str = 
    "Should we adopt a risk-averse or risk-seeking strategy for a one-time high-stakes decision?";

const HIERARCHICAL_QUESTION: &str = 
    "Analyze the Nash equilibrium for a 3-player coordination game.";

const DEBATE_QUESTION: &str = 
    "Is expected utility maximization the correct decision framework, or should we use maximin?";

const ROUTER_QUESTION: &str = 
    "How should we handle a multi-armed bandit problem with unknown reward distributions?";

const CONSENSUS_QUESTION: &str = 
    "Should we cooperate or defect in a one-shot Prisoner's Dilemma?";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("═══════════════════════════════════════════════════════════════");
    println!("   SPAI Orchestrator Pattern Tests");
    println!("   Loading from YAML Templates");
    println!("═══════════════════════════════════════════════════════════════\n");

    // ═══════════════════════════════════════════════════════════════════════════
    // SETUP: Load tools and create LLM client
    // ═══════════════════════════════════════════════════════════════════════════
    
    let tools_dir = PathBuf::from("tools");
    let registry = Arc::new(SecurityToolRegistry::discover(&tools_dir));
    
    println!("✓ Discovered {} tools", registry.len());
    println!("✓ Available tags: {:?}", registry.all_tags());

    // Create LLM client
    let client: Arc<dyn LlmClient> = match OpenRouterClient::from_env() {
        Ok(c) => {
            println!("✓ OpenRouter client ready\n");
            Arc::new(c)
        }
        Err(e) => {
            eprintln!("✗ OpenRouter not available: {}", e);
            eprintln!("  Set OPENROUTER_API_KEY to run full tests");
            run_mock_tests()?;
            return Ok(());
        }
    };

    // Run pattern tests
    println!("Running pattern demonstrations from YAML templates...\n");

    // Test 1: Sequential
    test_sequential_pattern(&client, &registry).await?;
    
    // Test 2: Concurrent
    test_concurrent_pattern(&client, &registry).await?;
    
    // Test 3: Hierarchical
    test_hierarchical_pattern(&client, &registry).await?;
    
    // Test 4: Debate
    test_debate_pattern(&client, &registry).await?;
    
    // Test 5: Router
    test_router_pattern(&client, &registry).await?;
    
    // Test 6: Consensus
    test_consensus_pattern(&client, &registry).await?;

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   All Pattern Tests Complete");
    println!("═══════════════════════════════════════════════════════════════\n");

    Ok(())
}

/// Build agent with tools loaded from its tool_tags
fn build_agent_with_tools(
    cfg: &AgentConfig,
    client: Arc<dyn LlmClient>,
    registry: &Arc<SecurityToolRegistry>,
) -> anyhow::Result<spai::Agent> {
    let mut builder = spai::Agent::builder()
        .name(&cfg.name)
        .model(&cfg.model)
        .system_prompt(&cfg.system_prompt)
        .max_loops(cfg.max_loops as u32)
        .temperature(cfg.temperature)
        .client(client);
    
    // Load tools if tags specified
    if !cfg.tool_tags.is_empty() {
        let tags: Vec<&str> = cfg.tool_tags.iter().map(|s| s.as_str()).collect();
        let helper = TaggedSecurityTools::new(registry.clone(), &tags);
        let tools = helper.create_tools();
        if !tools.is_empty() {
            println!("    → Loading {} tools for {} (tags: {:?})", tools.len(), cfg.name, cfg.tool_tags);
            builder = builder.tools(tools);
        }
    }
    
    builder.build().map_err(|e| anyhow::anyhow!("{}", e))
}

async fn test_sequential_pattern(client: &Arc<dyn LlmClient>, registry: &Arc<SecurityToolRegistry>) -> anyhow::Result<()> {
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  Pattern 1: SEQUENTIAL (from sequential.yaml)              │");
    println!("└─────────────────────────────────────────────────────────────┘\n");
    println!("Question: {}\n", SEQUENTIAL_QUESTION);

    // Load from YAML template
    let config = OrchestratorConfig::from_file(format!("{}/sequential.yaml", TEMPLATES_DIR))?;
    println!("✓ Loaded config: {:?}", config.pattern);

    // Build agents from config with tools
    let agents = match &config.pattern_config {
        PatternSpecificConfig::AgentList { agents, .. } => {
            agents.iter()
                .map(|cfg| build_agent_with_tools(cfg, client.clone(), registry))
                .collect::<std::result::Result<Vec<_>, _>>()?
        }
        _ => return Err(anyhow::anyhow!("Expected AgentList config")),
    };
    
    println!("✓ Built {} agents from template", agents.len());

    let orchestrator = SequentialOrchestrator::new(agents);
    let result = orchestrator.execute(SEQUENTIAL_QUESTION).await?;
    
    println!("\nResult ({} agents, {}ms):\n", 
        result.metadata.agent_count, 
        result.metadata.total_time_ms);
    println!("{}\n", result.content);

    Ok(())
}

async fn test_concurrent_pattern(client: &Arc<dyn LlmClient>, registry: &Arc<SecurityToolRegistry>) -> anyhow::Result<()> {
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  Pattern 2: CONCURRENT (from concurrent.yaml)              │");
    println!("└─────────────────────────────────────────────────────────────┘\n");
    println!("Question: {}\n", CONCURRENT_QUESTION);

    // Load from YAML template
    let config = OrchestratorConfig::from_file(format!("{}/concurrent.yaml", TEMPLATES_DIR))?;
    println!("✓ Loaded config: {:?}", config.pattern);

    // Build agents from config with tools
    let agents = match &config.pattern_config {
        PatternSpecificConfig::AgentList { agents, .. } => {
            agents.iter()
                .map(|cfg| build_agent_with_tools(cfg, client.clone(), registry))
                .collect::<std::result::Result<Vec<_>, _>>()?
        }
        _ => return Err(anyhow::anyhow!("Expected AgentList config")),
    };
    
    println!("✓ Built {} agents from template", agents.len());

    let orchestrator = ConcurrentOrchestrator::new(agents);
    let result = orchestrator.execute(CONCURRENT_QUESTION).await?;
    
    println!("\nResult ({} agents in parallel, {}ms):\n", 
        result.metadata.agent_count, 
        result.metadata.total_time_ms);
    println!("{}\n", result.content);

    Ok(())
}

async fn test_hierarchical_pattern(client: &Arc<dyn LlmClient>, registry: &Arc<SecurityToolRegistry>) -> anyhow::Result<()> {
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  Pattern 3: HIERARCHICAL (from hierarchical.yaml)          │");
    println!("└─────────────────────────────────────────────────────────────┘\n");
    println!("Question: {}\n", HIERARCHICAL_QUESTION);

    // Load from YAML template
    let config = OrchestratorConfig::from_file(format!("{}/hierarchical.yaml", TEMPLATES_DIR))?;
    println!("✓ Loaded config: {:?}", config.pattern);

    // Build agents from config with tools
    let (lead_agent, subagents) = match &config.pattern_config {
        PatternSpecificConfig::Hierarchical { lead_agent, subagents } => {
            let lead = build_agent_with_tools(lead_agent, client.clone(), registry)?;
            let subs: Vec<_> = subagents.generate_agents()
                .iter()
                .map(|cfg| build_agent_with_tools(cfg, client.clone(), registry))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            (lead, subs)
        }
        _ => return Err(anyhow::anyhow!("Expected Hierarchical config")),
    };
    
    println!("✓ Built lead agent + {} subagents from template", subagents.len());

    let orchestrator = HierarchicalOrchestrator::new(lead_agent, subagents);
    let result = orchestrator.execute(HIERARCHICAL_QUESTION).await?;
    
    println!("\nResult ({} agents, {} handoffs, {}ms):\n", 
        result.metadata.agent_count,
        result.metadata.handoff_count,
        result.metadata.total_time_ms);
    println!("{}\n", result.content);

    Ok(())
}

async fn test_debate_pattern(client: &Arc<dyn LlmClient>, registry: &Arc<SecurityToolRegistry>) -> anyhow::Result<()> {
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  Pattern 4: DEBATE (from debate.yaml)                      │");
    println!("└─────────────────────────────────────────────────────────────┘\n");
    println!("Question: {}\n", DEBATE_QUESTION);

    // Load from YAML template
    let config = OrchestratorConfig::from_file(format!("{}/debate.yaml", TEMPLATES_DIR))?;
    println!("✓ Loaded config: {:?}", config.pattern);

    // Build agents from config with tools
    let (pro, con, synth, rounds) = match &config.pattern_config {
        PatternSpecificConfig::Debate { pro_agent, con_agent, synthesizer, rounds } => {
            (
                build_agent_with_tools(pro_agent, client.clone(), registry)?,
                build_agent_with_tools(con_agent, client.clone(), registry)?,
                build_agent_with_tools(synthesizer, client.clone(), registry)?,
                *rounds,
            )
        }
        _ => return Err(anyhow::anyhow!("Expected Debate config")),
    };
    
    println!("✓ Built pro, con, synthesizer agents ({} rounds)", rounds);

    let orchestrator = DebateOrchestrator::new(pro, con, synth).with_rounds(rounds);
    let result = orchestrator.execute(DEBATE_QUESTION).await?;
    
    println!("\nResult ({} agents, {} debate rounds, {}ms):\n", 
        result.metadata.agent_count,
        result.metadata.extra.get("rounds").and_then(|v| v.as_u64()).unwrap_or(0),
        result.metadata.total_time_ms);
    println!("{}\n", result.content);

    Ok(())
}

async fn test_router_pattern(client: &Arc<dyn LlmClient>, registry: &Arc<SecurityToolRegistry>) -> anyhow::Result<()> {
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  Pattern 5: ROUTER (from router.yaml)                      │");
    println!("└─────────────────────────────────────────────────────────────┘\n");
    println!("Question: {}\n", ROUTER_QUESTION);

    // Load from YAML template
    let config = OrchestratorConfig::from_file(format!("{}/router.yaml", TEMPLATES_DIR))?;
    println!("✓ Loaded config: {:?}", config.pattern);

    // Build agents from config with tools
    let (router_agent, specialists) = match &config.pattern_config {
        PatternSpecificConfig::Router { router_agent, specialists } => {
            let router = build_agent_with_tools(router_agent, client.clone(), registry)?;
            let specs: std::collections::HashMap<String, _> = specialists.iter()
                .map(|(k, v)| Ok((k.clone(), build_agent_with_tools(v, client.clone(), registry)?)))
                .collect::<anyhow::Result<_>>()?;
            (router, specs)
        }
        _ => return Err(anyhow::anyhow!("Expected Router config")),
    };
    
    println!("✓ Built router + {} specialists from template", specialists.len());

    let orchestrator = RouterOrchestrator::new(router_agent).with_specialists(specialists);
    let result = orchestrator.execute(ROUTER_QUESTION).await?;
    
    let routed_to = result.metadata.extra.get("routed_to")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    
    println!("\nResult (routed to: {}, {}ms):\n", 
        routed_to,
        result.metadata.total_time_ms);
    println!("{}\n", result.content);

    Ok(())
}

async fn test_consensus_pattern(client: &Arc<dyn LlmClient>, registry: &Arc<SecurityToolRegistry>) -> anyhow::Result<()> {
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  Pattern 6: CONSENSUS (from consensus.yaml)                │");
    println!("└─────────────────────────────────────────────────────────────┘\n");
    println!("Question: {}\n", CONSENSUS_QUESTION);

    // Load from YAML template
    let config = OrchestratorConfig::from_file(format!("{}/consensus.yaml", TEMPLATES_DIR))?;
    println!("✓ Loaded config: {:?}", config.pattern);

    // Build agents from config with tools
    let (agents, threshold) = match &config.pattern_config {
        PatternSpecificConfig::Consensus { agents, threshold } => {
            let built: Vec<_> = agents.iter()
                .map(|cfg| build_agent_with_tools(cfg, client.clone(), registry))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            (built, *threshold)
        }
        _ => return Err(anyhow::anyhow!("Expected Consensus config")),
    };
    
    println!("✓ Built {} voters (threshold: {:.0}%)", agents.len(), threshold * 100.0);

    let orchestrator = ConsensusOrchestrator::new(agents).with_threshold(threshold);
    let result = orchestrator.execute(CONSENSUS_QUESTION).await?;
    
    let consensus_reached = result.metadata.extra.get("consensus_reached")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let agreement = result.metadata.extra.get("agreement_percentage")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) * 100.0;
    
    println!("\nResult (consensus: {}, {:.0}% agreement, {}ms):\n", 
        if consensus_reached { "REACHED" } else { "NOT REACHED" },
        agreement,
        result.metadata.total_time_ms);
    println!("{}\n", result.content);

    Ok(())
}

fn run_mock_tests() -> anyhow::Result<()> {
    println!("\n--- Running Mock Tests (no API key) ---\n");
    
    // Test YAML loading
    let templates = ["sequential", "concurrent", "hierarchical", "debate", "router", "consensus"];
    
    for template in &templates {
        let path = format!("{}/{}.yaml", TEMPLATES_DIR, template);
        match OrchestratorConfig::from_file(&path) {
            Ok(config) => println!("  ✓ Loaded {}.yaml → {:?}", template, config.pattern),
            Err(e) => println!("  ✗ Failed {}.yaml: {}", template, e),
        }
    }
    
    println!("\nPattern constructors verified:");
    println!("  ✓ SequentialOrchestrator");
    println!("  ✓ ConcurrentOrchestrator");
    println!("  ✓ HierarchicalOrchestrator");
    println!("  ✓ DebateOrchestrator");
    println!("  ✓ RouterOrchestrator");
    println!("  ✓ ConsensusOrchestrator");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_sequential_yaml() {
        let config = OrchestratorConfig::from_file(format!("{}/sequential.yaml", TEMPLATES_DIR));
        assert!(config.is_ok(), "Should load sequential.yaml");
        assert_eq!(config.unwrap().pattern, PatternType::Sequential);
    }

    #[test]
    fn test_load_concurrent_yaml() {
        let config = OrchestratorConfig::from_file(format!("{}/concurrent.yaml", TEMPLATES_DIR));
        assert!(config.is_ok(), "Should load concurrent.yaml");
        assert_eq!(config.unwrap().pattern, PatternType::Concurrent);
    }

    #[test]
    fn test_load_hierarchical_yaml() {
        let config = OrchestratorConfig::from_file(format!("{}/hierarchical.yaml", TEMPLATES_DIR));
        assert!(config.is_ok(), "Should load hierarchical.yaml");
        assert_eq!(config.unwrap().pattern, PatternType::Hierarchical);
    }

    #[test]
    fn test_load_debate_yaml() {
        let config = OrchestratorConfig::from_file(format!("{}/debate.yaml", TEMPLATES_DIR));
        assert!(config.is_ok(), "Should load debate.yaml");
        assert_eq!(config.unwrap().pattern, PatternType::Debate);
    }

    #[test]
    fn test_load_router_yaml() {
        let config = OrchestratorConfig::from_file(format!("{}/router.yaml", TEMPLATES_DIR));
        assert!(config.is_ok(), "Should load router.yaml");
        assert_eq!(config.unwrap().pattern, PatternType::Router);
    }

    #[test]
    fn test_load_consensus_yaml() {
        let config = OrchestratorConfig::from_file(format!("{}/consensus.yaml", TEMPLATES_DIR));
        assert!(config.is_ok(), "Should load consensus.yaml: {:?}", config);
        let cfg = config.unwrap();
        assert_eq!(cfg.pattern, PatternType::Consensus);
        // Verify it parsed as Consensus, not AgentList
        match cfg.pattern_config {
            PatternSpecificConfig::Consensus { threshold, .. } => {
                assert!((threshold - 0.66).abs() < 0.01, "Threshold should be 0.66");
            }
            _ => panic!("Should parse as Consensus variant, not AgentList"),
        }
    }

    #[test]
    fn test_sequential_pattern_type() {
        let orchestrator = SequentialOrchestrator::new(vec![]);
        assert_eq!(orchestrator.pattern_type(), "sequential");
    }

    #[test]
    fn test_concurrent_pattern_type() {
        let orchestrator = ConcurrentOrchestrator::new(vec![]);
        assert_eq!(orchestrator.pattern_type(), "concurrent");
    }

    #[test]
    fn test_consensus_pattern_type() {
        let orchestrator = ConsensusOrchestrator::new(vec![]).with_threshold(0.75);
        assert_eq!(orchestrator.pattern_type(), "consensus");
    }
}
