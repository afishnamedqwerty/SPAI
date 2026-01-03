//! Swarm Security Agent - Multi-agent orchestration with DIRECT tool execution
//!
//! This agent runs security tools DIRECTLY (via shell commands) and passes the
//! REAL output to specialized agents for analysis. This prevents LLM hallucination
//! by ensuring agents can only analyze actual system data.
//!
//! Workflow:
//! 1. Run tools directly and capture real output
//! 2. Pass real output to specialized agents for analysis
//! 3. Agents hand off findings to next agent via context
//! 4. Coordinator synthesizes all findings
//! 5. Generate summary with verification bash commands

use spai::prelude::*;
use spai::react::Observation;
use spai::handoffs::HandoffContext;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::fs;
use chrono::Utc;

/// Run a shell command and capture its output
fn run_command(cmd: &str) -> String {
    match Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.is_empty() && stdout.is_empty() {
                format!("STDERR: {}", stderr)
            } else {
                stdout.to_string()
            }
        }
        Err(e) => format!("Command failed: {}", e),
    }
}

/// Security findings from each phase
#[derive(Debug, Clone, Default)]
struct SecurityFindings {
    network_output: String,
    process_output: String,
    rootkit_output: String,
    lynis_output: String,
    network_analysis: String,
    process_analysis: String,
    rootkit_analysis: String,
    hardening_analysis: String,
    suspicious_pids: Vec<String>,
    high_severity_findings: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let output_dir = PathBuf::from(format!("security_swarm_{}", timestamp));
    fs::create_dir_all(&output_dir)?;

    println!("═══════════════════════════════════════════════════════════════");
    println!("   SPAI Swarm Security Agent - Direct Tool Execution");
    println!("   Output Directory: {}", output_dir.display());
    println!("═══════════════════════════════════════════════════════════════\n");

    // Create OpenRouter client
    let client: Arc<dyn LlmClient> = match OpenRouterClient::from_env() {
        Ok(openrouter) => {
            println!("✓ Using OpenRouter API");
            Arc::new(openrouter)
        }
        Err(e) => {
            eprintln!("❌ OpenRouter client not available: {}", e);
            return Err(anyhow::anyhow!("OpenRouter API key required"));
        }
    };

    let model_id = "anthropic/claude-sonnet-4".to_string();
    println!("✓ Model: {}\n", model_id);

    let mut findings = SecurityFindings::default();
    let mut handoff_context = HandoffContext::new("Comprehensive security assessment");

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 1: COLLECT REAL DATA (Direct command execution - NO LLM)
    // ═══════════════════════════════════════════════════════════════════════════
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  PHASE 1: Collecting REAL System Data (Direct Execution)   │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    // Network data - 60 second tshark capture
    println!("  📡 Capturing network traffic for 60 seconds with tshark...");
    println!("     (This will take 60 seconds - please wait)");

    // Capture packets for 60 seconds with tshark
    let tshark_capture = run_command(
        "sudo timeout 60 tshark -i any -q -z conv,tcp -z conv,udp 2>/dev/null || \
         echo 'tshark not available, falling back to ss/netstat'"
    );

    println!("     ✓ Traffic capture complete, collecting connection data...");

    // Use SPAI portlist tool for comprehensive port analysis
    let portlist_all = run_command("sudo portlist -a -p 2>/dev/null || echo 'portlist not available'");
    let portlist_suspicious = run_command("sudo portlist -s 2>/dev/null || echo 'portlist not available'");

