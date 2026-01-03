///! IPC Channel for TypeScript Bridge Communication
///!
///! Implements JSON-RPC over stdio for communicating with the Node.js subprocess.

use crate::Result;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tokio::process::{ChildStdin as TokioChildStdin, ChildStdout as TokioChildStdout};

/// IPC channel for communicating with TypeScript bridge
pub struct IpcChannel {
    /// Child process handle
    child: Child,
    /// stdin for sending requests
    stdin: ChildStdin,
    /// stdout for receiving responses
    stdout: BufReader<ChildStdout>,
    /// Request ID counter
    request_id: AtomicU64,
}

impl IpcChannel {
    /// Spawn TypeScript bridge and create IPC channel
    pub fn spawn(bridge_path: PathBuf) -> Result<Self> {
        // Verify bridge exists
        if !bridge_path.exists() {
            return Err(anyhow::anyhow!(
                "TypeScript bridge not found at: {}. Run 'npm run build' in tools/solid-identity-bridge",
                bridge_path.display()
            ));
        }

        // Spawn Node.js process
        let mut child = Command::new("node")
            .arg(&bridge_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // Log errors to parent's stderr
            .spawn()
            .context("Failed to spawn TypeScript bridge")?;

        let stdin = child.stdin.take()
            .context("Failed to get stdin")?;

        let stdout = child.stdout.take()
            .context("Failed to get stdout")?;

        let stdout = BufReader::new(stdout);

        Ok(Self {
            child,
            stdin,
            stdout,
            request_id: AtomicU64::new(1),
        })
    }

    /// Send JSON-RPC request and wait for response
    pub fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        // Serialize and send request
        let request_json = serde_json::to_string(&request)?;
        writeln!(self.stdin, "{}", request_json)
            .context("Failed to write to bridge stdin")?;

        self.stdin.flush()
            .context("Failed to flush stdin")?;

        // Read response line
        let mut response_line = String::new();
        self.stdout.read_line(&mut response_line)
            .context("Failed to read from bridge stdout")?;

        // Parse response
        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .context("Failed to parse bridge response")?;

        // Check for errors
        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "Bridge error (code {}): {}",
                error.code,
                error.message
            ));
        }

        // Return result
        response.result.context("No result in response")
    }

    /// Shutdown the bridge gracefully
    pub fn shutdown(mut self) -> Result<()> {
        // Send SIGTERM to child
        self.child.kill()
            .context("Failed to kill bridge process")?;

        // Wait for child to exit
        self.child.wait()
            .context("Failed to wait for bridge exit")?;

        Ok(())
    }
}

impl Drop for IpcChannel {
    fn drop(&mut self) {
        // Attempt to kill child on drop
        let _ = self.child.kill();
    }
}

/// JSON-RPC request
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    params: Value,
}

/// JSON-RPC response
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

/// JSON-RPC error object
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(default)]
    data: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires TypeScript bridge to be built
    fn test_bridge_communication() {
        let bridge_path = PathBuf::from("tools/solid-identity-bridge/dist/index.js");

        if !bridge_path.exists() {
            return; // Skip if bridge not built
        }

        let mut channel = IpcChannel::spawn(bridge_path).unwrap();

        // Test simple request
        let params = serde_json::json!({});
        let result = channel.request("getSessionInfo", params);

        // Should return session info
        assert!(result.is_ok());

        channel.shutdown().unwrap();
    }
}
