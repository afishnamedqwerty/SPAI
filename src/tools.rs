//! Tool trait and implementations

use crate::error::Result;
use crate::types::AgentId;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "mcp-tools")]
use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, PaginatedRequestParam, RawContent,
    },
    service::ServiceExt,
    transport::child_process::TokioChildProcess,
};
#[cfg(feature = "mcp-tools")]
use tokio::process::Command;

/// Context provided to tools during execution
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// ID of the agent executing the tool
    pub agent_id: AgentId,
    /// Additional context data
    pub data: HashMap<String, Value>,
}

impl ToolContext {
    /// Create a new tool context
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            data: HashMap::new(),
        }
    }

    /// Add data to the context
    pub fn with_data(mut self, key: impl Into<String>, value: Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }

    /// Get data from the context
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }
}

/// Output from a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Whether the tool execution was successful
    pub success: bool,
    /// Output content
    pub content: String,
    /// Optional structured data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    /// Optional error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolOutput {
    /// Create a successful tool output
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            success: true,
            content: content.into(),
            data: None,
            error: None,
        }
    }

    /// Create a successful tool output with data
    pub fn success_with_data(content: impl Into<String>, data: Value) -> Self {
        Self {
            success: true,
            content: content.into(),
            data: Some(data),
            error: None,
        }
    }

    /// Create a failed tool output
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            content: String::new(),
            data: None,
            error: Some(error.into()),
        }
    }

    /// Create a failed tool output with content
    pub fn failure_with_content(content: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            success: false,
            content: content.into(),
            data: None,
            error: Some(error.into()),
        }
    }
}

/// JSON Schema for tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    /// Schema type
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Schema properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Value>>,
    /// Required properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    /// Additional properties
    #[serde(flatten)]
    pub additional: HashMap<String, Value>,
}

impl JsonSchema {
    /// Create an empty object schema
    pub fn empty() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: None,
            required: None,
            additional: HashMap::new(),
        }
    }

    /// Create an object schema with properties
    pub fn object(properties: HashMap<String, Value>) -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: None,
            additional: HashMap::new(),
        }
    }

    /// Set required properties
    pub fn with_required(mut self, required: Vec<String>) -> Self {
        self.required = Some(required);
        self
    }
}

/// Tool trait defining the interface for agent capabilities
#[async_trait]
pub trait Tool: Send + Sync {
    /// Unique identifier for this tool
    fn id(&self) -> &str;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Description for LLM function calling
    fn description(&self) -> &str;

    /// JSON Schema for input parameters
    fn input_schema(&self) -> JsonSchema;

    /// Execute the tool with given parameters
    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolOutput>;

    /// Optional: Validate parameters before execution
    fn validate(&self, _params: &Value) -> Result<()> {
        Ok(())
    }

    /// Optional: Estimated execution time for planning
    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(1)
    }
}

/// A simple echo tool for testing
pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn id(&self) -> &str {
        "echo"
    }

    fn name(&self) -> &str {
        "Echo"
    }

    fn description(&self) -> &str {
        "Echoes back the input message"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "message".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The message to echo back"
            }),
        );

        JsonSchema::object(properties).with_required(vec!["message".to_string()])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("(no message)");

        Ok(ToolOutput::success(format!("Echo: {}", message)))
    }
}

/// Calculator tool for basic arithmetic
pub struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn id(&self) -> &str {
        "calculator"
    }

    fn name(&self) -> &str {
        "Calculator"
    }

    fn description(&self) -> &str {
        "Performs basic arithmetic operations (add, subtract, multiply, divide)"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "operation".to_string(),
            serde_json::json!({
                "type": "string",
                "enum": ["add", "subtract", "multiply", "divide"],
                "description": "The operation to perform"
            }),
        );
        properties.insert(
            "a".to_string(),
            serde_json::json!({
                "type": "number",
                "description": "First operand"
            }),
        );
        properties.insert(
            "b".to_string(),
            serde_json::json!({
                "type": "number",
                "description": "Second operand"
            }),
        );

        JsonSchema::object(properties)
            .with_required(vec!["operation".to_string(), "a".to_string(), "b".to_string()])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let operation = params
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::error::Error::InvalidInput("Missing 'operation'".to_string()))?;

        let a = params
            .get("a")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| crate::error::Error::InvalidInput("Missing 'a'".to_string()))?;

        let b = params
            .get("b")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| crate::error::Error::InvalidInput("Missing 'b'".to_string()))?;

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return Ok(ToolOutput::failure("Division by zero"));
                }
                a / b
            }
            _ => {
                return Ok(ToolOutput::failure(format!(
                    "Unknown operation: {}",
                    operation
                )))
            }
        };

        Ok(ToolOutput::success_with_data(
            format!("{} {} {} = {}", a, operation, b, result),
            serde_json::json!({ "result": result }),
        ))
    }
}

