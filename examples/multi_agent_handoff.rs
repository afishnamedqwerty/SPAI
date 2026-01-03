//! Multi-agent handoff example

use spai::prelude::*;
use spai::tools::{calculator_tool, echo_tool};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== ATHPTTGH Multi-Agent Handoff Example ===\n");

    // Create OpenRouter client
    let client = Arc::new(OpenRouterClient::from_env()?);
    println!("✓ OpenRouter client initialized");

    // Create a triage agent
    let triage = Agent::builder()
        .name("Triage Agent")
        .model("tngtech/deepseek-r1t2-chimera:free")
        .system_prompt(
            "You are a triage agent that routes requests to appropriate specialists. \
             Analyze the user's request and determine if it requires mathematical calculation \
             or general assistance. Always provide a final answer."
        )
        .react_config(ReActConfig::default())
        .max_loops(3)
        .client(client.clone())
        .build()?;

    println!("✓ Triage agent created (ID: {})", triage.id);

    // Create a math specialist agent
    let math_agent = Agent::builder()
        .name("Math Specialist")
        .model("tngtech/deepseek-r1t2-chimera:free")
        .system_prompt(
            "You are a mathematics specialist. Use the calculator tool to perform \
             accurate calculations and explain your work. Always end with 'Final answer:'"
        )
        .tools(vec![calculator_tool()])
        .react_config(ReActConfig::default())
        .max_loops(5)
        .client(client.clone())
        .build()?;

    println!("✓ Math specialist created (ID: {})", math_agent.id);

    // Create a general assistant agent
    let general = Agent::builder()
        .name("General Assistant")
        .model("tngtech/deepseek-r1t2-chimera:free")
        .system_prompt(
            "You are a helpful general assistant. Provide clear and concise answers \
             to user questions. Always end with 'Final answer:'"
        )
        .tools(vec![echo_tool()])
        .react_config(ReActConfig::default())
        .max_loops(3)
        .client(client)
        .build()?;

    println!("✓ General assistant created (ID: {})", general.id);

    // Test with a math question
    let input = "What is 15 multiplied by 23?";
    println!("\n📝 Input: {}", input);
    println!("🤖 Triage agent is analyzing...\n");

    match triage.react_loop(input).await {
        Ok(output) => {
            println!("✅ Triage completed!\n");
            println!("📤 Output: {}\n", output.content);

            // In a full implementation, the triage agent would hand off to math_agent
            println!("ℹ️  In a complete implementation, this would hand off to the Math Specialist");
        }
        Err(e) => {
            eprintln!("❌ Triage failed: {}", e);
        }
    }

    println!("\n=== Direct Math Specialist Test ===\n");
    println!("📝 Input: {}", input);

    match math_agent.react_loop(input).await {
        Ok(output) => {
            println!("✅ Math specialist completed!\n");
            println!("📤 Output: {}\n", output.content);
        }
        Err(e) => {
            eprintln!("❌ Math specialist failed: {}", e);
        }
    }

    Ok(())
}
