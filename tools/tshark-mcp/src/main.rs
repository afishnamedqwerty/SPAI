//! tshark MCP Server - Network traffic capture and analysis
//!
//! This MCP server provides tools for capturing and analyzing network traffic
//! using tshark (Wireshark CLI) with process correlation capabilities.
//!
//! Prerequisites:
//! - Install tshark: `sudo apt-get install tshark`
//! - Configure non-root capture: `sudo dpkg-reconfigure wireshark-common`
//!   and add user to wireshark group: `sudo usermod -aG wireshark $USER`

use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::io::stdio,
    ServerHandler, ServiceExt,
};
use rmcp::model::ErrorData;
use rmcp::serde_json;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone)]
pub struct TsharkServer {
    inner: Arc<Mutex<()>>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PacketStats {
    total_packets: u64,
    protocols: HashMap<String, u64>,
    top_talkers: Vec<(String, u64)>,
    suspicious_ports: Vec<u16>,
    duration_seconds: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessConnection {
    pid: u32,
    process_name: String,
    local_addr: String,
    remote_addr: String,
    state: String,
}

#[tool_router]
impl TsharkServer {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(())),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Capture network traffic for N seconds using tshark. Returns packet summary and saves pcap file.")]
    async fn capture_traffic(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.inner.lock().await;

        // Extract parameters
        let duration = params
            .get("duration_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(60);
        let interface = params
            .get("interface")
            .and_then(|v| v.as_str())
            .unwrap_or("any");
        let filter = params
            .get("filter")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let output_file = params
            .get("output_file")
            .and_then(|v| v.as_str())
            .unwrap_or("/tmp/spai_capture.pcap");

        // Build tshark command
        let mut cmd = Command::new("sudo");
        cmd.arg("tshark")
            .arg("-i").arg(interface)
            .arg("-a").arg(format!("duration:{}", duration))
            .arg("-w").arg(output_file);

        if !filter.is_empty() {
            cmd.arg("-f").arg(filter);
        }

        let output = cmd.output();
        let output = match output {
            Ok(out) => out,
            Err(err) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to execute tshark: {}. Ensure:\n\
                     1. tshark is installed (apt-get install tshark)\n\
                     2. sudo is available OR user is in 'wireshark' group\n\
                     3. Run: sudo dpkg-reconfigure wireshark-common (select 'Yes')\n\
                     4. Run: sudo usermod -aG wireshark $USER",
                    err
                ))]));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Parse packet count from stderr (tshark reports there)
        let packet_count = extract_packet_count(&stderr);

        let summary = format!(
            "📡 Network Capture Complete\n\n\
             Duration: {} seconds\n\
             Interface: {}\n\
             Filter: {}\n\
             Packets captured: {}\n\
             Output file: {}\n",
            duration,
            interface,
            if filter.is_empty() { "none" } else { filter },
            packet_count,
            output_file
        );

        let mut content = vec![Content::text(summary)];

        if !stdout.is_empty() {
            content.push(Content::text(format!("stdout: {}", truncate(&stdout, 2000))));
        }
        if !stderr.is_empty() && !output.status.success() {
            content.push(Content::text(format!("stderr: {}", truncate(&stderr, 2000))));
        }

        // Also get current network connections for correlation
        let connections = get_network_connections();
        if !connections.is_empty() {
            let conn_summary = format!(
                "\n🔗 Active Network Connections (for PID correlation):\n{}",
                connections.iter()
                    .take(20)
                    .map(|c| format!("  PID {} ({}): {} → {} [{}]",
                        c.pid, c.process_name, c.local_addr, c.remote_addr, c.state))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            content.push(Content::text(conn_summary));
        }

        Ok(CallToolResult::success(content))
    }

    #[tool(description = "Analyze a captured pcap file for suspicious patterns. Detects unusual ports, high-frequency connections, and maps to processes.")]
    async fn analyze_packets(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.inner.lock().await;

        let pcap_file = params
            .get("pcap_file")
            .and_then(|v| v.as_str())
            .unwrap_or("/tmp/spai_capture.pcap");

        // Read the pcap file with tshark
        let output = Command::new("tshark")
            .arg("-r").arg(pcap_file)
            .arg("-T").arg("fields")
            .arg("-e").arg("ip.src")
            .arg("-e").arg("ip.dst")
            .arg("-e").arg("tcp.srcport")
            .arg("-e").arg("tcp.dstport")
            .arg("-e").arg("udp.srcport")
            .arg("-e").arg("udp.dstport")
            .arg("-e").arg("frame.protocols")
            .arg("-E").arg("separator=|")
            .output();

        let output = match output {
            Ok(out) => out,
            Err(err) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to analyze pcap: {}. Ensure tshark is installed.",
                    err
                ))]));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        // Parse the output
        let mut protocols: HashMap<String, u64> = HashMap::new();
        let mut ip_counts: HashMap<String, u64> = HashMap::new();
        let mut suspicious_ports: Vec<u16> = Vec::new();
        let mut total_packets: u64 = 0;

        // Known suspicious ports
        let suspicious_port_list: Vec<u16> = vec![
            4444, 5555, 6666, 7777, 8888, 9999,  // Common malware ports
            31337, 12345, 54321,                  // Backdoor ports
            1337, 666,                            // Hacker culture ports
            6667, 6668, 6669,                     // IRC (potential C2)
        ];

        for line in stdout.lines() {
            if line.trim().is_empty() {
                continue;
            }

            total_packets += 1;
            let parts: Vec<&str> = line.split('|').collect();

            // Count source IPs
            if let Some(src_ip) = parts.first() {
                if !src_ip.is_empty() {
                    *ip_counts.entry(src_ip.to_string()).or_insert(0) += 1;
                }
            }

            // Check for suspicious ports
            for (idx, part) in parts.iter().enumerate() {
                if idx >= 2 && idx <= 5 {
                    if let Ok(port) = part.parse::<u16>() {
                        if suspicious_port_list.contains(&port) && !suspicious_ports.contains(&port) {
                            suspicious_ports.push(port);
                        }
                    }
                }
            }

            // Count protocols
            if let Some(proto_str) = parts.last() {
                for proto in proto_str.split(':') {
                    *protocols.entry(proto.to_string()).or_insert(0) += 1;
                }
            }
        }

        // Sort top talkers
        let mut top_talkers: Vec<(String, u64)> = ip_counts.into_iter().collect();
        top_talkers.sort_by(|a, b| b.1.cmp(&a.1));
        top_talkers.truncate(10);

        // Build analysis report
        let mut report = format!(
            "🔍 Packet Analysis Report\n\
             ═══════════════════════════════════════\n\n\
             📦 Total Packets: {}\n\n",
            total_packets
        );

        // Protocols
        report.push_str("📋 Protocol Distribution:\n");
        let mut proto_vec: Vec<(&String, &u64)> = protocols.iter().collect();
        proto_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (proto, count) in proto_vec.iter().take(10) {
            let percent = (**count as f64 / total_packets as f64) * 100.0;
            report.push_str(&format!("  • {}: {} ({:.1}%)\n", proto, count, percent));
        }

        // Top talkers
        report.push_str("\n🗣️ Top Talkers (by packet count):\n");
        for (ip, count) in &top_talkers {
            report.push_str(&format!("  • {}: {} packets\n", ip, count));
        }

        // Suspicious findings
        if !suspicious_ports.is_empty() {
            report.push_str("\n⚠️ SUSPICIOUS PORTS DETECTED:\n");
            for port in &suspicious_ports {
                report.push_str(&format!("  🔴 Port {} (known malware/backdoor port)\n", port));
            }
        } else {
            report.push_str("\n✅ No known suspicious ports detected\n");
        }

        // Get process correlation
        let connections = get_network_connections();
        if !connections.is_empty() {
            report.push_str("\n🔗 Process Correlation (current connections):\n");
            for conn in connections.iter().take(15) {
                report.push_str(&format!(
                    "  PID {} ({}): {} → {}\n",
                    conn.pid, conn.process_name, conn.local_addr, conn.remote_addr
                ));
            }
        }

        let stats = PacketStats {
            total_packets,
            protocols,
            top_talkers,
            suspicious_ports,
            duration_seconds: 0, // Would need to parse from pcap
        };

        let json_data = serde_json::to_string_pretty(&stats)
            .unwrap_or_else(|_| "{}".to_string());

        Ok(CallToolResult::success(vec![
            Content::text(report),
            Content::text(format!("\nJSON data:\n{}", json_data)),
        ]))
    }

    #[tool(description = "Get summary statistics from a pcap file including protocol distribution, connection counts, and traffic volume.")]
    async fn get_packet_stats(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.inner.lock().await;

        let pcap_file = params
            .get("pcap_file")
            .and_then(|v| v.as_str())
            .unwrap_or("/tmp/spai_capture.pcap");

        // Use capinfos for statistics
        let capinfos_output = Command::new("capinfos")
            .arg(pcap_file)
            .output();

        let mut report = String::from("📊 Packet Statistics\n═══════════════════════════════════════\n\n");

        match capinfos_output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                report.push_str(&stdout);
            }
            Err(_) => {
                // Fallback to tshark -z io,stat
                let output = Command::new("tshark")
                    .arg("-r").arg(pcap_file)
                    .arg("-q")
                    .arg("-z").arg("io,stat,1")
                    .output();

                match output {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                        report.push_str(&stdout);
                    }
                    Err(e) => {
                        report.push_str(&format!("Error getting stats: {}", e));
                    }
                }
            }
        }

        // Also get protocol hierarchy
        let proto_output = Command::new("tshark")
            .arg("-r").arg(pcap_file)
            .arg("-q")
            .arg("-z").arg("io,phs")
            .output();

        if let Ok(out) = proto_output {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            if !stdout.is_empty() {
                report.push_str("\n\n📋 Protocol Hierarchy:\n");
                report.push_str(&stdout);
            }
        }

        // Get conversation stats
        let conv_output = Command::new("tshark")
            .arg("-r").arg(pcap_file)
            .arg("-q")
            .arg("-z").arg("conv,ip")
            .output();

        if let Ok(out) = conv_output {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            if !stdout.is_empty() {
                report.push_str("\n\n🔗 IP Conversations:\n");
                report.push_str(&truncate(&stdout, 3000));
            }
        }

        Ok(CallToolResult::success(vec![Content::text(report)]))
    }

    #[tool(description = "Correlate network connections with running processes using ss and lsof. Maps PIDs to their network activity.")]
    async fn correlate_processes(
        &self,
        _params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.inner.lock().await;

        let mut report = String::from(
            "🔗 Process-Network Correlation\n\
             ═══════════════════════════════════════\n\n"
        );

        // Get connections from ss
        let connections = get_network_connections();

        if connections.is_empty() {
            report.push_str("No active network connections found.\n");
        } else {
            report.push_str(&format!("Found {} active connections:\n\n", connections.len()));

            // Group by process
            let mut by_process: HashMap<String, Vec<&ProcessConnection>> = HashMap::new();
            for conn in &connections {
                by_process
                    .entry(format!("{} (PID {})", conn.process_name, conn.pid))
                    .or_default()
                    .push(conn);
            }

            for (proc_key, conns) in by_process.iter() {
                report.push_str(&format!("📦 {}\n", proc_key));
                for conn in conns.iter().take(5) {
                    report.push_str(&format!(
                        "   {} → {} [{}]\n",
                        conn.local_addr, conn.remote_addr, conn.state
                    ));
                }
                if conns.len() > 5 {
                    report.push_str(&format!("   ... and {} more\n", conns.len() - 5));
                }
                report.push('\n');
            }
        }

        // Also run lsof for more detail
        let lsof_output = Command::new("lsof")
            .arg("-i")
            .arg("-n")
            .arg("-P")
            .output();

        if let Ok(out) = lsof_output {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            if !stdout.is_empty() {
                report.push_str("\n📋 lsof Network Files:\n");
                report.push_str(&truncate(&stdout, 4000));
            }
        }

        let json_data = serde_json::to_string_pretty(&connections)
            .unwrap_or_else(|_| "[]".to_string());

        Ok(CallToolResult::success(vec![
            Content::text(report),
            Content::text(format!("\nJSON data:\n{}", json_data)),
        ]))
    }
}

