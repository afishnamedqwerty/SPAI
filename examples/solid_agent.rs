///! Solid-Enabled Agent Example
///!
///! Demonstrates SPAI agent with Solid Pod integration for data sovereignty.
///!
///! Prerequisites:
///! 1. Build TypeScript bridge:
///!    cd tools/solid-identity-bridge && npm install && npm run build
///!
///! 2. Set environment variables:
///!    export OPENROUTER_API_KEY=your_key
///!    export SOLID_WEBID=https://your-name.solidcommunity.net/profile/card#me
///!    export SOLID_POD=https://your-name.solidcommunity.net/
///!    export SOLID_CLIENT_ID=https://your-agent-domain.com/client.jsonld
///!
///! 3. Run:
///!    cargo run --example solid_agent --features solid-integration,mcp-tools

use anyhow::Result;
use spai::prelude::*;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use url::Url;

#[cfg(feature = "solid-integration")]
use spai::solid::{
    ConsentEnforcementGuardrail, ConsentManifest, DPoPManager, SolidIdentityClient,
    SolidOidcClient, SolidPodTool,
};

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(not(feature = "solid-integration"))]
    {
        eprintln!("❌ This example requires the 'solid-integration' feature");
        eprintln!("   Run with: cargo run --example solid_agent --features solid-integration");
        std::process::exit(1);
    }

    #[cfg(feature = "solid-integration")]
    {
        // Initialize tracing
        tracing_subscriber::fmt()
            .with_env_filter("solid_agent=debug,spai=info")
            .init();

        println!("════════════════════════════════════════════════════════════");
        println!("        SPAI Solid-Enabled Agent Example");
        println!("════════════════════════════════════════════════════════════");
        println!();

        // Get configuration from environment
        let webid_str = env::var("SOLID_WEBID")
            .unwrap_or_else(|_| "https://example.solidcommunity.net/profile/card#me".to_string());

        let pod_str = env::var("SOLID_POD")
            .unwrap_or_else(|_| "https://example.solidcommunity.net/".to_string());

        let client_id_str = env::var("SOLID_CLIENT_ID")
            .unwrap_or_else(|_| "https://spai.example.com/client.jsonld".to_string());

        let webid = Url::parse(&webid_str)?;
        let pod_iri = Url::parse(&pod_str)?;
        let client_id = Url::parse(&client_id_str)?;

        println!("✓ Configuration:");
        println!("  WebID:     {}", webid);
        println!("  Pod:       {}", pod_iri);
        println!("  Client ID: {}", client_id);
        println!();

        // Initialize OpenRouter client
        let openrouter_client = Arc::new(OpenRouterClient::from_env()?);
        println!("✓ OpenRouter client initialized");

        // Initialize Solid components
        let bridge_path = PathBuf::from("tools/solid-identity-bridge/dist/index.js");

        if !bridge_path.exists() {
            eprintln!("❌ TypeScript bridge not found at: {}", bridge_path.display());
            eprintln!("   Build it with:");
            eprintln!("   cd tools/solid-identity-bridge");
            eprintln!("   npm install && npm run build");
            std::process::exit(1);
        }

        println!("✓ TypeScript bridge found at: {}", bridge_path.display());

        // Create Solid identity client
        let identity_client = Arc::new(SolidIdentityClient::new(
            webid.clone(),
            client_id.clone(),
            bridge_path,
        )?);

        println!("✓ Solid identity client created");

        // Fetch WebID profile
        println!("🔍 Fetching WebID profile...");
        match identity_client.fetch_webid_profile() {
            Ok(profile) => {
                println!("✓ WebID Profile:");
                println!("  Name:         {:?}", profile.name);
                println!("  OIDC Issuer:  {:?}", profile.oidc_issuer);
                println!("  Storage:      {:?}", profile.storage);
            }
            Err(e) => {
                println!("⚠️  Could not fetch profile: {}", e);
                println!("   (This is expected if using example credentials)");
            }
        }
        println!();

        // Create DPoP manager
        let dpop_manager = Arc::new(DPoPManager::generate()?);
        println!("✓ DPoP manager created (key ID: {})", dpop_manager.kid());

        // Save DPoP key to keychain (optional)
        if let Err(e) = dpop_manager.save_to_keychain("spai-solid-agent", "dpop-key") {
            println!("⚠️  Could not save key to keychain: {}", e);
        } else {
            println!("✓ DPoP key saved to OS keychain");
        }

        // Create OIDC client
        let oidc_client = Arc::new(SolidOidcClient::new(
            identity_client.clone(),
            dpop_manager.clone(),
        ));

        println!("✓ Solid-OIDC client created");

        // Load consent manifest
        println!("🔍 Loading consent manifest from Pod...");
        match ConsentManifest::load(pod_iri.clone(), identity_client.clone()).await {
            Ok(manifest) => {
                let manifest = Arc::new(tokio::sync::RwLock::new(manifest));
                println!("✓ Consent manifest loaded");

                // Create consent enforcement guardrail
                let consent_guardrail = Arc::new(ConsentEnforcementGuardrail::new(manifest.clone()));
                println!("✓ Consent enforcement guardrail created");

                // Create Solid Pod tool
                let pod_tool: Arc<dyn Tool> = Arc::new(SolidPodTool::new(oidc_client.clone()));
                println!("✓ Solid Pod tool created");
                println!();

                // Create agent with Solid integration
                let agent = Agent::builder()
                    .name("Privacy-Respecting Research Agent")
                    .model("anthropic/claude-sonnet-4")
                    .system_prompt(
                        "You are a research assistant with access to the user's Solid Pod. \
                         You have a tool called 'solid_pod' that allows you to read and write data \
                         to the user's personal data store. Always respect consent policies and ACL restrictions. \
                         When asked to search for information, you can use the Pod to access user's \
                         personal documents and data."
                    )
                    .client(openrouter_client)
                    .tools(vec![pod_tool])
                    .input_guardrails(vec![consent_guardrail])
                    .react_config(ReActConfig {
                        enable_reasoning_traces: true,
                        reasoning_format: ReasoningFormat::ThoughtAction,
                        max_reasoning_tokens: 2000,
                        expose_reasoning: true,
                    })
                    .max_loops(5)
                    .build()?;

                println!("✓ Agent '{}' created", agent.name());
                println!();

                // Run agent with example task
                let task = "Please list what you know about my Solid Pod and explain what operations you can perform on it.";

                println!("📝 Task: {}", task);
                println!();
                println!("🤖 Agent processing...");
                println!("─────────────────────────────────────────────────────────");
                println!();

                match agent.react_loop(task).await {
                    Ok(output) => {
                        println!("═══════════════════════════════════════════════════════════");
                        println!("                   AGENT RESPONSE");
                        println!("═══════════════════════════════════════════════════════════");
                        println!();
                        println!("{}", output.content);
                        println!();
                        println!("═══════════════════════════════════════════════════════════");
                        println!();

                        // Show reasoning trace
                        println!("🔍 Agent Reasoning Trace:");
                        println!("─────────────────────────────────────────────────────────");
                        for (i, thought) in output.trace.thoughts.iter().enumerate() {
                            println!("=== Iteration {} ===", i + 1);
                            println!("Thought: {}", thought.content);
                            if let Some(action) = output.trace.actions.get(i) {
                                println!("Action: {:?}", action);
                            }
                            if let Some(observation) = output.trace.observations.get(i) {
                                println!("Observation: {}", observation.content);
                            }
                            println!();
                        }
                        println!("─────────────────────────────────────────────────────────");
                        println!();

                        // Show statistics
                        println!("📊 Statistics:");
                        println!("  • Agent iterations: {}", output.trace.thoughts.len());
                        println!("  • Total tokens used: {}", output.trace.total_tokens.total_tokens);
                        println!("  • Prompt tokens: {}", output.trace.total_tokens.prompt_tokens);
                        println!("  • Completion tokens: {}", output.trace.total_tokens.completion_tokens);
                        println!();
                    }
                    Err(e) => {
                        eprintln!("❌ Error: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Could not load consent manifest: {}", e);
                println!("   (This is expected if Pod doesn't have manifest yet)");
                println!();
                println!("   To create a consent manifest, add this to your Pod:");
                println!("   Location: {}/consents/browser-agent.ttl", pod_iri);
                println!();
                println!("   See SOLID_INTEGRATION_ANALYSIS.md for manifest format.");
            }
        }

        println!("════════════════════════════════════════════════════════════");
        println!("                   Example Complete!");
        println!("════════════════════════════════════════════════════════════");
    }

    Ok(())
}
