# htop MCP Server

MCP server providing process visualization and system monitoring via htop.

## Features

- List running processes with resource usage
- Get detailed process information
- Monitor system load and memory
- Identify resource-intensive processes
- Process tree visualization

## Tools

### `list_processes`
Get list of running processes sorted by CPU or memory usage

**Input:**
```json
{
  "sort_by": "cpu|memory",
  "limit": 20
}
```

### `get_process_info`
Get detailed information about a specific process

**Input:**
```json
{
  "pid": 1234
}
```

### `get_system_stats`
Get overall system statistics (CPU, memory, load average)

**Input:** None

### `find_suspicious_processes`
Identify potentially suspicious processes based on heuristics

**Input:**
```json
{
  "high_cpu_threshold": 80.0,
  "high_memory_threshold": 80.0
}
```

## Installation

Ensure `htop` is installed on the system:
```bash
# Ubuntu/Debian
sudo apt-get install htop

# RHEL/CentOS
sudo yum install htop

# macOS
brew install htop
```

## Usage

```bash
cargo run --bin htop-mcp
```
