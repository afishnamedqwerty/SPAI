//! Basic agent example demonstrating the ReAct loop

use spai::prelude::*;
use spai::tools::echo_tool;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== ATHPTTGH Basic Agent Example ===\n");

    // Create OpenRouter client
    let client = OpenRouterClient::from_env()?;
    println!("✓ OpenRouter client initialized");

    // Create an agent with ReAct enabled
    let agent = Agent::builder()
        .name("Research Assistant")
        .model("tngtech/deepseek-r1t2-chimera:free")
        .system_prompt(
            "You are a helpful research assistant. When answering questions, \
             think step by step and provide clear, concise answers. \
             Always end your response with 'Final answer:' followed by your conclusion."
        )
        .tools(vec![echo_tool()])
        .react_config(ReActConfig {
            enable_reasoning_traces: true,
            reasoning_format: ReasoningFormat::ThoughtAction,
            max_reasoning_tokens: 1000,
            expose_reasoning: true,
        })
        .max_loops(5)
        .temperature(0.7)
        .client(Arc::new(client))
        .build()?;

    println!("✓ Agent '{}' created (ID: {})", agent.name, agent.id);

    // Run the agent
    let input = "What is 2 + 2?";
    println!("\n Input: {}", input);
    println!("\n🤖 Agent is thinking...\n");

    match agent.react_loop(input).await {
        Ok(output) => {
            println!("✅ Agent completed successfully!\n");
            println!("📤 Output: {}\n", output.content);

            if agent.react_config.expose_reasoning {
                println!("🔍 Reasoning Trace:");
                println!("{}", output.trace.format());
            }

            println!("📊 Statistics:");
            println!("  - Iterations: {}", output.trace.iteration_count());
            println!("  - Total tokens: {}", output.trace.total_tokens.total_tokens);
            println!("  - Prompt tokens: {}", output.trace.total_tokens.prompt_tokens);
            println!("  - Completion tokens: {}", output.trace.total_tokens.completion_tokens);
        }
        Err(e) => {
            eprintln!("❌ Agent failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