/// Create an echo tool
pub fn echo_tool() -> Arc<dyn Tool> {
    Arc::new(EchoTool)
}

/// Create a calculator tool
pub fn calculator_tool() -> Arc<dyn Tool> {
    Arc::new(CalculatorTool)
}

/// MCP tool wrapper that launches an MCP server over stdio as a subprocess.
/// Requires the `mcp-tools` feature.
#[cfg(feature = "mcp-tools")]
pub struct McpSubprocessTool {
    id: String,
    name: String,
    description: String,
    input_schema: JsonSchema,
    command: PathBuf,
    args: Vec<String>,
    mcp_tool_name: String,
}

#[cfg(feature = "mcp-tools")]
impl McpSubprocessTool {
    /// Create a new MCP subprocess tool wrapper.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        mcp_tool_name: impl Into<String>,
        command: impl Into<PathBuf>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            input_schema: JsonSchema::empty(),
            command: command.into(),
            args: Vec::new(),
            mcp_tool_name: mcp_tool_name.into(),
        }
    }

    /// Set CLI arguments for launching the subprocess (e.g., `["--foo", "bar"]`).
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Override the advertised input schema (propagated to the LLM).
    pub fn with_schema(mut self, schema: JsonSchema) -> Self {
        self.input_schema = schema;
        self
    }
}

#[cfg(feature = "mcp-tools")]
#[async_trait]
impl Tool for McpSubprocessTool {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn input_schema(&self) -> JsonSchema {
        self.input_schema.clone()
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let command = self.command.clone();
        let args = self.args.clone();
        let command_str = command
            .to_str()
            .ok_or_else(|| crate::error::Error::config("Invalid MCP command path"))?
            .to_string();

        let args_map = match params {
            Value::Object(map) => map,
            _ => {
                return Err(crate::error::Error::InvalidInput(
                    "MCP tool expects an object payload".to_string(),
                ))
            }
        };

        let mut cmd = Command::new(&command_str);
        cmd.args(&args);

        let transport = TokioChildProcess::new(cmd)
            .map_err(|e| crate::error::Error::tool_execution(self.id(), e.to_string()))?;

        let service = ()
            .serve(transport)
            .await
            .map_err(|e| crate::error::Error::tool_execution(self.id(), e.to_string()))?;

        // Optional sanity check to ensure the tool exists.
        let _ = service
            .list_tools(Some(PaginatedRequestParam::default()))
            .await
            .map_err(|e| crate::error::Error::tool_execution(self.id(), e.to_string()));

        let call_result = service
            .call_tool(CallToolRequestParam {
                name: self.mcp_tool_name.clone().into(),
                arguments: Some(args_map),
            })
            .await
            .map_err(|e| crate::error::Error::tool_execution(self.id(), e.to_string()))?;

        // Best-effort shutdown
        let _ = service.cancel().await;

        Ok(convert_mcp_result(call_result))
    }
}

/// Convert an MCP tool call result into the framework's `ToolOutput`.
#[cfg(feature = "mcp-tools")]
fn convert_mcp_result(result: CallToolResult) -> ToolOutput {
    let mut text_parts: Vec<String> = Vec::new();

    for item in &result.content {
        match &item.raw {
            RawContent::Text(text) => text_parts.push(text.text.clone()),
            RawContent::Image(image) => text_parts.push(format!(
                "MCP tool returned an image (mime: {})",
                image.mime_type
            )),
            RawContent::Resource(resource) => {
                text_parts.push(format!("MCP tool returned a resource: {:?}", resource))
            }
            RawContent::Audio(_) => text_parts.push("MCP tool returned audio content".to_string()),
            RawContent::ResourceLink(link) => {
                text_parts.push(format!("MCP tool returned resource link: {:?}", link))
            }
        }
    }

    let content = if text_parts.is_empty() {
        "MCP tool completed without textual output".to_string()
    } else {
        text_parts.join("\n")
    };

    let data = serde_json::to_value(&result.content).ok();

    if result.is_error.unwrap_or(false) {
        ToolOutput::failure_with_content(content, "MCP tool reported an error")
    } else {
        ToolOutput {
            success: true,
            content,
            data,
            error: None,
        }
    }
}
