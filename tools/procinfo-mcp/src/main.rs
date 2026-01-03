//! procinfo MCP Server - Process profiling and analysis
//!
//! This MCP server provides granular process profiling tools using standard
//! Linux utilities: ps, pstree, lsof, and ss.
//!
//! No special permissions required for basic usage, but some features
//! may require elevated privileges for full process visibility.

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
pub struct ProcInfoServer {
    inner: Arc<Mutex<()>>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessInfo {
    user: String,
    pid: u32,
    cpu_percent: f32,
    mem_percent: f32,
    vsz_kb: u64,
    rss_kb: u64,
    tty: String,
    stat: String,
    start: String,
    time: String,
    command: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NetworkFile {
    command: String,
    pid: u32,
    user: String,
    fd: String,
    file_type: String,
    device: String,
    size_off: String,
    node: String,
    name: String,
}

#[tool_router]
impl ProcInfoServer {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(())),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get detailed process listing using ps aux with optional filtering by user, command pattern, or resource usage thresholds.")]
    async fn ps_aux_detailed(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.inner.lock().await;

        // Extract filter parameters
        let user_filter = params.get("user").and_then(|v| v.as_str());
        let command_filter = params.get("command_pattern").and_then(|v| v.as_str());
        let min_cpu = params.get("min_cpu_percent").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let min_mem = params.get("min_mem_percent").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let sort_by = params.get("sort_by").and_then(|v| v.as_str()).unwrap_or("cpu");

        // Run ps aux
        let output = Command::new("ps")
            .arg("aux")
            .arg("--sort")
            .arg(match sort_by {
                "memory" | "mem" => "-%mem",
                "cpu" => "-%cpu",
                "pid" => "pid",
                "user" => "user",
                _ => "-%cpu",
            })
            .output();

        let output = match output {
            Ok(out) => out,
            Err(err) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to execute ps aux: {}",
                    err
                ))]));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut processes: Vec<ProcessInfo> = Vec::new();

        // Parse ps aux output
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 11 {
                continue;
            }

            let user = parts[0].to_string();
            let pid: u32 = parts[1].parse().unwrap_or(0);
            let cpu_percent: f32 = parts[2].parse().unwrap_or(0.0);
            let mem_percent: f32 = parts[3].parse().unwrap_or(0.0);
            let vsz_kb: u64 = parts[4].parse().unwrap_or(0);
            let rss_kb: u64 = parts[5].parse().unwrap_or(0);
            let tty = parts[6].to_string();
            let stat = parts[7].to_string();
            let start = parts[8].to_string();
            let time = parts[9].to_string();
            let command = parts[10..].join(" ");

            // Apply filters
            if let Some(user_f) = user_filter {
                if !user.contains(user_f) {
                    continue;
                }
            }

            if let Some(cmd_f) = command_filter {
                let re = Regex::new(cmd_f).ok();
                if let Some(regex) = re {
                    if !regex.is_match(&command) {
                        continue;
                    }
                } else if !command.contains(cmd_f) {
                    continue;
                }
            }

            if cpu_percent < min_cpu || mem_percent < min_mem {
                continue;
            }

            processes.push(ProcessInfo {
                user,
                pid,
                cpu_percent,
                mem_percent,
                vsz_kb,
                rss_kb,
                tty,
                stat,
                start,
                time,
                command,
            });
        }

        // Build report
        let mut report = format!(
            "📋 Process Listing ({} processes)\n\
             ═══════════════════════════════════════\n\n",
            processes.len()
        );

        if processes.is_empty() {
            report.push_str("No processes matched the specified filters.\n");
        } else {
            report.push_str(&format!(
                "{:<10} {:>6} {:>6} {:>6} {:>10} {:>10} {}\n",
                "USER", "PID", "CPU%", "MEM%", "VSZ(KB)", "RSS(KB)", "COMMAND"
            ));
            report.push_str(&"-".repeat(80));
            report.push('\n');

            for proc in processes.iter().take(50) {
                report.push_str(&format!(
                    "{:<10} {:>6} {:>5.1}% {:>5.1}% {:>10} {:>10} {}\n",
                    truncate_str(&proc.user, 10),
                    proc.pid,
                    proc.cpu_percent,
                    proc.mem_percent,
                    proc.vsz_kb,
                    proc.rss_kb,
                    truncate_str(&proc.command, 50)
                ));
            }

            if processes.len() > 50 {
                report.push_str(&format!("\n... and {} more processes\n", processes.len() - 50));
            }
        }

        let json_data = serde_json::to_string_pretty(&processes.iter().take(50).collect::<Vec<_>>())
            .unwrap_or_else(|_| "[]".to_string());

        Ok(CallToolResult::success(vec![
            Content::text(report),
            Content::text(format!("\nJSON data:\n{}", json_data)),
        ]))
    }

    #[tool(description = "Get process tree visualization using pstree. Shows parent-child relationships with PIDs and command arguments.")]
    async fn get_pstree(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.inner.lock().await;

        let pid = params.get("pid").and_then(|v| v.as_u64());
        let show_pids = params.get("show_pids").and_then(|v| v.as_bool()).unwrap_or(true);
        let show_args = params.get("show_args").and_then(|v| v.as_bool()).unwrap_or(true);

        let mut cmd = Command::new("pstree");

        if show_pids {
            cmd.arg("-p");
        }
        if show_args {
            cmd.arg("-a");
        }

        // Show threads
        cmd.arg("-T");

        if let Some(pid_val) = pid {
            cmd.arg(pid_val.to_string());
        }

        let output = match cmd.output() {
            Ok(out) => out,
            Err(err) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to execute pstree: {}. Install with: apt-get install psmisc",
                    err
                ))]));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        let report = format!(
            "🌳 Process Tree\n\
             ═══════════════════════════════════════\n\n\
             {}\n",
            if stdout.len() > 8000 {
                truncate(&stdout, 8000)
            } else {
                stdout
            }
        );

        Ok(CallToolResult::success(vec![Content::text(report)]))
    }

    #[tool(description = "List network file descriptors by process using lsof. Shows which processes have network connections open.")]
    async fn lsof_network(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.inner.lock().await;

        let pid = params.get("pid").and_then(|v| v.as_u64());
        let protocol = params.get("protocol").and_then(|v| v.as_str());

        let mut cmd = Command::new("lsof");
        cmd.arg("-i");  // Network files
        cmd.arg("-n");  // No hostname resolution
        cmd.arg("-P");  // No port name resolution

        if let Some(pid_val) = pid {
            cmd.arg("-p").arg(pid_val.to_string());
        }

        if let Some(proto) = protocol {
            match proto.to_lowercase().as_str() {
                "tcp" => { cmd.arg("-iTCP"); }
                "udp" => { cmd.arg("-iUDP"); }
                _ => {}
            }
        }

        let output = match cmd.output() {
            Ok(out) => out,
            Err(err) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to execute lsof: {}. May require elevated privileges for full visibility.",
                    err
                ))]));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut network_files: Vec<NetworkFile> = Vec::new();

        // Parse lsof output
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 9 {
                let nf = NetworkFile {
                    command: parts[0].to_string(),
                    pid: parts[1].parse().unwrap_or(0),
                    user: parts[2].to_string(),
                    fd: parts[3].to_string(),
                    file_type: parts[4].to_string(),
                    device: parts[5].to_string(),
                    size_off: parts[6].to_string(),
                    node: parts[7].to_string(),
                    name: parts[8..].join(" "),
                };
                network_files.push(nf);
            }
        }

        // Group by process
        let mut by_process: HashMap<String, Vec<&NetworkFile>> = HashMap::new();
        for nf in &network_files {
            by_process
                .entry(format!("{} (PID {})", nf.command, nf.pid))
                .or_default()
                .push(nf);
        }

        let mut report = format!(
            "🔌 Network File Descriptors ({} files, {} processes)\n\
             ═══════════════════════════════════════\n\n",
            network_files.len(),
            by_process.len()
        );

        for (proc_key, files) in by_process.iter() {
            report.push_str(&format!("📦 {}\n", proc_key));
            for nf in files.iter().take(10) {
                report.push_str(&format!(
                    "   {} {} {} → {}\n",
                    nf.fd, nf.file_type, nf.node, nf.name
                ));
            }
            if files.len() > 10 {
                report.push_str(&format!("   ... and {} more\n", files.len() - 10));
            }
            report.push('\n');
        }

        let json_data = serde_json::to_string_pretty(&network_files.iter().take(100).collect::<Vec<_>>())
            .unwrap_or_else(|_| "[]".to_string());

        Ok(CallToolResult::success(vec![
            Content::text(report),
            Content::text(format!("\nJSON data:\n{}", json_data)),
        ]))
    }

    #[tool(description = "Correlate PIDs with network connections using ss. Maps each PID to its active TCP/UDP connections.")]
    async fn correlate_pid_packets(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.inner.lock().await;

        let target_pid = params.get("pid").and_then(|v| v.as_u64());

        // Use ss to get connections with process info
        let output = Command::new("ss")
            .arg("-tunap")
            .output();

        let output = match output {
            Ok(out) => out,
            Err(err) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to execute ss: {}",
                    err
                ))]));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        #[derive(Debug, Serialize)]
        struct PidConnection {
            pid: u32,
            process: String,
            protocol: String,
            state: String,
            local: String,
            remote: String,
        }

        let mut connections: Vec<PidConnection> = Vec::new();
        let pid_re = Regex::new(r"pid=(\d+)").ok();
        let name_re = Regex::new(r#"\("([^"]+)""#).ok();

        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 6 {
                continue;
            }

            let protocol = parts[0].to_string();
            let state = parts[1].to_string();
            let local = parts[4].to_string();
            let remote = parts[5].to_string();

            let users_part = parts.get(6).unwrap_or(&"");

            let mut pid: u32 = 0;
            let mut process = String::from("unknown");

            if let Some(ref regex) = pid_re {
                if let Some(caps) = regex.captures(users_part) {
                    if let Some(pid_str) = caps.get(1) {
                        pid = pid_str.as_str().parse().unwrap_or(0);
                    }
                }
            }

            if let Some(ref regex) = name_re {
                if let Some(caps) = regex.captures(users_part) {
                    if let Some(name) = caps.get(1) {
                        process = name.as_str().to_string();
                    }
                }
            }

            // Filter by target PID if specified
            if let Some(target) = target_pid {
                if pid != target as u32 {
                    continue;
                }
            }

            if pid > 0 {
                connections.push(PidConnection {
                    pid,
                    process,
                    protocol,
                    state,
                    local,
                    remote,
                });
            }
        }

        // Group by PID
        let mut by_pid: HashMap<u32, Vec<&PidConnection>> = HashMap::new();
        for conn in &connections {
            by_pid.entry(conn.pid).or_default().push(conn);
        }

        let mut report = format!(
            "🔗 PID-Network Correlation ({} connections, {} processes)\n\
             ═══════════════════════════════════════\n\n",
            connections.len(),
            by_pid.len()
        );

        // Sort by connection count
        let mut pid_counts: Vec<(u32, usize, String)> = by_pid
            .iter()
            .map(|(pid, conns)| {
                let name = conns.first().map(|c| c.process.clone()).unwrap_or_default();
                (*pid, conns.len(), name)
            })
            .collect();
        pid_counts.sort_by(|a, b| b.1.cmp(&a.1));

        for (pid, count, name) in pid_counts.iter().take(20) {
            report.push_str(&format!("📦 {} (PID {}) - {} connections\n", name, pid, count));

            if let Some(conns) = by_pid.get(pid) {
                for conn in conns.iter().take(5) {
                    report.push_str(&format!(
                        "   {} {} → {} [{}]\n",
                        conn.protocol, conn.local, conn.remote, conn.state
                    ));
                }
                if conns.len() > 5 {
                    report.push_str(&format!("   ... and {} more\n", conns.len() - 5));
                }
            }
            report.push('\n');
        }

        if pid_counts.len() > 20 {
            report.push_str(&format!("\n... and {} more processes\n", pid_counts.len() - 20));
        }

        let json_data = serde_json::to_string_pretty(&connections.iter().take(100).collect::<Vec<_>>())
            .unwrap_or_else(|_| "[]".to_string());

        Ok(CallToolResult::success(vec![
            Content::text(report),
            Content::text(format!("\nJSON data:\n{}", json_data)),
        ]))
    }

    #[tool(description = "Get detailed information about a specific process including environment, open files, and memory maps.")]
    async fn get_process_details(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.inner.lock().await;

        let pid = params.get("pid").and_then(|v| v.as_u64())
            .ok_or_else(|| ErrorData::invalid_request("Missing required parameter: pid", None))?;

        let mut report = format!(
            "🔍 Process Details (PID {})\n\
             ═══════════════════════════════════════\n\n",
            pid
        );

        // Basic process info from /proc
        let proc_path = format!("/proc/{}", pid);

        // cmdline
        if let Ok(cmdline) = std::fs::read_to_string(format!("{}/cmdline", proc_path)) {
            let cmd = cmdline.replace('\0', " ");
            report.push_str(&format!("📋 Command Line:\n   {}\n\n", cmd.trim()));
        }

        // status
        if let Ok(status) = std::fs::read_to_string(format!("{}/status", proc_path)) {
            report.push_str("📊 Status:\n");
            for line in status.lines().take(15) {
                report.push_str(&format!("   {}\n", line));
            }
            report.push('\n');
        }

        // cwd
        if let Ok(cwd) = std::fs::read_link(format!("{}/cwd", proc_path)) {
            report.push_str(&format!("📁 Working Directory:\n   {}\n\n", cwd.display()));
        }

        // exe
        if let Ok(exe) = std::fs::read_link(format!("{}/exe", proc_path)) {
            report.push_str(&format!("🔧 Executable:\n   {}\n\n", exe.display()));
        }

        // fd count
        if let Ok(entries) = std::fs::read_dir(format!("{}/fd", proc_path)) {
            let fd_count = entries.count();
            report.push_str(&format!("📂 Open File Descriptors: {}\n\n", fd_count));
        }

        // Network connections for this PID
        let ss_output = Command::new("ss")
            .arg("-tunap")
            .output();

        if let Ok(out) = ss_output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let pid_str = format!("pid={}", pid);
            let matching_lines: Vec<&str> = stdout
                .lines()
                .filter(|l| l.contains(&pid_str))
                .collect();

            if !matching_lines.is_empty() {
                report.push_str("🔗 Network Connections:\n");
                for line in matching_lines.iter().take(10) {
                    report.push_str(&format!("   {}\n", line));
                }
                report.push('\n');
            }
        }

        Ok(CallToolResult::success(vec![Content::text(report)]))
    }
}

#[tool_handler]
impl ServerHandler for ProcInfoServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Profile and analyze running processes using ps, pstree, lsof, and ss. \
                 Get detailed process listings, visualize process trees, examine network \
                 file descriptors, and correlate PIDs with network connections.".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let service = ProcInfoServer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            eprintln!("Error starting procinfo MCP server: {}", e);
        })?;

    info!("procinfo MCP server running over stdio");
    service.waiting().await?;
    Ok(())
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn truncate(input: &str, limit: usize) -> String {
    if input.len() <= limit {
        return input.to_string();
    }

    let mut truncated = input[..limit].to_string();
    truncated.push_str("\n...[truncated]...");
    truncated
}
