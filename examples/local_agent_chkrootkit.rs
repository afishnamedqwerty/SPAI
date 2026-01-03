//! Comprehensive security agent using LOCAL MODEL (OLMo via vLLM) with multiple security scanning tools.
//!
//! This example demonstrates using multiple MCP subprocess tools with a locally-hosted
//! model to perform a thorough security audit using chkrootkit, rkhunter, lynis, and htop.
//!
//! Prerequisites:
//! 1. Install and start vLLM server:
//!    ```bash
//!    pip install vllm
//!    python -m vllm.entrypoints.openai.api_server \
//!        --model allenai/OLMo-7B-1124-Instruct \
//!        --host 0.0.0.0 \
//!        --port 8000 \
//!        --dtype auto \
//!        --max-model-len 4096 \
//!        --gpu-memory-utilization 0.9
//!    ```
//!
//! 2. Install security tools:
//!    `sudo apt-get install chkrootkit rkhunter lynis htop`
//!
//! 3. Set up passwordless sudo (optional but recommended):
//!    Add to /etc/sudoers.d/security-tools:
//!    ```
//!    your_username ALL=(ALL) NOPASSWD: /usr/sbin/chkrootkit
//!    your_username ALL=(ALL) NOPASSWD: /usr/bin/rkhunter
//!    your_username ALL=(ALL) NOPASSWD: /usr/sbin/lynis
//!    ```
//!
//! 4. Update rkhunter database:
//!    `sudo rkhunter --update`
//!
//! 5. Build all MCP servers:
//!    ```
//!    cargo build --release --manifest-path tools/chkrootkit-mcp/Cargo.toml
//!    cargo build --release --manifest-path tools/rkhunter-mcp/Cargo.toml
//!    cargo build --release --manifest-path tools/lynis-mcp/Cargo.toml
//!    cargo build --release --manifest-path tools/htop-mcp/Cargo.toml
//!    ```
//!
//! 6. Set vLLM endpoint (optional, defaults to localhost:8000):
//!    `export VLLM_BASE_URL=http://localhost:8000`

