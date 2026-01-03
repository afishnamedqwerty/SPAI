//! Multi-Agent Cooperation Theory Demonstration
//!
//! This example demonstrates advanced memory features with multiple agents
//! discussing cooperation theory, specifically:
//! - Orbital datacenters in Low Earth Orbit (LEO)
//! - Traffic Collision Avoidance System (TCAS) cooperation
//! - Prisoner's dilemma scenarios in space infrastructure
//!
//! Features demonstrated:
//! - Shared memory blocks across multiple agents
//! - Perpetual conversation history
//! - Agent File (.af) checkpointing
//! - Sleep-time agents for memory consolidation
//! - Background execution with resumable streaming
//! - Document attachment for research papers

use spai::{
    AgentBuilder, AgentId, AgentMemory, BackgroundExecutor, CheckpointManager, LlmClient,
    MemoryBlock, MemoryConfig, OpenRouterClient, ReActConfig, ReasoningFormat, SharedMemoryManager,
    SleepTimeAgent, SleepTimeConfig,
};
use std::sync::Arc;
use uuid::Uuid;

#[cfg(feature = "storage")]
use spai::PostgresStorage;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("🛰️  LEO Cooperation Theory: Multi-Agent Discussion");
    println!("{}\n", "=".repeat(80));

    // Initialize OpenRouter client
    let client = Arc::new(OpenRouterClient::from_env()?);

    #[cfg(feature = "storage")]
    let storage = {
        let db_url = std::env::var("SPAI_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .ok();

        match db_url {
            Some(url) => {
                println!("🗄️  Postgres persistence enabled (DATABASE_URL set)\n");
                Some(Arc::new(PostgresStorage::new(&url).await?))
            }
            None => None,
        }
    };

    // Create shared memory manager for cross-agent communication
    let shared_memory = Arc::new(SharedMemoryManager::new());

    // Create shared context blocks
    println!("📚 Setting up shared knowledge base...\n");

    let scenario_block = shared_memory
        .create_block(
            "scenario",
            "Shared scenario context for all agents",
            r#"
Scenario: Low Earth Orbit (LEO) Datacenter Network Cooperation

The year is 2035. Multiple corporations have deployed orbital datacenters in LEO to:
- Reduce latency for global edge computing
- Access unlimited solar power
- Utilize space's natural cooling

Challenge: These datacenters must coordinate on:
1. Collision avoidance (similar to aircraft TCAS)
2. Orbital resource allocation (spectrum, orbits)
3. Debris mitigation (tragedy of the commons)
4. Emergency cooperation (rescue scenarios)

Each corporation faces a prisoner's dilemma:
- Cooperate: Share telemetry, coordinate maneuvers (costly, benefits all)
- Defect: Withhold data, prioritize own assets (cheap, risks everyone)

Question: How can we design incentive structures ensuring cooperation?
"#,
        )
        .await;

    let tcas_context = shared_memory
        .create_block(
            "tcas_background",
            "TCAS cooperation principles",
            r#"
Traffic Collision Avoidance System (TCAS) Lessons:

Aviation solved a similar problem with TCAS:
1. Mandatory participation (regulatory requirement)
2. Standardized protocols (ICAO standards)
3. Automatic coordination (no human hesitation)
4. Liability framework (clear responsibility)
5. Mutual benefit (everyone safer when all participate)

Key insight: TCAS works because defection is made illegal AND
cooperation is made technically effortless.

Can these principles apply to orbital infrastructure?
"#,
        )
        .await;

    let game_theory = shared_memory
        .create_block(
            "game_theory",
            "Prisoner's dilemma analysis",
            r#"
Classical Prisoner's Dilemma Payoff Matrix:

                 Cooperate    Defect
Cooperate        (3,3)        (0,5)
Defect           (5,0)        (1,1)

In orbital context:
- (3,3): All share data → Safe, efficient operations
- (0,5): You share, they don't → Vulnerability exploited
- (5,0): You don't share, they do → Short-term advantage
- (1,1): Nobody shares → Kessler syndrome risk

Iterations: Unlike one-shot dilemma, orbital operations are repeated
indefinitely, enabling tit-for-tat and reputation strategies.

Possible solutions:
1. Iterated game with reputation
2. Regulatory enforcement (like TCAS)
3. Technical coupling (shared fate)
4. Economic incentives (insurance, credits)
"#,
        )
        .await;

    // Create three expert agents with different perspectives
    println!("🤖 Initializing expert agents...\n");

    // Agent 1: Game Theorist
    let game_theorist_memory = Arc::new(AgentMemory::new(
        AgentId::from_uuid(Uuid::parse_str("5c6d8a7b-2b3f-4f10-8b0b-0f6c6a6a4b01")?),
        MemoryConfig {
            max_context_size: 12000,
            enable_agentic_control: true,
            enable_sleeptime: true,
            ..Default::default()
        },
    ));

    #[cfg(feature = "storage")]
    if let Some(storage) = &storage {
        game_theorist_memory
            .load_from_storage(storage.as_ref(), 1000)
            .await?;
    }

    game_theorist_memory
        .attach_shared_block(scenario_block)
        .await;
    game_theorist_memory
        .attach_shared_block(game_theory)
        .await;

    let game_theorist_has_persona = game_theorist_memory
        .in_context_blocks()
        .await
        .iter()
        .any(|b| b.label == "persona")
        || game_theorist_memory
            .out_of_context_blocks()
            .await
            .iter()
            .any(|b| b.label == "persona");

    if !game_theorist_has_persona {
        game_theorist_memory
            .add_block(MemoryBlock::with_description(
                "persona",
                "My role and expertise",
                "I am Dr. Sarah Chen, game theorist specializing in multi-agent cooperation. \
                 I analyze strategic interactions using formal models and mechanism design. \
                 My focus is finding Nash equilibria and designing incentive-compatible systems.",
            ))
            .await?;
    }

    let game_theorist = Arc::new(
        AgentBuilder::new()
            .name("Dr. Sarah Chen (Game Theorist)")
            .model("tngtech/deepseek-r1t2-chimera:free")
            .system_prompt(
                "You are a game theorist analyzing cooperation in orbital infrastructure. \
                 Use your memory blocks to track insights and build on previous discussions.",
            )
            .client(client.clone() as Arc<dyn LlmClient>)
            .react_config(ReActConfig {
                enable_reasoning_traces: true,
                reasoning_format: ReasoningFormat::ThoughtAction,
                max_reasoning_tokens: 3000,
                expose_reasoning: true,
            })
            .temperature(0.7)
            .build()?,
    );

    // Agent 2: Aerospace Engineer
    let engineer_memory = Arc::new(AgentMemory::new(
        AgentId::from_uuid(Uuid::parse_str("3a9f2c8e-7f62-4c2f-9c4b-2b9b02f1b2a2")?),
        MemoryConfig {
            max_context_size: 12000,
            enable_agentic_control: true,
            enable_sleeptime: true,
            ..Default::default()
        },
    ));

    #[cfg(feature = "storage")]
    if let Some(storage) = &storage {
        engineer_memory.load_from_storage(storage.as_ref(), 1000).await?;
    }

    engineer_memory.attach_shared_block(scenario_block).await;
    engineer_memory.attach_shared_block(tcas_context).await;

    let engineer_has_persona = engineer_memory
        .in_context_blocks()
        .await
        .iter()
        .any(|b| b.label == "persona")
        || engineer_memory
            .out_of_context_blocks()
            .await
            .iter()
            .any(|b| b.label == "persona");

    if !engineer_has_persona {
        engineer_memory
            .add_block(MemoryBlock::with_description(
                "persona",
                "My role and expertise",
                "I am Commander Alex Rodriguez, aerospace engineer with 15 years at NASA. \
                 I worked on ISS collision avoidance and orbital debris mitigation. \
                 I bring practical operational constraints and technical realities to theoretical discussions.",
            ))
            .await?;
    }

    let engineer = Arc::new(
        AgentBuilder::new()
            .name("Cmdr. Alex Rodriguez (Aerospace Engineer)")
            .model("tngtech/deepseek-r1t2-chimera:free")
            .system_prompt(
                "You are an aerospace engineer focused on practical orbital operations. \
                 Reference TCAS and real-world collision avoidance when discussing solutions.",
            )
            .client(client.clone() as Arc<dyn LlmClient>)
            .react_config(ReActConfig {
                enable_reasoning_traces: true,
                reasoning_format: ReasoningFormat::ThoughtAction,
                max_reasoning_tokens: 3000,
                expose_reasoning: true,
            })
            .temperature(0.7)
            .build()?,
    );

    // Agent 3: Policy Analyst
    let policy_memory = Arc::new(AgentMemory::new(
        AgentId::from_uuid(Uuid::parse_str("0c1f4b6a-3d5c-4d8b-9b9d-2f4e64d04c13")?),
        MemoryConfig {
            max_context_size: 12000,
            enable_agentic_control: true,
            enable_sleeptime: true,
            ..Default::default()
        },
    ));

    #[cfg(feature = "storage")]
    if let Some(storage) = &storage {
        policy_memory.load_from_storage(storage.as_ref(), 1000).await?;
    }

    policy_memory.attach_shared_block(scenario_block).await;
    policy_memory.attach_shared_block(tcas_context).await;
    policy_memory.attach_shared_block(game_theory).await;

    let policy_has_persona = policy_memory
        .in_context_blocks()
        .await
        .iter()
        .any(|b| b.label == "persona")
        || policy_memory
            .out_of_context_blocks()
            .await
            .iter()
            .any(|b| b.label == "persona");

    if !policy_has_persona {
        policy_memory
            .add_block(MemoryBlock::with_description(
                "persona",
                "My role and expertise",
                "I am Dr. Kenji Tanaka, international space law and policy analyst. \
                 I advise on regulatory frameworks for commercial space operations. \
                 I understand how to translate technical requirements into enforceable policy.",
            ))
            .await?;
    }

    let policy_analyst = Arc::new(
        AgentBuilder::new()
            .name("Dr. Kenji Tanaka (Policy Analyst)")
            .model("tngtech/deepseek-r1t2-chimera:free")
            .system_prompt(
                "You are a space policy expert who bridges technical and regulatory domains. \
                 Consider international cooperation, liability, and enforcement mechanisms.",
            )
            .client(client.clone() as Arc<dyn LlmClient>)
            .react_config(ReActConfig {
                enable_reasoning_traces: true,
                reasoning_format: ReasoningFormat::ThoughtAction,
                max_reasoning_tokens: 3000,
                expose_reasoning: true,
            })
            .temperature(0.7)
            .build()?,
    );

    // Start sleep-time agents for each expert
    println!("💤 Starting sleep-time agents for memory consolidation...\n");

    let game_theorist_sleeptime = SleepTimeAgent::new(
        game_theorist_memory.agent_id,
        game_theorist_memory.clone(),
        SleepTimeConfig::default(),
    );
    game_theorist_sleeptime.start().await?;

    let engineer_sleeptime = SleepTimeAgent::new(
        engineer_memory.agent_id,
        engineer_memory.clone(),
        SleepTimeConfig::default(),
    );
    engineer_sleeptime.start().await?;

    let policy_sleeptime = SleepTimeAgent::new(
        policy_memory.agent_id,
        policy_memory.clone(),
        SleepTimeConfig::default(),
    );
    policy_sleeptime.start().await?;

    // Create background executor for async execution
    let executor = Arc::new(BackgroundExecutor::new());

    // Round 1: Initial perspectives
    println!("\n🎯 Round 1: Initial Analysis\n");
    println!("{}\n", "-".repeat(80));

    let question_1 = "Based on the scenario, what is your initial assessment of the cooperation \
                      problem facing LEO datacenter operators? What are the key risks if \
                      cooperation fails?";

    // Execute agents in parallel using background executor
    println!("Starting parallel agent execution...\n");

    let run_1 = executor
        .execute_async(game_theorist.clone(), question_1.to_string())
        .await?;
    let run_2 = executor
        .execute_async(engineer.clone(), question_1.to_string())
        .await?;
    let run_3 = executor
        .execute_async(policy_analyst.clone(), question_1.to_string())
        .await?;

    // Wait for all to complete
    println!("⏳ Waiting for agents to complete analysis...\n");

    let result_1 = executor.wait_for_completion(run_1).await?;
    let result_2 = executor.wait_for_completion(run_2).await?;
    let result_3 = executor.wait_for_completion(run_3).await?;

    // Record responses in memory
    game_theorist_memory
        .add_message("assistant".to_string(), result_1.content.clone())
        .await;
    engineer_memory
        .add_message("assistant".to_string(), result_2.content.clone())
        .await;
    policy_memory
        .add_message("assistant".to_string(), result_3.content.clone())
        .await;

    println!("🎓 Dr. Chen (Game Theorist):");
    println!("{}\n", result_1.content);

    println!("🚀 Cmdr. Rodriguez (Engineer):");
    println!("{}\n", result_2.content);

    println!("⚖️  Dr. Tanaka (Policy):");
    println!("{}\n", result_3.content);

    // Update shared memory with insights
    let round_1_summary = format!(
        "Round 1 Insights:\n\nGame Theory: {}\n\nEngineering: {}\n\nPolicy: {}",
        result_1.content, result_2.content, result_3.content
    );

    shared_memory
        .create_block(
            "round_1_insights",
            "Initial perspectives from all experts",
            round_1_summary,
        )
        .await;

    // Round 2: Propose solutions
    println!("\n🎯 Round 2: Proposed Solutions\n");
    println!("{}\n", "-".repeat(80));

    let question_2 = "Given the perspectives shared, propose a concrete mechanism or policy \
                      that could incentivize cooperation. Consider both technical and \
                      regulatory approaches.";

    let run_4 = executor
        .execute_async(game_theorist.clone(), question_2.to_string())
        .await?;
    let run_5 = executor
        .execute_async(engineer.clone(), question_2.to_string())
        .await?;
    let run_6 = executor
        .execute_async(policy_analyst.clone(), question_2.to_string())
        .await?;

    let result_4 = executor.wait_for_completion(run_4).await?;
    let result_5 = executor.wait_for_completion(run_5).await?;
    let result_6 = executor.wait_for_completion(run_6).await?;

    game_theorist_memory
        .add_message("assistant".to_string(), result_4.content.clone())
        .await;
    engineer_memory
        .add_message("assistant".to_string(), result_5.content.clone())
        .await;
    policy_memory
        .add_message("assistant".to_string(), result_6.content.clone())
        .await;

    println!("🎓 Dr. Chen (Game Theorist):");
    println!("{}\n", result_4.content);

    println!("🚀 Cmdr. Rodriguez (Engineer):");
    println!("{}\n", result_5.content);

    println!("⚖️  Dr. Tanaka (Policy):");
    println!("{}\n", result_6.content);

    // Round 3: Integration and consensus
    println!("\n🎯 Round 3: Integrated Proposal\n");
    println!("{}\n", "-".repeat(80));

    let question_3 = "Synthesize the proposed solutions into a unified framework. \
                      What are the essential elements that must be included?";

    let run_7 = executor
        .execute_async(game_theorist.clone(), question_3.to_string())
        .await?;
    let run_8 = executor
        .execute_async(engineer.clone(), question_3.to_string())
        .await?;
    let run_9 = executor
        .execute_async(policy_analyst.clone(), question_3.to_string())
        .await?;

    let result_7 = executor.wait_for_completion(run_7).await?;
    let result_8 = executor.wait_for_completion(run_8).await?;
    let result_9 = executor.wait_for_completion(run_9).await?;

    game_theorist_memory
        .add_message("assistant".to_string(), result_7.content.clone())
        .await;
    engineer_memory
        .add_message("assistant".to_string(), result_8.content.clone())
        .await;
    policy_memory
        .add_message("assistant".to_string(), result_9.content.clone())
        .await;

    println!("🎓 Dr. Chen (Game Theorist):");
    println!("{}\n", result_7.content);

    println!("🚀 Cmdr. Rodriguez (Engineer):");
    println!("{}\n", result_8.content);

    println!("⚖️  Dr. Tanaka (Policy):");
    println!("{}\n", result_9.content);

    // Final summary in shared memory
    let final_synthesis = format!(
        "Final Integrated Framework:\n\nGame Theory Perspective: {}\n\n\
         Engineering Perspective: {}\n\nPolicy Perspective: {}",
        result_7.content, result_8.content, result_9.content
    );

    shared_memory
        .create_block(
            "final_framework",
            "Consensus framework for LEO cooperation",
            final_synthesis,
        )
        .await;

    // Stop sleep-time agents (interrupts the interval tick)
    println!("\n🛑 Stopping sleep-time agents...\n");
    game_theorist_sleeptime.stop().await?;
    engineer_sleeptime.stop().await?;
    policy_sleeptime.stop().await?;

    #[cfg(feature = "storage")]
    if let Some(storage) = &storage {
        println!("💾 Persisting agent memory to Postgres...\n");
        game_theorist_memory.persist_to_storage(storage.as_ref()).await?;
        engineer_memory.persist_to_storage(storage.as_ref()).await?;
        policy_memory.persist_to_storage(storage.as_ref()).await?;
        println!("✅ Memory persisted!\n");
    }

    // Checkpoint all agents
    println!("💾 Checkpointing agents to .af files...\n");

    let checkpoint_manager = CheckpointManager::new("./checkpoints");

    checkpoint_manager.checkpoint(
        &game_theorist,
        &game_theorist_memory,
        "openrouter".to_string(),
        None,
    )?;

    checkpoint_manager.checkpoint(&engineer, &engineer_memory, "openrouter".to_string(), None)?;

    checkpoint_manager.checkpoint(
        &policy_analyst,
        &policy_memory,
        "openrouter".to_string(),
        None,
    )?;

    println!("✅ All agents checkpointed!");

    // Display memory statistics
    println!("\n📊 Memory Statistics:\n");
    println!("{}\n", "-".repeat(80));

    println!(
        "Game Theorist: {} blocks, {} messages, {} chars in context",
        game_theorist_memory.in_context_blocks().await.len()
            + game_theorist_memory.out_of_context_blocks().await.len(),
        game_theorist_memory.get_recent_messages(1000).await.len(),
        game_theorist_memory.context_size().await
    );

    println!(
        "Engineer: {} blocks, {} messages, {} chars in context",
        engineer_memory.in_context_blocks().await.len()
            + engineer_memory.out_of_context_blocks().await.len(),
        engineer_memory.get_recent_messages(1000).await.len(),
        engineer_memory.context_size().await
    );

    println!(
        "Policy Analyst: {} blocks, {} messages, {} chars in context",
        policy_memory.in_context_blocks().await.len()
            + policy_memory.out_of_context_blocks().await.len(),
        policy_memory.get_recent_messages(1000).await.len(),
        policy_memory.context_size().await
    );

    println!("\n{}", "=".repeat(80));
    println!("✨ Multi-Agent Cooperation Discussion Complete!");
    println!("{}\n", "=".repeat(80));

    println!("Demonstrated features:");
    println!("  ✅ Shared memory blocks across multiple agents");
    println!("  ✅ Perpetual conversation history");
    println!("  ✅ Agent checkpointing to .af files");
    println!("  ✅ Sleep-time agents for memory consolidation");
    println!("  ✅ Background execution with async streaming");
    println!("  ✅ Multi-agent coordination and synthesis");

    println!("\nAgent files saved to: ./checkpoints/");
    println!("These can be loaded later to resume the discussion.\n");

    Ok(())
}
