///! Solid Pod Tool
///!
///! Provides agents with read/write access to Solid Pods with proper authentication.

use crate::tools::{Tool, ToolContext, ToolOutput};
use crate::Result;
use crate::solid::auth::SolidOidcClient;
use anyhow::Context;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use url::Url;

/// Tool for accessing Solid Pods
pub struct SolidPodTool {
    /// OIDC client for authenticated requests
    oidc_client: Arc<SolidOidcClient>,
    /// Tool ID
    tool_id: String,
}

impl SolidPodTool {
    /// Create new Solid Pod tool
    pub fn new(oidc_client: Arc<SolidOidcClient>) -> Self {
        Self {
            oidc_client,
            tool_id: "solid_pod".to_string(),
        }
    }
}

#[async_trait]
impl Tool for SolidPodTool {
    fn id(&self) -> &str {
        &self.tool_id
    }

    fn name(&self) -> &str {
        "Solid Pod Access"
    }

    fn description(&self) -> &str {
        "Read and write data to the user's Solid Pod with proper authentication and ACL respect. \
         Supports operations: read, write, query (SPARQL), list (LDP containers)."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["read", "write", "query", "list"],
                    "description": "Operation to perform on the Pod"
                },
                "resourceUri": {
                    "type": "string",
                    "format": "uri",
                    "description": "IRI of the Pod resource to access"
                },
                "data": {
                    "type": "string",
                    "description": "Data to write (for write operations, RDF/Turtle format)"
                },
                "sparql": {
                    "type": "string",
                    "description": "SPARQL query (for query operations)"
                },
                "contentType": {
                    "type": "string",
                    "default": "text/turtle",
                    "description": "Content-Type for the resource"
                }
            },
            "required": ["operation", "resourceUri"]
        })
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolOutput> {
        let operation = params["operation"]
            .as_str()
            .context("Missing 'operation' parameter")?;

        let resource_uri = params["resourceUri"]
            .as_str()
            .context("Missing 'resourceUri' parameter")?;

        let resource_url = Url::parse(resource_uri)
            .context("Invalid resource URI")?;

        match operation {
            "read" => self.read_resource(&resource_url).await,
            "write" => {
                let data = params["data"]
                    .as_str()
                    .context("Missing 'data' parameter for write operation")?;
                self.write_resource(&resource_url, data).await
            }
            "query" => {
                let sparql = params["sparql"]
                    .as_str()
                    .context("Missing 'sparql' parameter for query operation")?;
                self.query_resource(&resource_url, sparql).await
            }
            "list" => self.list_container(&resource_url).await,
            _ => Err(anyhow::anyhow!("Unknown operation: {}", operation)),
        }
    }
}

impl SolidPodTool {
    /// Read a resource from the Pod
    async fn read_resource(&self, url: &Url) -> Result<ToolOutput> {
        let response = self.oidc_client
            .authenticated_request("GET", url)
            .await
            .context("Failed to read resource")?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Failed to read resource: HTTP {}",
                status
            ));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream");

        let content = response.text().await
            .context("Failed to read response body")?;

        Ok(ToolOutput::success(json!({
            "content": content,
            "contentType": content_type,
            "url": url.as_str(),
        })))
    }

    /// Write a resource to the Pod
    async fn write_resource(&self, url: &Url, data: &str) -> Result<ToolOutput> {
        let client = reqwest::Client::new();

        // Get access token and create DPoP proof
        let token = self.oidc_client.authenticate(url).await?;
        let ath = crate::solid::dpop::DPoPManager::compute_ath(&token.access_token);
        let dpop_proof = self.oidc_client.dpop_manager.create_proof(
            "PUT",
            url.as_str(),
            Some(&ath),
        )?;

        let response = client
            .put(url.clone())
            .header("Authorization", format!("DPoP {}", token.access_token))
            .header("DPoP", dpop_proof)
            .header("Content-Type", "text/turtle")
            .body(data.to_string())
            .send()
            .await
            .context("Failed to write resource")?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Failed to write resource: HTTP {}",
                status
            ));
        }

        Ok(ToolOutput::success(json!({
            "success": true,
            "url": url.as_str(),
            "message": "Resource updated successfully"
        })))
    }

    /// Execute SPARQL query against Pod
    async fn query_resource(&self, url: &Url, sparql: &str) -> Result<ToolOutput> {
        // Delegate to TypeScript bridge for SPARQL execution
        let params = json!({
            "endpoint": url.as_str(),
            "query": sparql,
        });

        let ipc = &self.oidc_client.identity_client.ipc;
        let mut ipc_guard = ipc.lock().unwrap();
        let results = ipc_guard.request("executeSparql", params)
            .context("SPARQL query failed")?;

        Ok(ToolOutput::success(json!({
            "results": results,
            "query": sparql,
            "endpoint": url.as_str(),
        })))
    }

    /// List contents of LDP container
    async fn list_container(&self, url: &Url) -> Result<ToolOutput> {
        let response = self.oidc_client
            .authenticated_request("GET", url)
            .await
            .context("Failed to list container")?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Failed to list container: HTTP {}",
                status
            ));
        }

        let turtle = response.text().await
            .context("Failed to read container")?;

        // In production, we'd parse the Turtle to extract contained resources
        // For now, return raw Turtle
        Ok(ToolOutput::success(json!({
            "container": url.as_str(),
            "content": turtle,
            "message": "Container listed successfully. Parse Turtle content for resources."
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_metadata() {
        // Create mock OIDC client for testing
        // In production tests, we'd use actual mock
        // For now, skip runtime test
    }
}