use spai::prelude::*;
use spai::tools::McpSubprocessTool;
use std::path::Path;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== SPAI Local Security Agent (OLMo-7B via vLLM) ===\n");
    println!("This agent uses a locally-hosted OLMo model to coordinate");
    println!("multiple security tools for comprehensive system auditing:");
    println!("  • chkrootkit - Rootkit detection");
    println!("  • rkhunter   - Rootkit hunter");
    println!("  • lynis      - System hardening audit");
    println!("  • htop       - Process monitoring & suspicious activity detection\n");

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
    let model_id = if let Ok(models) = client.get_models().await {
        println!("✓ Available models:");
        for model in &models.data {
            println!("  - {} (owned by: {})", model.id, model.owned_by);
        }
        models
            .data
            .first()
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "allenai/OLMo-7B-1124-Instruct".to_string())
    } else {
        "allenai/OLMo-7B-1124-Instruct".to_string()
    };

    println!("\n🤖 Using model: {}", model_id);

    // Paths to the built MCP server binaries
    let chkrootkit_bin = "tools/chkrootkit-mcp/target/release/chkrootkit-mcp";
    let rkhunter_bin = "tools/rkhunter-mcp/target/release/rkhunter-mcp";
    let lynis_bin = "tools/lynis-mcp/target/release/lynis-mcp";
    let htop_bin = "tools/htop-mcp/target/release/htop-mcp";

    // Verify all binaries exist
    let binaries = vec![
        (
            "chkrootkit",
            chkrootkit_bin,
            "tools/chkrootkit-mcp/Cargo.toml",
        ),
        (
            "rkhunter",
            rkhunter_bin,
            "tools/rkhunter-mcp/Cargo.toml",
        ),
        ("lynis", lynis_bin, "tools/lynis-mcp/Cargo.toml"),
        ("htop", htop_bin, "tools/htop-mcp/Cargo.toml"),
    ];

    for (name, path, manifest) in &binaries {
        if !Path::new(path).exists() {
            eprintln!("❌ {} MCP server binary not found at: {}", name, path);
            eprintln!("   Please build it first:");
            eprintln!(
                "   cargo build --release --manifest-path {}",
                manifest
            );
            return Err(anyhow::anyhow!("{} MCP server binary not found", name));
        }
    }
    println!("✓ All MCP server binaries verified");

    // Create all four MCP subprocess tools
    let chkrootkit_tool = Arc::new(McpSubprocessTool::new(
        "chkrootkit",
        "chkrootkit Rootkit Scan",
        "Run 'sudo chkrootkit -x' to perform a rootkit scan. Detects known rootkits, \
         worms, and suspicious file modifications.",
        "chkrootkit_scan",
        chkrootkit_bin,
    ));

    let rkhunter_tool = Arc::new(McpSubprocessTool::new(
        "rkhunter",
        "rkhunter Rootkit Hunter",
        "Run 'sudo rkhunter --checkall' to scan for rootkits and backdoors. Checks file \
         properties, hidden processes, and suspicious files.",
        "rkhunter_scan",
        rkhunter_bin,
    ));

    let lynis_tool = Arc::new(McpSubprocessTool::new(
        "lynis",
        "Lynis System Audit",
        "Run 'sudo lynis audit system' to perform comprehensive security hardening audit. \
         Provides system hardening score and security recommendations.",
        "lynis_scan",
        lynis_bin,
    ));

    let htop_tool = Arc::new(McpSubprocessTool::new(
        "htop",
        "htop Process Monitor",
        "Monitor system processes and resource usage. List running processes sorted by CPU or \
         memory usage, identify suspicious processes, and get detailed system statistics.",
        "htop_monitor",
        htop_bin,
    ));

    // Create a comprehensive security agent with all four tools
    // Using local model with optimized settings for security analysis
    let agent = Agent::builder()
        .name("Local Security Auditor")
        .model(&model_id)
        .system_prompt(
            "You are an expert Ubuntu linux security auditor with access to four complementary security scanning tools:\n\
             \n\
             1. **chkrootkit** - Rootkit detection focusing on known signatures\n\
             2. **rkhunter** - Rootkit hunter checking file properties and hidden processes\n\
             3. **lynis** - Comprehensive system hardening and security audit\n\
             4. **htop** - Process monitoring and suspicious activity detection\n\
             \n\
             Your methodology:\n\
             1. Use htop to identify suspicious processes first (high CPU/memory, unusual names, hidden processes)\n\
             2. Run ALL FOUR rootkit/hardening tools (chkrootkit, rkhunter, lynis) sequentially\n\
             3. Compare and correlate findings across ALL tools with exquisite empirical rigor:\n\
                - If multiple tools flag the same issue → CRITICAL priority\n\
                - If htop shows suspicious process + rootkit detection confirms → HIGH priority\n\
                - If one tool flags something → Investigate and assess\n\
                - Cross-reference warnings for validation\n\
             4. Analyze the lynis hardening index and recommendations\n\
             5. Synthesize findings into a coherent security assessment\n\
             6. Prioritize issues: CRITICAL (infections/backdoors) > HIGH (anomalies) > MEDIUM (misconfigurations) > LOW (suggestions)\n\
             7. Provide specific, actionable remediation steps\n\
             \n\
             If a tool fails:\n\
             - Note which tool failed and why\n\
             - Continue with remaining tools\n\
             - Provide troubleshooting guidance\n\
             \n\
             Output format:\n\
             - Executive Summary (2-3 sentences)\n\
             - Process Analysis (htop findings)\n\
             - Rootkit Detection (chkrootkit + rkhunter)\n\
             - System Hardening (lynis)\n\
             - Cross-tool correlation matrix\n\
             - Prioritized action items\n\
             - System hardening score (from lynis)\n\
             - Final answer: Overall security posture assessment using cross-combinatorial knowledge from each tool with an exquisitely granular guide to checking and auditing suspicious processes (high resource usage), hidden processes, and rootkit indicators\n\
             \n\
             Always end with 'Final answer:' followed by your comprehensive security assessment."
        )
        .tools(vec![
            htop_tool,
            chkrootkit_tool,
            rkhunter_tool,
            lynis_tool,
        ])
        .react_config(ReActConfig {
            enable_reasoning_traces: true,
            reasoning_format: ReasoningFormat::ThoughtAction,
            max_reasoning_tokens: 7000, // Optimized for local model
            expose_reasoning: true,
        })
        .max_loops(16) // Allow sufficient iterations for 4 tools
        .temperature(0.3) // Lower temperature for more deterministic security analysis
        .client(Arc::new(client))
        .build()?;

    println!(
        "✓ Security Agent '{}' created (ID: {})",
        agent.name, agent.id
    );
    println!(
        "  - Model: {} (local)",
        model_id
    );
    println!(
        "  - Tools: {} security scanners",
        agent.tools.len()
    );
    println!("  - Max reasoning tokens: 2000");
    println!("  - Max iterations: {}\n", agent.max_loops);
    println!("✓ Configured tools:");
    println!("   1. htop       → {}", htop_bin);
    println!("   2. chkrootkit → {}", chkrootkit_bin);
    println!("   3. rkhunter   → {}", rkhunter_bin);
    println!("   4. lynis      → {}\n", lynis_bin);

    // Run the agent
    let input = "Perform a comprehensive security audit of this system. \
                 1. First use htop to identify suspicious processes. \
                 2. Run ALL THREE rootkit/hardening tools (chkrootkit, rkhunter, and lynis). \
                 Compare findings across all 4 tools and provide a detailed security assessment \
                 with prioritized recommendations based on cross-tool correlation.";
    println!("📝 Task: {}", input);
    println!("\n🔍 Initiating comprehensive security audit with local model...\n");

    match agent.react_loop(input).await {
        Ok(output) => {
            println!("✅ Comprehensive security audit completed!\n");
            println!("{}", "═".repeat(65));
            println!("     COMPREHENSIVE SECURITY ASSESSMENT (LOCAL MODEL)");
            println!("{}\n", "═".repeat(65));
            println!("{}\n", output.content);
            println!("{}\n", "═".repeat(65));

            if agent.react_config.expose_reasoning {
                println!("🔍 Agent Reasoning Trace:");
                println!("{}", "─".repeat(60));
                println!("{}", output.trace.format());
                println!("{}\n", "─".repeat(60));
            }

            println!("📊 Scan Statistics:");
            println!(
                "  • Agent iterations: {}",
                output.trace.iteration_count()
            );
            println!(
                "  • Total tokens used: {}",
                output.trace.total_tokens.total_tokens
            );
            println!(
                "  • Prompt tokens: {}",
                output.trace.total_tokens.prompt_tokens
            );
            println!(
                "  • Completion tokens: {}",
                output.trace.total_tokens.completion_tokens
            );
            println!("  • Model: {} (local vLLM)", model_id);
        }
        Err(e) => {
            eprintln!("\n❌ Security audit failed: {}", e);
            eprintln!("\nTroubleshooting:");
            eprintln!("  1. Ensure vLLM server is running:");
            eprintln!("     python -m vllm.entrypoints.openai.api_server \\");
            eprintln!("         --model allenai/OLMo-7B-1124-Instruct \\");
            eprintln!("         --host 0.0.0.0 --port 8000");
            eprintln!("  2. Ensure all security tools are installed:");
            eprintln!("     sudo apt-get install chkrootkit rkhunter lynis htop");
            eprintln!("  3. Update rkhunter database: sudo rkhunter --update");
            eprintln!("  4. Ensure you have sudo access (or run as root)");
            eprintln!("  5. Consider setting up passwordless sudo for all tools");
            eprintln!("  6. Check that all MCP server binaries exist:");
            eprintln!("     - {}", htop_bin);
            eprintln!("     - {}", chkrootkit_bin);
            eprintln!("     - {}", rkhunter_bin);
            eprintln!("     - {}", lynis_bin);
            return Err(e.into());
        }
    }

    Ok(())
}
