//! Comprehensive security agent that uses multiple security scanning tools.
//!
//! This example demonstrates using multiple MCP subprocess tools in conjunction
//! to perform a thorough security audit using chkrootkit, rkhunter, and lynis.
//!
//! Prerequisites:
//! 1. Install security tools:
//!    `sudo apt-get install chkrootkit rkhunter lynis`
//! 2. Set up passwordless sudo (optional but recommended):
//!    Add to /etc/sudoers.d/security-tools:
//!    ```
//!    your_username ALL=(ALL) NOPASSWD: /usr/sbin/chkrootkit
//!    your_username ALL=(ALL) NOPASSWD: /usr/bin/rkhunter
//!    your_username ALL=(ALL) NOPASSWD: /usr/sbin/lynis
//!    ```
//! 3. Update rkhunter database:
//!    `sudo rkhunter --update`
//! 4. Build all MCP servers:
//!    ```
//!    cargo build --release --manifest-path tools/chkrootkit-mcp/Cargo.toml
//!    cargo build --release --manifest-path tools/rkhunter-mcp/Cargo.toml
//!    cargo build --release --manifest-path tools/lynis-mcp/Cargo.toml
//!    ```
//! 5. Set your OpenRouter API key:
//!    `export OPENROUTER_API_KEY=your_key_here`

