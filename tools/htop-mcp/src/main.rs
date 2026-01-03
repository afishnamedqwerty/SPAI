//! htop MCP Server - Process monitoring and suspicious activity detection
//!
//! This MCP server provides tools for monitoring system processes using the sysinfo crate.
//! It does NOT require the htop binary to be installed - it's pure Rust.

use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::io::stdio,
    ServerHandler, ServiceExt,
};
use rmcp::model::ErrorData;
use rmcp::serde_json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sysinfo::{Pid, System};
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone)]
pub struct HtopServer {
    inner: Arc<Mutex<()>>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessInfo {
    pid: u32,
    name: String,
    cpu_usage: f32,
    memory_mb: u64,
    memory_percent: f32,
    status: String,
    parent_pid: Option<u32>,
    exe_path: String,
    cmd: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SystemStats {
    total_cpu_usage: f32,
    cpu_count: usize,
    total_memory_mb: u64,
    used_memory_mb: u64,
    memory_usage_percent: f32,
    total_swap_mb: u64,
    used_swap_mb: u64,
    swap_usage_percent: f32,
    uptime_seconds: u64,
    process_count: usize,
}

#[tool_router]
impl HtopServer {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(())),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List running processes sorted by CPU or memory usage. Returns top N processes with detailed information.")]
    async fn list_processes(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.inner.lock().await;

        // Extract parameters
        let sort_by = params
            .get("sort_by")
            .and_then(|v| v.as_str())
            .unwrap_or("cpu");
        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(20) as usize;

        let mut sys = System::new_all();
        sys.refresh_all();

        let total_memory = sys.total_memory();

        let mut processes: Vec<ProcessInfo> = sys
            .processes()
            .iter()
            .map(|(pid, process)| {
                let memory = process.memory();
                let memory_percent = if total_memory > 0 {
                    (memory as f32 / total_memory as f32) * 100.0
                } else {
                    0.0
                };

                ProcessInfo {
                    pid: pid.as_u32(),
                    name: process.name().to_string_lossy().to_string(),
                    cpu_usage: process.cpu_usage(),
                    memory_mb: memory / (1024 * 1024),
                    memory_percent,
                    status: format!("{:?}", process.status()),
                    parent_pid: process.parent().map(|p| p.as_u32()),
                    exe_path: process
                        .exe()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "N/A".to_string()),
                    cmd: process.cmd().iter().map(|s| s.to_string_lossy().to_string()).collect(),
                }
            })
            .collect();

        // Sort processes
        match sort_by {
            "memory" => processes.sort_by(|a, b| b.memory_mb.cmp(&a.memory_mb)),
            _ => processes.sort_by(|a, b| {
                b.cpu_usage
                    .partial_cmp(&a.cpu_usage)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }

        processes.truncate(limit);

        // Format output
        let mut output = format!(
            "Top {} processes by {}:\n\n",
            processes.len(),
            sort_by.to_uppercase()
        );
        output.push_str(&format!(
            "{:<8} {:<25} {:>8} {:>10} {:>8} {}\n",
            "PID", "NAME", "CPU%", "MEM(MB)", "MEM%", "STATUS"
        ));
        output.push_str(&"-".repeat(80));
        output.push('\n');

        for proc in &processes {
            output.push_str(&format!(
                "{:<8} {:<25} {:>7.1}% {:>9} {:>7.1}% {}\n",
                proc.pid,
                truncate_string(&proc.name, 25),
                proc.cpu_usage,
                proc.memory_mb,
                proc.memory_percent,
                proc.status
            ));
        }

        let json_data = serde_json::to_string_pretty(&processes)
            .unwrap_or_else(|_| "[]".to_string());

        Ok(CallToolResult::success(vec![
            Content::text(output),
            Content::text(format!("\nDetailed JSON data:\n{}", json_data)),
        ]))
    }

    #[tool(description = "Get detailed information about a specific process by PID")]
    async fn get_process_info(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.inner.lock().await;

        let pid = params
            .get("pid")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                ErrorData::invalid_request("Missing required parameter: pid", None)
            })?;

        let mut sys = System::new_all();
        sys.refresh_all();

        let sysinfo_pid = Pid::from_u32(pid as u32);
        let process = sys.process(sysinfo_pid).ok_or_else(|| {
            ErrorData::invalid_request(format!("Process with PID {} not found", pid), None)
        })?;

        let total_memory = sys.total_memory();
        let memory = process.memory();
        let memory_percent = if total_memory > 0 {
            (memory as f32 / total_memory as f32) * 100.0
        } else {
            0.0
        };

        let info = ProcessInfo {
            pid: sysinfo_pid.as_u32(),
            name: process.name().to_string_lossy().to_string(),
            cpu_usage: process.cpu_usage(),
            memory_mb: memory / (1024 * 1024),
            memory_percent,
            status: format!("{:?}", process.status()),
            parent_pid: process.parent().map(|p| p.as_u32()),
            exe_path: process
                .exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            cmd: process.cmd().iter().map(|s| s.to_string_lossy().to_string()).collect(),
        };

        let output = format!(
            "Process Details (PID {}):\n\n\
             Name:           {}\n\
             CPU Usage:      {:.1}%\n\
             Memory:         {} MB ({:.1}%)\n\
             Status:         {}\n\
             Parent PID:     {:?}\n\
             Executable:     {}\n\
             Command Line:   {}\n",
            info.pid,
            info.name,
            info.cpu_usage,
            info.memory_mb,
            info.memory_percent,
            info.status,
            info.parent_pid,
            info.exe_path,
            info.cmd.join(" ")
        );

        let json_data = serde_json::to_string_pretty(&info)
            .unwrap_or_else(|_| "{}".to_string());

        Ok(CallToolResult::success(vec![
            Content::text(output),
            Content::text(format!("\nJSON data:\n{}", json_data)),
        ]))
    }

