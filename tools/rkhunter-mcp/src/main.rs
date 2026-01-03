use rmcp::{
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::io::stdio,
    ServerHandler, ServiceExt,
};
use rmcp::serde_json;
use rmcp::model::ErrorData;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone)]
pub struct RkhunterServer {
    inner: Arc<Mutex<()>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl RkhunterServer {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(())),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Run rkhunter --checkall with sudo and summarize any findings")]
    async fn rkhunter_scan(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        // Serialize execution to avoid overlapping scans.
        let _guard = self.inner.lock().await;

        // Extract flags from params, default to ["--checkall", "--skip-keypress"]
        let flags = params
            .get("flags")
            .and_then(|v| v.as_array())
            .map(|values| {
                values
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(|| vec![
                "--checkall".to_string(),
                "--skip-keypress".to_string(),
                "--report-warnings-only".to_string(),
            ]);

        // Run rkhunter with sudo
        let mut cmd = Command::new("sudo");
        cmd.arg("rkhunter");
        cmd.args(&flags);

        let output = cmd.output();
        let output = match output {
            Ok(out) => out,
            Err(err) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to execute 'sudo rkhunter': {}. Ensure:\n\
                     1. rkhunter is installed (apt-get install rkhunter)\n\
                     2. sudo is available\n\
                     3. User has passwordless sudo access for rkhunter OR run this as root\n\
                     4. rkhunter database is updated (sudo rkhunter --update)",
                    err
                ))]));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let (summary, findings) = summarize_rkhunter(&stdout);

        let mut content = vec![Content::text(summary)];

        if !findings.is_empty() {
            let bullet_list = findings.join("\n- ");
            content.push(Content::text(format!(
                "Flagged issues:\n- {}",
                bullet_list
            )));
        }

        if !stdout.trim().is_empty() {
            content.push(Content::text(format!(
                "rkhunter stdout (truncated):\n{}",
                truncate(&stdout, 8000)
            )));
        }

        if !stderr.trim().is_empty() {
            content.push(Content::text(format!("rkhunter stderr:\n{}", stderr)));
        }

        // rkhunter returns non-zero on warnings, which is expected
        let result = CallToolResult::success(content);
        Ok(result)
    }
}

#[tool_handler]
impl ServerHandler for RkhunterServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Run 'sudo rkhunter --checkall' and return a concise summary of any warnings. \
                 Requires passwordless sudo or root access.".into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let service = RkhunterServer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            eprintln!("Error starting rkhunter MCP server: {}", e);
        })?;

    info!("rkhunter MCP server running over stdio");
    service.waiting().await?;
    Ok(())
}

fn summarize_rkhunter(stdout: &str) -> (String, Vec<String>) {
    let mut findings = Vec::new();
    let mut warning_count = 0;
    let mut critical_count = 0;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let normalized = trimmed.to_lowercase();

        // Check for security issues
        if normalized.contains("warning") && normalized.contains("[") {
            warning_count += 1;
            findings.push(format!("🟡 {}", trimmed));
        } else if normalized.contains("infection")
            || normalized.contains("rootkit")
            || normalized.contains("backdoor")
            || (normalized.contains("found") && !normalized.contains("not found"))
        {
            critical_count += 1;
            findings.push(format!("🔴 {}", trimmed));
        } else if normalized.contains("properties have changed")
            || normalized.contains("file property")
            || normalized.contains("inode changed")
        {
            warning_count += 1;
            findings.push(format!("🟡 {}", trimmed));
        }
    }

    let summary = if findings.is_empty() {
        "✅ rkhunter scan completed. No warnings or infections detected.".to_string()
    } else if critical_count > 0 {
        format!(
            "🔴 CRITICAL: rkhunter detected {} critical issue(s) and {} warning(s). Immediate review required!",
            critical_count, warning_count
        )
    } else if warning_count > 0 {
        format!(
            "🟡 rkhunter flagged {} warning(s). Review recommended.",
            warning_count
        )
    } else {
        format!(
            "ℹ️  rkhunter completed with {} informational finding(s).",
            findings.len()
        )
    };

    (summary, findings)
}

fn truncate(input: &str, limit: usize) -> String {
    if input.len() <= limit {
        return input.to_string();
    }

    let mut truncated = input[..limit].to_string();
    truncated.push_str("\n...[truncated]...");
    truncated
}