use spai::prelude::*;
use spai::tools::McpSubprocessTool;
use std::sync::Arc;
use std::path::Path;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== SPAI Comprehensive Security Agent Example ===\n");
    println!("This agent will run multiple security tools:");
    println!("  • chkrootkit - Rootkit detection");
    println!("  • rkhunter   - Rootkit hunter");
    println!("  • lynis      - System hardening audit\n");

    // Create OpenRouter client
    let client = OpenRouterClient::from_env()?;
    println!("✓ OpenRouter client initialized");

    // Paths to the built MCP server binaries
    let chkrootkit_bin = "tools/chkrootkit-mcp/target/release/chkrootkit-mcp";
    let rkhunter_bin = "tools/rkhunter-mcp/target/release/rkhunter-mcp";
    let lynis_bin = "tools/lynis-mcp/target/release/lynis-mcp";

    // Verify all binaries exist
    let binaries = vec![
        ("chkrootkit", chkrootkit_bin, "tools/chkrootkit-mcp/Cargo.toml"),
        ("rkhunter", rkhunter_bin, "tools/rkhunter-mcp/Cargo.toml"),
        ("lynis", lynis_bin, "tools/lynis-mcp/Cargo.toml"),
    ];

    for (name, path, manifest) in &binaries {
        if !Path::new(path).exists() {
            eprintln!("❌ {} MCP server binary not found at: {}", name, path);
            eprintln!("   Please build it first:");
            eprintln!("   cargo build --release --manifest-path {}", manifest);
            return Err(anyhow::anyhow!("{} MCP server binary not found", name));
        }
    }
    println!("✓ All MCP server binaries verified");

    // Create all three MCP subprocess tools
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

    // Create a comprehensive security agent with all three tools
    let agent = Agent::builder()
        .name("Comprehensive Security Auditor")
        .model("anthropic/claude-opus-4.5")
        .system_prompt(
            "You are an expert security auditor with access to three complementary security scanning tools:\n\
             \n\
             1. **chkrootkit** - Rootkit detection focusing on known signatures\n\
             2. **rkhunter** - Rootkit hunter checking file properties and hidden processes\n\
             3. **lynis** - Comprehensive system hardening and security audit\n\
             \n\
             Your methodology:\n\
             1. Run ALL THREE tools sequentially to get a comprehensive security picture\n\
             2. Compare and correlate findings across tools:\n\
                - If multiple tools flag the same issue → CRITICAL priority\n\
                - If one tool flags something → Investigate and assess\n\
                - Cross-reference warnings for validation\n\
             3. Analyze the lynis hardening index and recommendations\n\
             4. Synthesize findings into a coherent security assessment\n\
             5. Prioritize issues: CRITICAL (infections/backdoors) > HIGH (vulnerabilities) > MEDIUM (misconfigurations) > LOW (suggestions)\n\
             6. Provide specific, actionable remediation steps\n\
             \n\
             If a tool fails:\n\
             - Note which tool failed and why\n\
             - Continue with remaining tools\n\
             - Provide troubleshooting guidance\n\
             \n\
             Output format:\n\
             - Executive Summary (2-3 sentences)\n\
             - Tool-by-tool findings\n\
             - Cross-tool correlation\n\
             - Prioritized action items\n\
             - System hardening score (from lynis)\n\
             - Final answer: Overall security posture assessment using cross-combinatorial knowledge from each tool\n\
             \n\
             Always end with 'Final answer:' followed by your comprehensive security assessment."
        )
        .tools(vec![chkrootkit_tool, rkhunter_tool, lynis_tool])
        .react_config(ReActConfig {
            enable_reasoning_traces: true,
            reasoning_format: ReasoningFormat::ThoughtAction,
            max_reasoning_tokens: 2500,
            expose_reasoning: true,
        })
        .max_loops(12)
        .temperature(0.3)
        .client(Arc::new(client))
        .build()?;

    println!("✓ Security Agent '{}' created (ID: {})", agent.name, agent.id);
    println!("✓ Configured with {} security scanning tools:\n", agent.tools.len());
    println!("   1. chkrootkit → {}", chkrootkit_bin);
    println!("   2. rkhunter   → {}", rkhunter_bin);
    println!("   3. lynis      → {}\n", lynis_bin);

    // Run the agent
    let input = "Perform a comprehensive security audit of this system. \
                 Run ALL THREE security tools (chkrootkit, rkhunter, and lynis), \
                 compare their findings, and provide a detailed security assessment \
                 with prioritized recommendations.";
    println!("📝 Task: {}", input);
    println!("\n🔍 Initiating comprehensive security audit...\n");

    match agent.react_loop(input).await {
        Ok(output) => {
            println!("✅ Comprehensive security audit completed!\n");
            println!("═══════════════════════════════════════════════════════════");
            println!("          COMPREHENSIVE SECURITY ASSESSMENT");
            println!("═══════════════════════════════════════════════════════════\n");
            println!("{}\n", output.content);
            println!("═══════════════════════════════════════════════════════════\n");

            if agent.react_config.expose_reasoning {
                println!("🔍 Agent Reasoning Trace:");
                println!("───────────────────────────────────────────");
                println!("{}", output.trace.format());
                println!("───────────────────────────────────────────\n");
            }

            println!("📊 Scan Statistics:");
            println!("  • Agent iterations: {}", output.trace.iteration_count());
            println!("  • Total tokens used: {}", output.trace.total_tokens.total_tokens);
            println!("  • Prompt tokens: {}", output.trace.total_tokens.prompt_tokens);
            println!("  • Completion tokens: {}", output.trace.total_tokens.completion_tokens);
        }
        Err(e) => {
            eprintln!("\n❌ Security audit failed: {}", e);
            eprintln!("\nTroubleshooting:");
            eprintln!("  1. Ensure all security tools are installed:");
            eprintln!("     sudo apt-get install chkrootkit rkhunter lynis");
            eprintln!("  2. Update rkhunter database: sudo rkhunter --update");
            eprintln!("  3. Ensure you have sudo access (or run as root)");
            eprintln!("  4. Consider setting up passwordless sudo for all tools");
            eprintln!("  5. Check that all MCP server binaries exist:");
            eprintln!("     - {}", chkrootkit_bin);
            eprintln!("     - {}", rkhunter_bin);
            eprintln!("     - {}", lynis_bin);
            return Err(e.into());
        }
    }

    Ok(())
}
