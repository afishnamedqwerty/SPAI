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
pub struct ChkrootkitServer {
    inner: Arc<Mutex<()>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl ChkrootkitServer {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(())),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Run chkrootkit with sudo -x and summarize any findings")]
    async fn chkrootkit_scan(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        // Serialize execution to avoid overlapping scans.
        let _guard = self.inner.lock().await;

        // Extract flags from params, default to ["-x"] for extended mode
        let flags = params
            .get("flags")
            .and_then(|v| v.as_array())
            .map(|values| {
                values
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<String>>()
            })
            .unwrap_or_else(|| vec!["-x".to_string()]);

        // Run chkrootkit with sudo
        let mut cmd = Command::new("sudo");
        cmd.arg("chkrootkit");
        cmd.args(&flags);

        let output = cmd.output();
        let output = match output {
            Ok(out) => out,
            Err(err) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to execute 'sudo chkrootkit': {}. Ensure:\n\
                     1. chkrootkit is installed (apt-get install chkrootkit)\n\
                     2. sudo is available\n\
                     3. User has passwordless sudo access for chkrootkit OR run this as root",
                    err
                ))]));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let (summary, findings) = summarize_chkrootkit(&stdout);

        let mut content = vec![Content::text(summary)];

        if !findings.is_empty() {
            let bullet_list = findings.join("\n- ");
            content.push(Content::text(format!(
                "Flagged lines:\n- {}",
                bullet_list
            )));
        }

        if !stdout.trim().is_empty() {
            content.push(Content::text(format!(
                "chkrootkit stdout (truncated):\n{}",
                truncate(&stdout, 8000)
            )));
        }

        if !stderr.trim().is_empty() {
            content.push(Content::text(format!("chkrootkit stderr:\n{}", stderr)));
        }

        let result = if output.status.success() {
            CallToolResult::success(content)
        } else {
            CallToolResult::error(content)
        };

        Ok(result)
    }
}

#[tool_handler]
impl ServerHandler for ChkrootkitServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Run 'sudo chkrootkit -x' and return a concise summary of any flagged lines. \
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

    let service = ChkrootkitServer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            eprintln!("Error starting chkrootkit MCP server: {}", e);
        })?;

    info!("chkrootkit MCP server running over stdio");
    service.waiting().await?;
    Ok(())
}

fn summarize_chkrootkit(stdout: &str) -> (String, Vec<String>) {
    let mut findings = Vec::new();
    let mut warning_count = 0;
    let mut infected_count = 0;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let normalized = trimmed.to_lowercase();

        // Check for security issues
        if normalized.contains("infected") {
            infected_count += 1;
            findings.push(format!("🔴 {}", trimmed));
        } else if normalized.contains("warning")
            || normalized.contains("vulnerable")
            || normalized.contains("suspicious")
            || normalized.contains("rootkit")
            || normalized.contains("malware")
            || normalized.contains("not found")
            || normalized.contains("!!!")
        {
            warning_count += 1;
            findings.push(format!("🟡 {}", trimmed));
        } else if normalized.contains("found") && !normalized.contains("not found") {
            // Potential positive finding
            findings.push(format!("ℹ️  {}", trimmed));
        }
    }

    let summary = if findings.is_empty() {
        "✅ chkrootkit scan completed. No obvious infections or warnings detected.".to_string()
    } else if infected_count > 0 {
        format!(
            "🔴 CRITICAL: chkrootkit detected {} infection(s) and {} warning(s). Immediate review required!",
            infected_count, warning_count
        )
    } else if warning_count > 0 {
        format!(
            "🟡 chkrootkit flagged {} warning(s). Review recommended.",
            warning_count
        )
    } else {
        format!(
            "ℹ️  chkrootkit completed with {} informational finding(s).",
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
