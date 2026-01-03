# dCAISPAI Agent Harness

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

## Implementation Summary

### Core Framework (ATHPTTGH)

The framework implements all eight core components of the ATHPTTGH architecture:

| Component | Description | Status |
|-----------|-------------|--------|
| **Agents** | Autonomous LLM-powered entities with ReAct loops | ✅ Complete |
| **Tools** | Extensible capability system (MCP, native, Bash, HTTP) | ✅ Complete |
| **Handoffs** | Inter-agent delegation protocols | ✅ Complete |
| **Patterns** | Workflow orchestration (Sequential, Concurrent, Router, etc.) | ✅ Complete |
| **Turns** | Session/conversation state management | ✅ Complete |
| **Tracing** | OpenTelemetry-compatible observability | ✅ Complete |
| **Guardrails** | Input/output safety validation with tripwires | ✅ Complete |
| **Human-in-the-Loop** | Approval workflows and intervention points | ✅ Complete |

**Key Design Decisions:**
- **Rust over Python** — Memory safety, zero-cost abstractions, fearless concurrency
- **rustls-tls** — No C dependencies, easier cross-platform builds
- **ReAct as First-Class** — Thought→Action→Observation is foundational, not bolted-on
- **Trait-Based Extensibility** — Custom tools, guardrails, patterns, and storage backends

---

### Advanced Features (Perpetual Agent System)

Production-grade perpetual agent capabilities matching **Letta** and **MemGPT**:

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
            │  Storage Backend        │ ← SQLite / PostgreSQL
            └─────────────────────────┘
```

**Framework Comparison:**

| Feature | SPAI | Letta | MemGPT | LangChain |
|---------|------|-------|--------|-----------|
| Memory Hierarchy | ✅ | ✅ | ✅ | ❌ |
| Agentic Context Control | ✅ | ✅ | ✅ | ❌ |
| Sleep-Time Agents | ✅ | ✅ | ❌ | ❌ |
| Resumable Streaming | ✅ | ❌ | ❌ | ❌ |
| Language | Rust | Python | Python | Python |
| Performance | **High** | Medium | Medium | Medium |

---

## Quick Start

Want to run a comprehensive security scan in 3 minutes? See [QUICKSTART.md](QUICKSTART.md)

### Prerequisites

- Rust 1.70 or later
- OpenRouter API key (get one at [openrouter.ai](https://openrouter.ai))
- For security agent: chkrootkit, rkhunter, lynis (automated install available)

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

### ReAct Loop Implementation

Each agent implements the ReAct (Reasoning and Acting) paradigm:

```
┌──────────────────────────────────────┐
│         ReAct Loop Iteration         │
├──────────────────────────────────────┤
│                                      │
│  1. THOUGHT                          │
│     ↓ Generate reasoning about       │
│     ↓ current state                  │
│                                      │
│  2. ACTION                           │
│     ↓ Decide next action:            │
│     ↓ - Tool call                    │
│     ↓ - Handoff                      │
│     ↓ - Final answer                 │
│                                      │
│  3. OBSERVATION                      │
│     ↓ Capture results/feedback       │
│     ↓                                │
│                                      │
│  Repeat until Final Answer           │
│  or Max Loops reached                │
└──────────────────────────────────────┘
```

## Examples

### 🔐 Comprehensive Security Agent (Featured)

The flagship example demonstrating all framework capabilities with three integrated security tools:

```bash
# One-time setup (2-3 minutes)
sudo bash tools/setup_all_security_tools.sh
./tools/build_all_mcp.sh
export OPENROUTER_API_KEY=your-key

# Run comprehensive security scan
cargo run --example basic_agent_chkrootkit --features mcp-tools
```

See [SECURITY_AGENT_SETUP.md](SECURITY_AGENT_SETUP.md) for full documentation.

**What it does:**
- Runs chkrootkit, rkhunter, portlist, tshark (60s default capture) and lynis security scans
- Uses designated OpenRouter model (or local vllm hosted with --local where a model can be hosted in your org's private net connected to SPAI harness on each device for routine heartbeat reporting of system configs, IAM/RBAC and firewall rules, and general security assessments at scale)
- Provides comprehensive security assessment with prioritized recommendations
- Demonstrates: MCP tools, multi-tool coordination, ReAct loop, detailed tracing

### Basic Agent

See [examples/basic_agent.rs](examples/basic_agent.rs) for a simple calculator agent demonstration.

```bash
OPENROUTER_API_KEY=your-key cargo run --example basic_agent
```

### OpenRouter Client

See [examples/openrouter_client.rs](examples/openrouter_client.rs) for direct API integration.

```bash
OPENROUTER_API_KEY=your-key cargo run --example openrouter_client
```

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

The framework supports multiple orchestration patterns:

- **Sequential**: Agents execute in predetermined order
- **Concurrent**: Agents execute in parallel with result aggregation
- **Hierarchical**: Lead agent decomposes tasks and delegates
- **Debate**: Agents argue different positions with synthesis
- **Router**: Triage agent routes to specialized agents
- **Consensus**: Multiple agents must agree before proceeding

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
