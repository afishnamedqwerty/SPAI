# chkrootkit MCP Server

An MCP (Model Context Protocol) server that wraps the `chkrootkit` security scanning tool and provides intelligent summarization of findings.

## Features

- Runs `sudo chkrootkit -x` (extended mode) for comprehensive rootkit detection
- Automatically categorizes findings:
  - 🔴 **CRITICAL** - Confirmed infections (requires immediate action)
  - 🟡 **WARNING** - Suspicious activity or vulnerabilities
  - ℹ️ **INFO** - Informational findings
- Provides truncated output for large scans
- Full stderr capture for debugging
- Configurable flags via MCP parameters

## Prerequisites

### 1. Install chkrootkit

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install chkrootkit
```

**Fedora/RHEL:**
```bash
sudo dnf install chkrootkit
```

**Arch Linux:**
```bash
sudo pacman -S chkrootkit
```

### 2. Set up passwordless sudo (recommended)

Run the included setup script:
```bash
sudo bash setup_sudo.sh
```

**Or manually create `/etc/sudoers.d/chkrootkit`:**
```
your_username ALL=(ALL) NOPASSWD: /usr/sbin/chkrootkit
```

Then set permissions:
```bash
sudo chmod 0440 /etc/sudoers.d/chkrootkit
```

## Building

From the project root:
```bash
cargo build --release --manifest-path tools/chkrootkit-mcp/Cargo.toml
```

The binary will be at: `tools/chkrootkit-mcp/target/release/chkrootkit-mcp`

## Usage

### Standalone Testing

Test the MCP server directly via stdio:

```bash
# Start the server
tools/chkrootkit-mcp/target/release/chkrootkit-mcp

# In the MCP protocol, send:
{"jsonrpc":"2.0","method":"tools/list","id":1}
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"chkrootkit_scan","arguments":{}},"id":2}
```

### With SPAI Agent Harness

See `examples/basic_agent_chkrootkit.rs` for a complete example:

```rust
use spai::prelude::*;
use spai::tools::McpSubprocessTool;
use std::sync::Arc;

let chkrootkit_tool = Arc::new(McpSubprocessTool::new(
    "chkrootkit",
    "Security Scan",
    "Run sudo chkrootkit -x and summarize findings",
    "chkrootkit_scan",
    "tools/chkrootkit-mcp/target/release/chkrootkit-mcp",
));

let agent = Agent::builder()
    .name("Security Auditor")
    .tools(vec![chkrootkit_tool])
    // ... other configuration
    .build()?;
```

## MCP Tool Interface

### Tool: `chkrootkit_scan`

**Description:** Run chkrootkit with sudo -x and summarize any findings

**Parameters:**
- `flags` (optional): Array of strings - Additional flags to pass to chkrootkit
  - Default: `["-x"]` (extended mode)
  - Example: `{"flags": ["-x", "-q"]}` for extended and quiet mode

**Returns:**
- `CallToolResult` with:
  - Summary of findings (categorized by severity)
  - List of flagged lines with emoji indicators
  - Truncated stdout (up to 8000 characters)
  - Full stderr output (if any)

## Example Output

### Clean System
```
✅ chkrootkit scan completed. No obvious infections or warnings detected.
```

### Warnings Detected
```
🟡 chkrootkit flagged 3 warning(s). Review recommended.

Flagged lines:
- 🟡 Checking `basename`... not found
- 🟡 Checking `dirname`... not found
- 🟡 Searching for suspicious files and dirs...
```

### Critical Issues
```
🔴 CRITICAL: chkrootkit detected 2 infection(s) and 5 warning(s). Immediate review required!

Flagged lines:
- 🔴 /usr/bin/suspicious INFECTED
- 🔴 /tmp/rootkit INFECTED
- 🟡 Warning: hidden process detected
```

## Troubleshooting

### "chkrootkit could not be executed"

1. Verify chkrootkit is installed:
   ```bash
   which chkrootkit
   ```

2. Test sudo access:
   ```bash
   sudo chkrootkit -x
   ```

3. Check sudoers configuration:
   ```bash
   sudo cat /etc/sudoers.d/chkrootkit
   ```

### Permission Denied

If you see permission errors, ensure:
- The sudoers file exists and has correct permissions (0440)
- Your username matches the one in the sudoers file
- You've logged out and back in after creating the sudoers file

### MCP Server Won't Start

1. Ensure it's built:
   ```bash
   ls -lh tools/chkrootkit-mcp/target/release/chkrootkit-mcp
   ```

2. Check for build errors:
   ```bash
   cargo build --release --manifest-path tools/chkrootkit-mcp/Cargo.toml
   ```

3. Test it directly:
   ```bash
   tools/chkrootkit-mcp/target/release/chkrootkit-mcp
   # Should start and wait for MCP protocol messages
   ```

## Security Considerations

This tool requires sudo access to run chkrootkit, which is necessary for:
- Reading system binaries and libraries
- Checking for hidden processes
- Accessing protected filesystem areas

**Recommendations:**
- Only grant passwordless sudo specifically for chkrootkit (not ALL commands)
- Review the sudoers configuration before applying
- Run regular scans to detect compromises early
- Consider running the agent with elevated privileges instead of using passwordless sudo

## License

MIT OR Apache-2.0 (same as parent project)