    findings.network_output = format!(
        "=== SPAI PORTLIST - ALL CONNECTIONS ===\n{}\n\n\
         === SPAI PORTLIST - SUSPICIOUS PORTS ===\n{}\n\n\
         === 60-SECOND TSHARK CAPTURE ===\n{}\n\n\
         === LISTENING PORTS ===\n{}\n\n\
         === ESTABLISHED CONNECTIONS ===\n{}\n\n\
         === SOCKET STATISTICS ===\n{}\n\n\
         === PROCESS NETWORK ACTIVITY (lsof) ===\n{}\n\n\
         === NETSTAT CONNECTIONS ===\n{}",
        portlist_all,
        portlist_suspicious,
        tshark_capture,
        run_command("ss -tulnp 2>/dev/null"),
        run_command("ss -tupn state established 2>/dev/null"),
        run_command("ss -s 2>/dev/null"),
        run_command("lsof -i -n -P 2>/dev/null | head -60"),
        run_command("netstat -tupn 2>/dev/null | head -60 || echo 'netstat not available'")
    );
    fs::write(output_dir.join("01_network_raw.txt"), &findings.network_output)?;
    println!("     ✓ Saved to 01_network_raw.txt");

    // Comprehensive process profiling
    println!("  🔍 Running comprehensive process profiling...");
    findings.process_output = format!(
        "=== TOP 40 PROCESSES BY CPU ===\n{}\n\n\
         === TOP 40 PROCESSES BY MEMORY ===\n{}\n\n\
         === ALL RUNNING PROCESSES ===\n{}\n\n\
         === PROCESS TREE (pstree -pa) ===\n{}\n\n\
         === PROCESS TREE (ps auxf) ===\n{}\n\n\
         === OPEN FILES BY PROCESS (lsof) ===\n{}\n\n\
         === NETWORK SOCKETS PER PROCESS ===\n{}\n\n\
         === SUSPICIOUS INDICATORS ===\n\
         Processes running from /tmp or /dev/shm:\n{}\n\n\
         Processes with very high CPU (>30%):\n{}\n\n\
         Processes with very high memory (>10%):\n{}\n\n\
         === KERNEL THREADS ===\n{}\n\n\
         === ZOMBIE PROCESSES ===\n{}\n\n\
         === PROCESSES WITH DELETED EXECUTABLES ===\n{}",
        run_command("ps aux --sort=-%cpu | head -41"),
        run_command("ps aux --sort=-%mem | head -41"),
        run_command("ps -eo pid,ppid,user,%cpu,%mem,stat,start,time,comm --sort=-%cpu | head -100"),
        run_command("pstree -pa 2>/dev/null | head -80"),
        run_command("ps auxf | head -80"),
        run_command("lsof +D /tmp 2>/dev/null | head -30 || echo 'No files open in /tmp'"),
        run_command("ss -tupn 2>/dev/null | awk '{print $7}' | sort | uniq -c | sort -rn | head -20"),
        run_command("ps aux | grep -E '/tmp/|/dev/shm/' | grep -v grep | head -15"),
        run_command("ps aux | awk 'NR>1 && $3>30'"),
        run_command("ps aux | awk 'NR>1 && $4>10'"),
        run_command("ps aux | awk '$8 ~ /^.?D/'"),
        run_command("ps aux | awk '$8 ~ /Z/'"),
        run_command("ls -la /proc/*/exe 2>&1 | grep deleted | head -10")
    );
    fs::write(output_dir.join("02_process_raw.txt"), &findings.process_output)?;
    println!("     ✓ Saved to 02_process_raw.txt");


    // Rootkit detection
    println!("  🦠 Running rootkit detection...");
    findings.rootkit_output = format!(
        "=== CHKROOTKIT SCAN ===\n{}\n\n=== RKHUNTER SCAN ===\n{}\n\n=== MANUAL CHECKS ===\nSUID files in /tmp:\n{}\nHidden files in /tmp:\n{}\nRecent /bin changes:\n{}",
        run_command("sudo chkrootkit 2>&1 | head -100"),
        run_command("sudo rkhunter --check --skip-keypress --report-warnings-only 2>&1 | head -100"),
        run_command("find /tmp -perm -4000 2>/dev/null | head -10"),
        run_command("find /tmp -name '.*' -type f 2>/dev/null | head -10"),
        run_command("find /bin /sbin -type f -mtime -7 2>/dev/null | head -10")
    );
    fs::write(output_dir.join("03_rootkit_raw.txt"), &findings.rootkit_output)?;
    println!("     ✓ Saved to 03_rootkit_raw.txt");

