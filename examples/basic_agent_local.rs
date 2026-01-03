//! Basic agent example using locally-hosted OLMo-7B-Reasoning via vLLM
//!
//! This example demonstrates how to use SPAI with a local model instead of OpenRouter.
//!
//! # Prerequisites
//!
//! 1. Install vLLM:
//!    ```bash
//!    pip install vllm
//!    ```
//!
//! 2. Start the vLLM server with OLMo-7B-Reasoning:
//!    ```bash
//!    python -m vllm.entrypoints.openai.api_server \
//!        --model allenai/OLMo-7B-1124-Instruct \
//!        --host 0.0.0.0 \
//!        --port 8000 \
//!        --dtype auto \
//!        --max-model-len 4096 \
//!        --gpu-memory-utilization 0.9
//!    ```
//!
//!    Note: For the reasoning-focused variant, you can use:
//!    - `allenai/OLMo-7B-1124-Instruct` (general instruct model)
//!    - Or check Hugging Face for OLMo-7B reasoning variants
//!
//! 3. Set environment variables (optional):
//!    ```bash
//!    export VLLM_BASE_URL=http://localhost:8000
//!    ```
//!
//! 4. Run this example:
//!    ```bash
//!    cargo run --example basic_agent_local
//!    ```

use spai::prelude::*;
use spai::tools::echo_tool;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== SPAI Local Model Example (OLMo-7B-Reasoning via vLLM) ===\n");

    // Create vLLM client
    let client = VllmClient::from_env()?;
    println!("✓ vLLM client initialized at {}", client.config().base_url);

    // Health check
    println!("🔍 Checking vLLM server health...");
    match client.health_check().await {
        Ok(health) => println!("✓ vLLM server is healthy: {:?}", health.status),
        Err(e) => {
            eprintln!("❌ vLLM health check failed: {}", e);
            eprintln!("\n💡 Make sure vLLM is running:");
            eprintln!("   python -m vllm.entrypoints.openai.api_server \\");
            eprintln!("       --model allenai/OLMo-7B-1124-Instruct \\");
            eprintln!("       --host 0.0.0.0 \\");
            eprintln!("       --port 8000");
            return Err(e.into());
        }
    }

    // Get available models
    println!("📋 Getting available models...");
    match client.get_models().await {
        Ok(models) => {
            println!("✓ Available models:");
            for model in &models.data {
                println!("  - {} (owned by: {})", model.id, model.owned_by);
            }
        }
        Err(e) => {
            eprintln!("⚠️  Could not fetch models: {}", e);
        }
    }

    // Get the model ID from the available models or use default
    let model_id = if let Ok(models) = client.get_models().await {
        models
            .data
            .first()
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "allenai/OLMo-7B-1124-Instruct".to_string())
    } else {
        "allenai/OLMo-7B-1124-Instruct".to_string()
    };

    println!("\n🤖 Using model: {}", model_id);

    // Create an agent with ReAct enabled
    let agent = Agent::builder()
        .name("Local Reasoning Assistant")
        .model(&model_id)
        .system_prompt(
            "You are a helpful reasoning assistant powered by OLMo-7B. \
             When answering questions, think step by step and provide clear, \
             logical reasoning. Break down complex problems into smaller parts. \
             Always end your response with 'Final answer:' followed by your conclusion.",
        )
        .tools(vec![echo_tool()])
        .react_config(ReActConfig {
            enable_reasoning_traces: true,
            reasoning_format: ReasoningFormat::ThoughtAction,
            max_reasoning_tokens: 2000, // OLMo can handle long reasoning chains
            expose_reasoning: true,
        })
        .max_loops(5)
        .temperature(0.7)
        .client(Arc::new(client))
        .build()?;

    println!("✓ Agent '{}' created (ID: {})", agent.name, agent.id);
    println!("  - Model: {}", model_id);
    println!("  - Client type: Local vLLM");
    println!("  - Max reasoning tokens: 2000");

    // Example 1: Simple reasoning task
    println!("\n{}", "=".repeat(60));
    println!("Example 1: Simple Arithmetic Reasoning");
    println!("{}\n", "=".repeat(60));

    let input1 = "What is 15 * 7, and explain your reasoning step by step.";
    println!("📥 Input: {}", input1);
    println!("\n🤖 Agent is thinking...\n");

    match agent.react_loop(input1).await {
        Ok(output) => {
            println!("✅ Agent completed successfully!\n");
            println!("📤 Output: {}\n", output.content);

            if agent.react_config.expose_reasoning {
                println!("🔍 Reasoning Trace:");
                println!("{}", output.trace.format());
            }

            print_statistics(&output);
        }
        Err(e) => {
            eprintln!("❌ Agent failed: {}", e);
        }
    }

    // Example 2: More complex reasoning
    println!("\n{}", "=".repeat(60));
    println!("Example 2: Logic Puzzle");
    println!("{}\n", "=".repeat(60));

    let input2 = "If all roses are flowers, and some flowers fade quickly, \
                  can we conclude that some roses fade quickly? Explain your reasoning.";
    println!("📥 Input: {}", input2);
    println!("\n🤖 Agent is thinking...\n");

    match agent.react_loop(input2).await {
        Ok(output) => {
            println!("✅ Agent completed successfully!\n");
            println!("📤 Output: {}\n", output.content);

            if agent.react_config.expose_reasoning {
                println!("🔍 Reasoning Trace:");
                println!("{}", output.trace.format());
            }

            print_statistics(&output);
        }
        Err(e) => {
            eprintln!("❌ Agent failed: {}", e);
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("🎉 Local model demonstration complete!");
    println!("{}", "=".repeat(60));

    Ok(())
}

fn print_statistics(output: &AgentOutput) {
    println!("📊 Statistics:");
    println!("  - Iterations: {}", output.trace.iteration_count());
    println!(
        "  - Total tokens: {}",
        output.trace.total_tokens.total_tokens
    );
    println!(
        "  - Prompt tokens: {}",
        output.trace.total_tokens.prompt_tokens
    );
    println!(
        "  - Completion tokens: {}",
        output.trace.total_tokens.completion_tokens
    );
}
