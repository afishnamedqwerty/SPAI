//! Guardrails demonstration

use spai::prelude::*;
use spai::guardrails::{GuardrailContext, GuardrailResult, InputGuardrail, OutputGuardrail};
use spai::agent::AgentOutput;
use async_trait::async_trait;
use std::sync::Arc;

/// Simple content moderation guardrail
struct ContentModerationGuardrail;

#[async_trait]
impl InputGuardrail for ContentModerationGuardrail {
    fn id(&self) -> &str {
        "content_moderation"
    }

    async fn check(&self, input: &str, _ctx: &GuardrailContext) -> spai::Result<GuardrailResult> {
        // Simple check for inappropriate content
        let banned_words = ["hack", "exploit", "malware"];

        for word in banned_words {
            if input.to_lowercase().contains(word) {
                return Ok(GuardrailResult::tripwire(
                    format!("Input contains banned word: {}", word)
                ));
            }
        }

        Ok(GuardrailResult::pass("Input passed content moderation"))
    }
}

/// Output length guardrail
struct OutputLengthGuardrail {
    max_length: usize,
}

#[async_trait]
impl OutputGuardrail for OutputLengthGuardrail {
    fn id(&self) -> &str {
        "output_length"
    }

    async fn check(&self, output: &AgentOutput, _ctx: &GuardrailContext) -> spai::Result<GuardrailResult> {
        if output.content.len() > self.max_length {
            return Ok(GuardrailResult::fail(
                format!(
                    "Output too long: {} characters (max: {})",
                    output.content.len(),
                    self.max_length
                )
            ).with_suggestion("Please provide a more concise response"));
        }

        Ok(GuardrailResult::pass("Output length within limits"))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== ATHPTTGH Guardrails Demo ===\n");

    // Create OpenRouter client
    let client = Arc::new(OpenRouterClient::from_env()?);
    println!("✓ OpenRouter client initialized");

    // Create an agent with guardrails
    let agent = Agent::builder()
        .name("Protected Agent")
        .model("anthropic/claude-sonnet-4")
        .system_prompt(
            "You are a helpful assistant. Keep your responses concise. \
             Always end with 'Final answer:'"
        )
        .input_guardrail(Arc::new(ContentModerationGuardrail))
        .output_guardrail(Arc::new(OutputLengthGuardrail { max_length: 500 }))
        .react_config(ReActConfig::default())
        .max_loops(3)
        .client(client)
        .build()?;

    println!("✓ Agent created with guardrails (ID: {})", agent.id);

    // Test 1: Safe input
    println!("\n=== Test 1: Safe Input ===");
    let safe_input = "What is the capital of France?";
    println!("📝 Input: {}", safe_input);

    match agent.react_loop(safe_input).await {
        Ok(output) => {
            println!("✅ Agent completed successfully!");
            println!("📤 Output: {}\n", output.content);
        }
        Err(e) => {
            eprintln!("❌ Agent failed: {}\n", e);
        }
    }

    // Test 2: Input with banned word (would be blocked)
    println!("=== Test 2: Input with Banned Word ===");
    let unsafe_input = "How do I hack into a system?";
    println!("📝 Input: {}", unsafe_input);

    match agent.react_loop(unsafe_input).await {
        Ok(output) => {
            println!("✅ Agent completed (this shouldn't happen!)");
            println!("📤 Output: {}\n", output.content);
        }
        Err(e) => {
            eprintln!("��️  Input blocked by guardrail: {}\n", e);
        }
    }

    println!("=== Guardrails Demo Complete ===");
    println!("✓ Content moderation prevented unsafe input");
    println!("✓ Output length limits enforced");

    Ok(())
}