#[tool_handler]
impl ServerHandler for TsharkServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Capture and analyze network traffic using tshark. Capture packets for a specified \
                 duration, analyze captured traffic for suspicious patterns, and correlate network \
                 activity with running processes. Requires tshark to be installed and proper permissions.".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let service = TsharkServer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            eprintln!("Error starting tshark MCP server: {}", e);
        })?;

    info!("tshark MCP server running over stdio");
    service.waiting().await?;
    Ok(())
}

fn extract_packet_count(stderr: &str) -> u64 {
    // tshark reports "X packets captured" in stderr
    let re = Regex::new(r"(\d+)\s+packets?\s+captured").ok();
    if let Some(regex) = re {
        if let Some(caps) = regex.captures(stderr) {
            if let Some(count) = caps.get(1) {
                return count.as_str().parse().unwrap_or(0);
            }
        }
    }
    0
}

fn get_network_connections() -> Vec<ProcessConnection> {
    let mut connections = Vec::new();

    // Use ss to get connections with process info
    let output = Command::new("ss")
        .arg("-tunap")
        .output();

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();

        for line in stdout.lines().skip(1) {
            // Parse ss output
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                let state = parts[1].to_string();
                let local_addr = parts[4].to_string();
                let remote_addr = parts[5].to_string();

                // Extract PID from users:((...)pid=XXXX,...)
                let mut pid: u32 = 0;
                let mut process_name = String::from("unknown");

                if let Some(users_part) = parts.get(6) {
                    let pid_re = Regex::new(r"pid=(\d+)").ok();
                    let name_re = Regex::new(r#"\("([^"]+)""#).ok();

                    if let Some(regex) = pid_re {
                        if let Some(caps) = regex.captures(users_part) {
                            if let Some(pid_str) = caps.get(1) {
                                pid = pid_str.as_str().parse().unwrap_or(0);
                            }
                        }
                    }

                    if let Some(regex) = name_re {
                        if let Some(caps) = regex.captures(users_part) {
                            if let Some(name) = caps.get(1) {
                                process_name = name.as_str().to_string();
                            }
                        }
                    }
                }

                if pid > 0 {
                    connections.push(ProcessConnection {
                        pid,
                        process_name,
                        local_addr,
                        remote_addr,
                        state,
                    });
                }
            }
        }
    }

    connections
}

fn truncate(input: &str, limit: usize) -> String {
    if input.len() <= limit {
        return input.to_string();
    }

    let mut truncated = input[..limit].to_string();
    truncated.push_str("\n...[truncated]...");
    truncated
}