    // Lynis hardening
    println!("  🛡️  Running system hardening audit...");
    findings.lynis_output = run_command("sudo lynis audit system --quick --no-colors 2>&1 | head -300");
    fs::write(output_dir.join("04_lynis_raw.txt"), &findings.lynis_output)?;
    println!("     ✓ Saved to 04_lynis_raw.txt\n");

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 2: AGENT ANALYSIS (Agents analyze REAL data only)
    // ═══════════════════════════════════════════════════════════════════════════
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  PHASE 2: Multi-Agent Analysis of Real Data                │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    // Network Analysis Agent
    println!("  📡 Network Monitor analyzing real data...");
    let network_agent = Agent::builder()
        .name("Network Monitor")
        .model(&model_id)
        .system_prompt(
            "You are a network security analyst. You will receive REAL network data from the system.\n\n\
             CRITICAL: Analyze ONLY the data provided below. Do NOT invent any findings.\n\n\
             The data includes output from SPAI portlist tool which highlights suspicious ports.\n\
             Look for:\n\
             - Suspicious listening ports (4444, 31337, 6667, 1337, etc.) - portlist will flag these\n\
             - Unusual established connections to unknown IPs\n\
             - Processes with unexpected network activity\n\
             - Any ports marked as SUSPICIOUS by portlist\n\n\
             Provide a brief analysis and list any suspicious PIDs.\n\
             End with: SUSPICIOUS_PIDS: [list] or SUSPICIOUS_PIDS: NONE"
        )
        .temperature(0.1)
        .client(client.clone())
        .build()?;

    let network_prompt = format!(
        "Analyze this REAL network data from the system:\n\n{}\n\nProvide analysis based ONLY on this data.",
        truncate_str(&findings.network_output, 6000)
    );

    match network_agent.react_loop(&network_prompt).await {
        Ok(output) => {
            findings.network_analysis = output.content.clone();
            handoff_context = handoff_context.with_observation(Observation::new(
                format!("[network] {}", truncate_str(&output.content, 500)),
            ));
            println!("     ✓ Network analysis complete");
        }
        Err(e) => {
            findings.network_analysis = format!("Analysis failed: {}", e);
            println!("     ⚠️ Network analysis failed: {}", e);
        }
    }

    // Process Analysis Agent
    println!("  🔍 Process Analyzer analyzing real data...");
    let process_agent = Agent::builder()
        .name("Process Analyzer")
        .model(&model_id)
        .system_prompt(
            "You are a process analyst. You will receive REAL process data from the system.\n\n\
             CRITICAL: Analyze ONLY the data provided below. Do NOT invent any findings.\n\n\
             Look for:\n\
             - Suspicious process names or paths\n\
             - Processes running from /tmp or /dev/shm\n\
             - Abnormally high CPU or memory usage\n\
             - Unusual parent-child relationships\n\n\
             Provide a brief analysis.\n\
             End with: SEVERITY: [CLEAN|LOW|MEDIUM|HIGH|CRITICAL] - [reason]"
        )
        .temperature(0.1)
        .client(client.clone())
        .build()?;

    let process_prompt = format!(
        "Analyze this REAL process data from the system:\n\n{}\n\n\
         Previous network analysis found:\n{}\n\n\
         Provide analysis based ONLY on this data.",
        truncate_str(&findings.process_output, 6000),
        truncate_str(&findings.network_analysis, 1000)
    );

    match process_agent.react_loop(&process_prompt).await {
        Ok(output) => {
            findings.process_analysis = output.content.clone();
            handoff_context = handoff_context.with_observation(Observation::new(
                format!("[process] {}", truncate_str(&output.content, 500)),
            ));
            println!("     ✓ Process analysis complete");
        }
        Err(e) => {
            findings.process_analysis = format!("Analysis failed: {}", e);
            println!("     ⚠️ Process analysis failed: {}", e);
        }
    }

