# dCAI / SPAI Agent Harness

**A Production-Grade Multi-Agent Orchestration Framework in Rust**

dCAI/SPAI is a comprehensive multi-agent orchestration framework built in Rust, integrating with OpenRouter for unified access to 200+ LLM providers. Current implementation is an inverted framework of CAI (https://github.com/aliasrobotics/cai) for recurring security audits of a local device. Future iterations (Solid Pods Agent Interface) aim to utilize Inrupt's Solid Pods for WebID authentication and authorization of agent swarms alongside IRI resource encapsulation for agent data controls.

## Features

- ** ReAct-Native**: Every agent implements the Thought→Action→Observation loop as a first-class primitive
- ** OpenRouter Integration**: Seamless access to 200+ LLM providers (Claude, GPT-4, Gemini, Llama, and more)
- ** Comprehensive Observability**: Full tracing of agent decisions, tool invocations, and handoffs
- ** Safety-First**: Input/output guardrails and human approval workflows
- ** Flexible Orchestration**: Multiple workflow patterns (Sequential, Concurrent, Hierarchical, Debate, Router, Consensus)
- ** High Performance**: Built on Rust's safety and performance guarantees with tokio async runtime
- ** Extensible Tools**: Native functions, MCP tools, Bash scripts, and HTTP APIs

## Architecture

### SPAI Pattern Components

1. **Agents (A)**: Autonomous LLM-powered entities with specific personas and capabilities
2. **Tools (T)**: Capability extensions via MCP, native functions, Bash scripts, and HTTP APIs
3. **Handoffs (H)**: Inter-agent delegation protocols for task distribution
4. **Patterns (P)**: Workflow orchestration strategies (Sequential, Concurrent, Hierarchical, etc.)
5. **Turns (T)**: Conversation state management and history tracking
6. **Tracing (T)**: Comprehensive observability infrastructure
7. **Guardrails (G)**: Safety validation layers for inputs and outputs
8. **Human-in-the-Loop (H)**: Approval workflows and intervention points

### Perpetual Agent System

Production-grade perpetual agent capabilities:

#### Stateful Agent Memory (`src/memory.rs`)
- **Memory Blocks** — Self-editing chunks with labels, size limits, and metadata
- **In-Context vs Out-of-Context** — Agents control their own context window
- **Perpetual Message History** — Infinite conversation log with search
- **Shared Memory Manager** — Multi-agent shared knowledge bases

#### Agentic Context Engineering (`src/memory_tools.rs`)
- `update_memory` — Edit memory block content
- `move_out_of_context` / `move_into_context` — Archive/restore memories
- `list_memory_blocks` — View all available memories
- `search_messages` — Search perpetual conversation history

#### Agent File Format (`src/agent_file.rs`)
- Complete `.af` serialization format for agent state
- Checkpoint manager for versioned snapshots
- Import/export for portable agent migration

#### Filesystem Integration (`src/filesystem.rs`)
- Attach folders with file pattern filtering
- `open_file`, `search_files`, `list_files` tools
- Per-agent folder attachments with caching

#### Sleep-Time Agents (`src/sleeptime.rs`)
- Background memory consolidation (archives old blocks, summarizes messages)
- Pattern detection across conversation history
- Configurable intervals (default: 5 minutes)
- Runs async without blocking the primary agent

#### Background Execution (`src/background.rs`)
- Async agent execution with run IDs
- **Resumable streaming** with sequence IDs and cursor pagination
- Connection recovery — clients can disconnect/reconnect without losing state
- Event types: Started, Thought, ToolCall, ToolResult, Output, Completed, Failed

#### Storage Backends (`src/storage.rs`)
- **PostgreSQL** — Distributed deployments with full-text search
- Automatic migrations and schema creation

---

### Memory System Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    SPAI Agent                           │
│  ┌───────────────────────────────────────────────────┐ │
│  │            AgentMemory                            │ │
│  │  ┌────────────────┐  ┌──────────────────────┐   │ │
│  │  │  In-Context    │  │   Out-of-Context     │   │ │
│  │  │  Memory Blocks │  │   (Archived)         │   │ │
│  │  └────────────────┘  └──────────────────────┘   │ │
│  │  ┌────────────────────────────────────────────┐ │ │
│  │  │   Perpetual Message History                │ │ │
│  │  └────────────────────────────────────────────┘ │ │
│  └───────────────────────────────────────────────────┘ │
│  ┌───────────────────────────────────────────────────┐ │
│  │  Memory Tools (update, move, list, search)       │ │
│  │  Filesystem Tools (open, search, list files)     │ │
│  └───────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────┘
                          │
                          ▼
            ┌─────────────────────────┐
            │  Shared Memory Manager  │ ← Multi-agent shared blocks
            └─────────────────────────┘
                          │
            ┌─────────────────────────┐
            │  Agent File (.af)       │ ← Serialization/checkpoints
            └─────────────────────────┘
                          │
            ┌─────────────────────────┐
            │  Storage Backend        │ ← PostgreSQL
            └─────────────────────────┘
```

### Prerequisites

- Rust 1.70 or later
- OpenRouter API key (get one at [openrouter.ai](https://openrouter.ai))

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
spai = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust
use spai::prelude::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Set up OpenRouter API key
    std::env::set_var("OPENROUTER_API_KEY", "your-api-key");

    // Create OpenRouter client
    let client = OpenRouterClient::from_env()?;

    // Create an agent with ReAct enabled
    let agent = Agent::builder()
        .name("Research Assistant")
        .model("anthropic/claude-sonnet-4")
        .system_prompt("You are a helpful research assistant.")
        .react_config(ReActConfig {
            enable_reasoning_traces: true,
            reasoning_format: ReasoningFormat::ThoughtAction,
            max_reasoning_tokens: 1000,
            expose_reasoning: true,
        })
        .max_loops(5)
        .client(Arc::new(client))
        .build()?;

    // Run the agent
    let output = agent.react_loop("What are the latest developments in quantum computing?").await?;

    println!("Answer: {}", output.content);
    println!("Reasoning trace:\n{}", output.trace.format());

    Ok(())
}
```

## Tagged Tool System

SPAI provides a modular tool discovery system with tag-based filtering. Tools are organized in the `tools/` directory with corresponding `.json` metadata files that define their tags.

### Available Tag Categories

| Tag | Description | Example Tools |
|-----|-------------|---------------|
| `security_tools` | Security auditing and vulnerability scanning | `file_integrity`, `cve_scanner`, `ssh_auditor`, `firewall_auditor` |
| `web_tools` | Web scraping and API interaction | `exa_search`, `url_fetcher`, `api_client`, `rss_parser` |
| `filesystem_tools` | File operations and search | `semantic_search`, `file_metadata`, `diff_tool`, `archive_handler`, `sed_tool`, `awk_tool`, `cut_tool`, `grep_tool`, `find_tool` |
| `dev_tools` | Development workflow tools | `code_linter`, `test_runner`, `git_ops`, `sandbox_exec`, `sed_tool`, `awk_tool`, `cut_tool`, `grep_tool`, `head_tail`, `sort_uniq`, `wc_tool`, `tr_tool`, `find_tool`, `xargs_tool` |
| `all` | Special tag - returns all tools | * |

### Agent Instantiation with Tagged Tools

```rust
use spai::prelude::*;
use spai::security_tools::{SecurityToolRegistry, TaggedSecurityTools};
use std::sync::Arc;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Discover all tools from the tools directory
    let tools_dir = PathBuf::from("tools");
    let registry = Arc::new(SecurityToolRegistry::discover(&tools_dir));
    
    println!("Discovered {} tools", registry.len());
    println!("Available tags: {:?}", registry.all_tags());
    
    // Load only dev_tools for this agent
    let dev_helper = TaggedSecurityTools::new(registry.clone(), &["dev_tools"]);
    let dev_tools = dev_helper.create_tools();
    
    // Create LLM client
    let client: Arc<dyn LlmClient> = Arc::new(OpenRouterClient::from_env()?);
    
    // Build agent with tagged tools
    let agent = Agent::builder()
        .name("Dev Assistant")
        .model("anthropic/claude-sonnet-4")
        .system_prompt("You are a development assistant with access to dev tools.")
        .tools(dev_tools)
        .max_loops(5)
        .client(client)
        .build()?;
    
    // Run agent
    let output = agent.react_loop("Run the code linter on src/").await?;
    println!("{}", output.content);
    
    Ok(())
}
```

### Loading Multiple Tag Categories

```rust
// Load security and web tools together
let helper = TaggedSecurityTools::new(registry.clone(), &["security_tools", "web_tools"]);

// Load all available tools
let all_helper = TaggedSecurityTools::new(registry.clone(), &["all"]);
```

### Tool Metadata Format

Each tool has a corresponding `.json` file (e.g., `tools/sandbox_exec.json`):

```json
{
    "name": "Sandbox Executor",
    "description": "Run untrusted code in isolated environment",
    "category": "security",
    "tags": ["dev_tools", "security_tools"],
    "requires_sudo": false,
    "args": [
        {"name": "-l", "description": "Language runtime", "required": false},
        {"name": "-t", "description": "Timeout in seconds", "required": false}
    ]
}
```

### Tag Reference File

See `tools/tags.json` for a complete reference of all available tags and their associated tools.

## Guardrails

Implement custom guardrails for input/output validation:

```rust
use spai::guardrails::{InputGuardrail, GuardrailContext, GuardrailResult};
use async_trait::async_trait;

struct MyInputGuardrail;

#[async_trait]
impl InputGuardrail for MyInputGuardrail {
    fn id(&self) -> &str {
        "my_guardrail"
    }

    async fn check(&self, input: &str, ctx: &GuardrailContext) -> spai::Result<GuardrailResult> {
        // Validation logic here
        Ok(GuardrailResult::pass("Input is safe"))
    }
}
```

### Provider Preferences

```rust
use spai::config::{OpenRouterConfig, ProviderPreferences, OptimizationTarget};

let config = OpenRouterConfig::from_env()?
    .with_provider_preferences(ProviderPreferences {
        preferred: vec!["anthropic".to_string(), "openai".to_string()],
        excluded: vec![],
        optimization: OptimizationTarget::Balanced,
    });
```

## Workflow Patterns

The orchestrator module (`src/orchestrator/`) provides YAML-configurable multi-agent coordination patterns:

| Pattern | Constructor | Description |
|---------|-------------|-------------|
| **Sequential** | `SequentialOrchestrator::new(agents)` | Agents execute in order; output of N → input of N+1 |
| **Concurrent** | `ConcurrentOrchestrator::new(agents)` | Parallel execution with aggregation (concatenate, merge, first, longest) |
| **Hierarchical** | `HierarchicalOrchestrator::new(lead, subagents)` | Lead decomposes task → delegates to subagents → synthesizes |
| **Debate** | `DebateOrchestrator::new(pro, con, synth)` | Pro/con argue N rounds; synthesizer produces balanced conclusion |
| **Router** | `RouterOrchestrator::new(router).with_specialists(map)` | Triage routes to domain specialists |
| **Consensus** | `ConsensusOrchestrator::new(agents).with_threshold(0.66)` | Majority voting with configurable threshold |

### YAML Template Example

```yaml
# src/orchestrator/templates/hierarchical.yaml
pattern: hierarchical

lead_agent:
  name: "Lead Analyst"
  model: "tngtech/deepseek-r1t2-chimera:free"
  system_prompt: "Decompose task and coordinate subagents."
  max_loops: 5
  tool_tags:
    - dev_tools  # Load specific tools for this agent

subagents:
  count: 3
  model: "tngtech/deepseek-r1t2-chimera:free"
  system_prompt_template: "You are Analyst {index}."
  tool_tags:
    - web_tools
```

### Usage

```rust
use spai::orchestrator::{OrchestratorConfig, HierarchicalOrchestrator, OrchestratorPattern};

let config = OrchestratorConfig::from_file("src/orchestrator/templates/hierarchical.yaml")?;
let orchestrator = HierarchicalOrchestrator::new(lead_agent, subagents);
let result = orchestrator.execute("Analyze this problem").await?;
println!("{}", result.content);
```

Run the demo: `cargo run --example test_patterns`

## Tracing & Observability

All agent operations are traced with comprehensive metadata:

```rust
let output = agent.react_loop(input).await?;

println!("Iterations: {}", output.trace.iteration_count());
println!("Total tokens: {}", output.trace.total_tokens.total_tokens);
println!("Reasoning trace:\n{}", output.trace.format());
```

Export traces to:
- Console (pretty-printed for development)
- OpenTelemetry (OTLP)
- JSON files
- Database (PostgreSQL)
- Custom backends

## Human-in-the-Loop

Define intervention points for human oversight:

- Pre-execution approval
- Tool authorization
- Handoff approval
- Output review
- Error recovery
- Confidence threshold triggers

## License

idc