    #[tool(description = "Get overall system statistics including CPU, memory, swap usage, and process count")]
    async fn get_system_stats(
        &self,
        _params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.inner.lock().await;

        let mut sys = System::new_all();
        sys.refresh_all();

        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();
        let total_swap = sys.total_swap();
        let used_swap = sys.used_swap();

        let memory_percent = if total_memory > 0 {
            (used_memory as f32 / total_memory as f32) * 100.0
        } else {
            0.0
        };

        let swap_percent = if total_swap > 0 {
            (used_swap as f32 / total_swap as f32) * 100.0
        } else {
            0.0
        };

        let stats = SystemStats {
            total_cpu_usage: sys.global_cpu_usage(),
            cpu_count: sys.cpus().len(),
            total_memory_mb: total_memory / (1024 * 1024),
            used_memory_mb: used_memory / (1024 * 1024),
            memory_usage_percent: memory_percent,
            total_swap_mb: total_swap / (1024 * 1024),
            used_swap_mb: used_swap / (1024 * 1024),
            swap_usage_percent: swap_percent,
            uptime_seconds: System::uptime(),
            process_count: sys.processes().len(),
        };

        let output = format!(
            "System Statistics:\n\n\
             CPU:\n\
             - Total Usage:    {:.1}%\n\
             - CPU Count:      {}\n\n\
             Memory:\n\
             - Total:          {} MB\n\
             - Used:           {} MB ({:.1}%)\n\n\
             Swap:\n\
             - Total:          {} MB\n\
             - Used:           {} MB ({:.1}%)\n\n\
             System:\n\
             - Uptime:         {} seconds ({} hours)\n\
             - Process Count:  {}\n",
            stats.total_cpu_usage,
            stats.cpu_count,
            stats.total_memory_mb,
            stats.used_memory_mb,
            stats.memory_usage_percent,
            stats.total_swap_mb,
            stats.used_swap_mb,
            stats.swap_usage_percent,
            stats.uptime_seconds,
            stats.uptime_seconds / 3600,
            stats.process_count
        );

        let json_data = serde_json::to_string_pretty(&stats)
            .unwrap_or_else(|_| "{}".to_string());

        Ok(CallToolResult::success(vec![
            Content::text(output),
            Content::text(format!("\nJSON data:\n{}", json_data)),
        ]))
    }