    // Rootkit Analysis Agent
    println!("  🦠 Rootkit Hunter analyzing real scan results...");
    let rootkit_agent = Agent::builder()
        .name("Rootkit Hunter")
        .model(&model_id)
        .system_prompt(
            "You are a rootkit analyst. You will receive REAL chkrootkit and rkhunter output.\n\n\
             CRITICAL: Analyze ONLY the data provided below. Do NOT invent any findings.\n\n\
             Look for:\n\
             - Any 'INFECTED' or 'WARNING' messages\n\
             - Hidden files or processes detected\n\
             - Modified system binaries\n\
             - Suspicious kernel modules\n\n\
             Provide a brief analysis.\n\
             End with: ROOTKIT_STATUS: [CLEAN|WARNING|INFECTED] - [reason]"
        )
        .temperature(0.1)
        .client(client.clone())
        .build()?;

    let rootkit_prompt = format!(
        "Analyze this REAL rootkit scan output:\n\n{}\n\nProvide analysis based ONLY on this data.",
        truncate_str(&findings.rootkit_output, 6000)
    );

    match rootkit_agent.react_loop(&rootkit_prompt).await {
        Ok(output) => {
            findings.rootkit_analysis = output.content.clone();
            handoff_context = handoff_context.with_observation(Observation::new(
                format!("[rootkit] {}", truncate_str(&output.content, 500)),
            ));
            println!("     ✓ Rootkit analysis complete");
        }
        Err(e) => {
            findings.rootkit_analysis = format!("Analysis failed: {}", e);
            println!("     ⚠️ Rootkit analysis failed: {}", e);
        }
    }

    // Hardening Analysis Agent
    println!("  🛡️  Hardening Auditor analyzing lynis output...");
    let hardening_agent = Agent::builder()
        .name("Hardening Auditor")
        .model(&model_id)
        .system_prompt(
            "You are a system hardening auditor. You will receive REAL lynis audit output.\n\n\
             CRITICAL: Analyze ONLY the data provided below. Do NOT invent any scores or findings.\n\n\
             Extract:\n\
             - The actual hardening index score from the output\n\
             - Top 5 security warnings or suggestions\n\n\
             Provide a brief analysis.\n\
             End with: HARDENING_SCORE: [score from output] - [top recommendation]"
        )
        .temperature(0.1)
        .client(client.clone())
        .build()?;

    let hardening_prompt = format!(
        "Analyze this REAL lynis audit output:\n\n{}\n\nProvide analysis based ONLY on this data.",
        truncate_str(&findings.lynis_output, 6000)
    );

