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
pub struct LynisServer {
    inner: Arc<Mutex<()>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl LynisServer {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(())),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Run lynis audit system with sudo and summarize findings")]
    async fn lynis_scan(
        &self,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<CallToolResult, ErrorData> {
        // Serialize execution to avoid overlapping scans.
        let _guard = self.inner.lock().await;

        // Extract flags from params, default to ["audit", "system"]
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
                "audit".to_string(),
                "system".to_string(),
                "--quick".to_string(),
            ]);

        // Run lynis with sudo
        let mut cmd = Command::new("sudo");
        cmd.arg("lynis");
        cmd.args(&flags);

        let output = cmd.output();
        let output = match output {
            Ok(out) => out,
            Err(err) => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Failed to execute 'sudo lynis': {}. Ensure:\n\
                     1. lynis is installed (apt-get install lynis)\n\
                     2. sudo is available\n\
                     3. User has passwordless sudo access for lynis OR run this as root",
                    err
                ))]));
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let (summary, findings, suggestions, hardening_index) = summarize_lynis(&stdout);

        let mut content = vec![Content::text(summary)];

        // Add hardening index if found
        if let Some(index) = hardening_index {
            content.push(Content::text(format!(
                "🛡️  System Hardening Index: {}",
                index
            )));
        }

        if !findings.is_empty() {
            let bullet_list = findings.join("\n- ");
            content.push(Content::text(format!(
                "Security Findings:\n- {}",
                bullet_list
            )));
        }

        if !suggestions.is_empty() {
            let suggestion_list = suggestions.join("\n- ");
            content.push(Content::text(format!(
                "Suggestions:\n- {}",
                suggestion_list
            )));
        }

        if !stdout.trim().is_empty() {
            content.push(Content::text(format!(
                "lynis stdout (truncated):\n{}",
                truncate(&stdout, 10000)
            )));
        }

        if !stderr.trim().is_empty() {
            content.push(Content::text(format!("lynis stderr:\n{}", stderr)));
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
impl ServerHandler for LynisServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Run 'sudo lynis audit system' and return a comprehensive security assessment. \
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

    let service = LynisServer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            eprintln!("Error starting lynis MCP server: {}", e);
        })?;

    info!("lynis MCP server running over stdio");
    service.waiting().await?;
    Ok(())
}

fn summarize_lynis(stdout: &str) -> (String, Vec<String>, Vec<String>, Option<String>) {
    let mut findings = Vec::new();
    let mut suggestions = Vec::new();
    let mut warning_count = 0;
    let mut suggestion_count = 0;
    let mut hardening_index = None;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let normalized = trimmed.to_lowercase();

        // Extract hardening index
        if normalized.contains("hardening index") && normalized.contains("[") {
            if let Some(start) = trimmed.find('[') {
                if let Some(end) = trimmed.find(']') {
                    hardening_index = Some(trimmed[start..=end].to_string());
                }
            }
        }

        // Check for warnings and issues
        if normalized.contains("warning") && normalized.contains("[") {
            warning_count += 1;
            findings.push(format!("🟡 {}", trimmed));
        } else if normalized.contains("suggestion") && normalized.contains("[") {
            suggestion_count += 1;
            suggestions.push(format!("💡 {}", trimmed));
        } else if normalized.contains("vulnerable")
            || normalized.contains("weak")
            || normalized.contains("not found")
            || normalized.contains("outdated")
        {
            warning_count += 1;
            findings.push(format!("🟡 {}", trimmed));
        } else if normalized.contains("recommendation") {
            suggestions.push(format!("💡 {}", trimmed));
        }
    }

    let summary = if warning_count == 0 && suggestion_count == 0 {
        "✅ lynis audit completed. System appears well-configured with no major warnings.".to_string()
    } else if warning_count > 0 {
        format!(
            "🟡 lynis found {} warning(s) and {} suggestion(s). Review recommended for security hardening.",
            warning_count, suggestion_count
        )
    } else {
        format!(
            "ℹ️  lynis completed with {} suggestion(s) for improvement.",
            suggestion_count
        )
    };

    (summary, findings, suggestions, hardening_index)
}

fn truncate(input: &str, limit: usize) -> String {
    if input.len() <= limit {
        return input.to_string();
    }

    let mut truncated = input[..limit].to_string();
    truncated.push_str("\n...[truncated]...");
    truncated
}