    #[tool(description = "Identify potentially suspicious processes based on heuristics (high CPU/memory usage, unusual names, hidden processes)")]
    async fn find_suspicious_processes(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        let _guard = self.inner.lock().await;

        // Extract thresholds
        let high_cpu_threshold = params
            .get("high_cpu_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(80.0) as f32;
        let high_memory_threshold = params
            .get("high_memory_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(80.0) as f32;

        let mut sys = System::new_all();
        sys.refresh_all();

        let total_memory = sys.total_memory();
        let mut suspicious_processes = Vec::new();

        for (pid, process) in sys.processes() {
            let name = process.name().to_string_lossy().to_string();
            let cpu = process.cpu_usage();
            let memory = process.memory();
            let mem_percent = if total_memory > 0 {
                (memory as f32 / total_memory as f32) * 100.0
            } else {
                0.0
            };

            let mut reasons = Vec::new();

            // Heuristic checks
            if cpu > high_cpu_threshold {
                reasons.push(format!("High CPU usage: {:.1}%", cpu));
            }
            if mem_percent > high_memory_threshold {
                reasons.push(format!("High memory usage: {:.1}%", mem_percent));
            }

            // Check for suspicious naming patterns
            if name.starts_with('.') && name.len() > 1 {
                reasons.push("Hidden process (starts with '.')".to_string());
            }
            if name.contains("tmp") || name.contains("...") {
                reasons.push(format!("Suspicious name pattern: '{}'", name));
            }
            if name.len() > 4 && name.chars().all(|c| c.is_ascii_hexdigit()) {
                reasons.push("Process name is all hexadecimal characters".to_string());
            }

            if !reasons.is_empty() {
                suspicious_processes.push((
                    ProcessInfo {
                        pid: pid.as_u32(),
                        name: name.clone(),
                        cpu_usage: cpu,
                        memory_mb: memory / (1024 * 1024),
                        memory_percent: mem_percent,
                        status: format!("{:?}", process.status()),
                        parent_pid: process.parent().map(|p| p.as_u32()),
                        exe_path: process
                            .exe()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "N/A".to_string()),
                        cmd: process.cmd().iter().map(|s| s.to_string_lossy().to_string()).collect(),
                    },
                    reasons,
                ));
            }
        }

        let mut output = format!(
            "Suspicious Process Detection (CPU>{:.0}%, MEM>{:.0}%):\n\n",
            high_cpu_threshold, high_memory_threshold
        );

        if suspicious_processes.is_empty() {
            output.push_str("✅ No suspicious processes detected based on heuristics.\n");
        } else {
            output.push_str(&format!(
                "⚠️  Found {} potentially suspicious process(es):\n\n",
                suspicious_processes.len()
            ));

            for (proc, reasons) in &suspicious_processes {
                output.push_str(&format!(
                    "PID {}: {} (CPU: {:.1}%, MEM: {} MB / {:.1}%)\n",
                    proc.pid, proc.name, proc.cpu_usage, proc.memory_mb, proc.memory_percent
                ));
                output.push_str(&format!("  Executable: {}\n", proc.exe_path));
                output.push_str(&format!("  Command: {}\n", proc.cmd.join(" ")));
                output.push_str("  Reasons:\n");
                for reason in reasons {
                    output.push_str(&format!("    - {}\n", reason));
                }
                output.push('\n');
            }
        }

        let json_data = serde_json::to_string_pretty(
            &suspicious_processes
                .iter()
                .map(|(p, r)| {
                    serde_json::json!({
                        "process": p,
                        "reasons": r
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".to_string());

        Ok(CallToolResult::success(vec![
            Content::text(output),
            Content::text(format!("\nDetailed JSON data:\n{}", json_data)),
        ]))
    }
}

#[tool_handler]
impl ServerHandler for HtopServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Monitor system processes and resource usage. List running processes sorted by CPU or \
                 memory usage, identify suspicious processes, and get detailed system statistics. \
                 This uses pure Rust (sysinfo crate) and does NOT require htop to be installed."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let service = HtopServer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            eprintln!("Error starting htop MCP server: {}", e);
        })?;

    info!("htop MCP server running over stdio");
    service.waiting().await?;
    Ok(())
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