    match hardening_agent.react_loop(&hardening_prompt).await {
        Ok(output) => {
            findings.hardening_analysis = output.content.clone();
            handoff_context = handoff_context.with_observation(Observation::new(
                format!("[hardening] {}", truncate_str(&output.content, 500)),
            ));
            println!("     ✓ Hardening analysis complete");
        }
        Err(e) => {
            findings.hardening_analysis = format!("Analysis failed: {}", e);
            println!("     ⚠️ Hardening analysis failed: {}", e);
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 3: COORDINATOR SYNTHESIS
    // ═══════════════════════════════════════════════════════════════════════════
    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│  PHASE 3: Coordinator Synthesis                             │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    println!("  🎯 Coordinator synthesizing all findings...");
    let coordinator_agent = Agent::builder()
        .name("Security Coordinator")
        .model(&model_id)
        .system_prompt(
            "You are the security coordinator. Synthesize the analyses from all agents.\n\n\
             CRITICAL: Base your assessment ONLY on the agent analyses provided.\n\n\
             Provide:\n\
             1. EXECUTIVE SUMMARY (2-3 sentences)\n\
             2. SECURITY POSTURE: [SECURE|WARNING|COMPROMISED]\n\
             3. HIGH SEVERITY FINDINGS (if any, with specific PIDs/IPs/ports)\n\
             4. VERIFICATION COMMANDS - bash commands to verify the highest severity findings\n\n\
             Format verification commands as:\n\
             ```bash\n\
             # Description of what this verifies\n\
             command here\n\
             ```"
        )
        .temperature(0.1)
        .client(client.clone())
        .build()?;

    let coordinator_prompt = format!(
        "Synthesize these security findings from our agent swarm:\n\n\
         === NETWORK ANALYSIS ===\n{}\n\n\
         === PROCESS ANALYSIS ===\n{}\n\n\
         === ROOTKIT ANALYSIS ===\n{}\n\n\
         === HARDENING ANALYSIS ===\n{}\n\n\
         Provide executive summary, security posture, and verification bash commands.",
        truncate_str(&findings.network_analysis, 1500),
        truncate_str(&findings.process_analysis, 1500),
        truncate_str(&findings.rootkit_analysis, 1500),
        truncate_str(&findings.hardening_analysis, 1500)
    );

    let final_assessment = match coordinator_agent.react_loop(&coordinator_prompt).await {
        Ok(output) => output.content,
        Err(e) => format!("Coordinator failed: {}", e),
    };

    // ═══════════════════════════════════════════════════════════════════════════
    // PHASE 4: GENERATE SUMMARY FILE
    // ═══════════════════════════════════════════════════════════════════════════
    println!("\n┌─────────────────────────────────────────────────────────────┐");
    println!("│  PHASE 4: Generating Summary                                │");
    println!("└─────────────────────────────────────────────────────────────┘\n");

    let summary = format!(
        "═══════════════════════════════════════════════════════════════\n\
         SPAI SWARM SECURITY ASSESSMENT SUMMARY\n\
         Generated: {}\n\
         Hostname: {}\n\
         ═══════════════════════════════════════════════════════════════\n\n\
         {}\n\n\
         ═══════════════════════════════════════════════════════════════\n\
         RAW DATA FILES\n\
         ═══════════════════════════════════════════════════════════════\n\
         cat {}/01_network_raw.txt\n\
         cat {}/02_process_raw.txt\n\
         cat {}/03_rootkit_raw.txt\n\
         cat {}/04_lynis_raw.txt\n\n\
         ═══════════════════════════════════════════════════════════════\n\
         AGENT ANALYSES\n\
         ═══════════════════════════════════════════════════════════════\n\n\
         --- Network Agent ---\n{}\n\n\
         --- Process Agent ---\n{}\n\n\
         --- Rootkit Agent ---\n{}\n\n\
         --- Hardening Agent ---\n{}\n\n\
         ═══════════════════════════════════════════════════════════════\n\
         QUICK VERIFICATION COMMANDS\n\
         ═══════════════════════════════════════════════════════════════\n\n\
         # Check all ports with SPAI portlist (highlights suspicious ones)\n\
         sudo portlist -a -s\n\n\
         # Check listening ports\n\
         ss -tulnp | grep LISTEN\n\n\
         # Check high CPU processes\n\
         ps aux --sort=-%cpu | head -10\n\n\
         # Check high memory processes\n\
         ps aux --sort=-%mem | head -10\n\n\
         # Check for suspicious network connections\n\
         lsof -i -n -P | grep ESTABLISHED\n\n\
         # Run quick rootkit check\n\
         sudo chkrootkit | grep -i infected\n\n\
         # Check for hidden files in /tmp\n\
         find /tmp -name '.*' -type f 2>/dev/null\n",
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        run_command("hostname").trim(),
        final_assessment,
        output_dir.display(), output_dir.display(), output_dir.display(), output_dir.display(),
        truncate_str(&findings.network_analysis, 1000),
        truncate_str(&findings.process_analysis, 1000),
        truncate_str(&findings.rootkit_analysis, 1000),
        truncate_str(&findings.hardening_analysis, 1000)
    );

    fs::write(output_dir.join("summary.txt"), &summary)?;
    println!("   ✓ Summary saved to {}/summary.txt", output_dir.display());

    // Print final summary
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   ASSESSMENT COMPLETE");
    println!("   Output: {}/", output_dir.display());
    println!("═══════════════════════════════════════════════════════════════\n");

    println!("{}", final_assessment);

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   View full summary: cat {}/summary.txt", output_dir.display());
    println!("═══════════════════════════════════════════════════════════════\n");

    Ok(())
}

fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}
