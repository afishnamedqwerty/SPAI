solid pods Agent Harness

Agents, Tools, Handoffs, Patterns, Turns, Tracing, Guardrails, Human-in-the-Loop

**A Production-Grade Multi-Agent Orchestration Framework**

Built with swarms-rs & OpenRouter Integration

*Implementing the ReAct Paradigm for Agent Decision Planning*

**Version 1.0.0**

December 2025

1\. Executive Summary

This document specifies the architecture and implementation plan for the SPAI Agent Harness --- a high-performance, production-ready multi-agent orchestration framework built in Rust. The system leverages the swarms-rs toolkit as its foundation while introducing novel architectural patterns inspired by Anthropic\'s Claude Agent SDK and OpenAI\'s Agents SDK.

The SPAI architecture encompasses eight core pillars: **Agents** (autonomous LLM-powered entities), **Tools** (capability extensions via MCP and native functions), **Handoffs** (inter-agent delegation protocols), **Patterns** (workflow orchestration strategies), **Turns** (conversation state management), **Tracing** (observability infrastructure), **Guardrails** (safety validation layers), and **Human-in-the-Loop** (approval workflows and intervention points).

Each instantiated agent operates under the ReAct (Reasoning and Acting) paradigm, interleaving chain-of-thought reasoning with task-specific actions to enable dynamic plan creation, maintenance, and adjustment. OpenRouter integration provides unified access to 200+ LLM providers through a single API surface.

2\. System Architecture Overview

2.1 High-Level Architecture Diagram

┌─────────────────────────────────────────────────────────────────────────────┐ │ SPAI Agent Harness │ ├─────────────────────────────────────────────────────────────────────────────┤ │ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ │ │ │ Agent 1 │ │ Agent 2 │ │ Agent N │ │ Human-in │ │ │ │ (ReAct) │◄─┤ (ReAct) │◄─┤ (ReAct) │◄─┤ the-Loop │ │ │ └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ │ │ │ │ │ │ │ │ └────────────────┴────────────────┴────────────────┘ │ │ │ │ │ ┌──────────────┴──────────────┐ │ │ │ Handoff Controller │ │ │ │ (Delegation & Routing) │ │ │ └──────────────┬──────────────┘ │ │ │ │ │ ┌───────────────────────────────────────────────────────────────────┐ │ │ │ Core Services Layer │ │ │ ├─────────────┬─────────────┬─────────────┬─────────────────────────┤ │ │ │ Pattern │ Turn │ Tracing │ Guardrails │ │ │ │ Executor │ Manager │ Engine │ System │ │ │ └─────────────┴─────────────┴─────────────┴─────────────────────────┘ │ │ │ │ │ ┌───────────────────────────────────────────────────────────────────┐ │ │ │ Tool Registry │ │ │ │ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ │ │ │ │ │ MCP │ │ Native │ │ Bash │ │ HTTP │ │ │ │ │ │ Tools │ │Functions│ │ Scripts │ │ APIs │ │ │ │ │ └─────────┘ └─────────┘ └─────────┘ └─────────┘ │ │ │ └───────────────────────────────────────────────────────────────────┘ │ │ │ │ │ ┌───────────────────────────────────────────────────────────────────┐ │ │ │ OpenRouter LLM Gateway │ │ │ │ Claude │ GPT-4 │ Gemini │ Llama │ Mistral │ DeepSeek │ 200+ │ │ │ └───────────────────────────────────────────────────────────────────┘ │ └─────────────────────────────────────────────────────────────────────────────┘

2.2 Design Philosophy

The SPAI harness follows these core design principles:

-   **Memory Safety First:** Leveraging Rust\'s ownership model to guarantee freedom from data races and memory leaks in concurrent agent execution

-   **ReAct-Native:** Every agent implements the Thought→Action→Observation loop as a first-class primitive, not an afterthought

-   **Provider Agnostic:** OpenRouter integration enables seamless switching between LLM providers without code changes

-   **Observable by Default:** Comprehensive tracing captures every decision point, tool invocation, and handoff for debugging and optimization

-   **Human-Centric:** Built-in approval workflows and intervention points ensure human oversight when required

3\. SPAI Component Specifications

3.1 Agents (A)

Agents are autonomous LLM-powered entities that serve as the primary actors within the system. Each agent encapsulates a specific persona, capabilities, and behavioral constraints.

3.1.1 Agent Structure

pub struct Agent\<TContext = ()\> { /// Unique identifier for this agent instance pub id: AgentId, /// Human-readable name for tracing and debugging pub name: String, /// System prompt defining agent persona and capabilities pub system_prompt: String, /// LLM model identifier (via OpenRouter) pub model: ModelConfig, /// Available tools this agent can invoke pub tools: Vec\<Arc\<dyn Tool\>\>, /// Agents this agent can hand off to pub handoff_targets: Vec\<AgentId\>, /// Input guardrails (run before processing) pub input_guardrails: Vec\<Arc\<dyn InputGuardrail\>\>, /// Output guardrails (run on final output) pub output_guardrails: Vec\<Arc\<dyn OutputGuardrail\>\>, /// Maximum reasoning loops before forcing completion pub max_loops: u32, /// Temperature for LLM sampling pub temperature: f32, /// ReAct configuration for this agent pub react_config: ReActConfig, /// Shared context accessible across agent runs pub context: Arc\<RwLock\<TContext\>\>, /// Agent lifecycle hooks pub hooks: AgentHooks, }

3.1.2 ReAct Implementation

Each agent implements the ReAct paradigm through a structured reasoning loop:

pub struct ReActConfig { /// Enable explicit thought traces before actions pub enable_reasoning_traces: bool, /// Format for reasoning output pub reasoning_format: ReasoningFormat, /// Maximum tokens for reasoning phase pub max_reasoning_tokens: u32, /// Whether to expose reasoning to external observers pub expose_reasoning: bool, } pub enum ReasoningFormat { /// Thought: \... Action: \... format ThoughtAction, /// \<thinking\>\...\</thinking\> XML format XmlThinking, /// JSON structured reasoning JsonStructured, } // ReAct Loop Implementation impl\<TContext\> Agent\<TContext\> { pub async fn react_loop(&self, input: &str) -\> Result\<AgentOutput\> { let mut observations = Vec::new(); let mut trace = ReActTrace::new(); for iteration in 0..self.max_loops { // THOUGHT: Generate reasoning about current state let thought = self.generate_thought(&observations).await?; trace.add_thought(thought.clone()); // ACTION: Decide and execute next action let action = self.decide_action(&thought).await?; trace.add_action(action.clone()); match action { Action::ToolCall(tool_call) =\> { // Execute tool and capture observation let observation = self.execute_tool(tool_call).await?; trace.add_observation(observation.clone()); observations.push(observation); } Action::Handoff(target_agent) =\> { // Delegate to another agent return self.perform_handoff(target_agent, &trace).await; } Action::FinalAnswer(answer) =\> { // Complete the loop with final output return Ok(AgentOutput::new(answer, trace)); } } } // Max loops exceeded - synthesize best-effort response self.synthesize_from_observations(&observations, &trace).await } }

3.2 Tools (T)

Tools extend agent capabilities by providing interfaces to external systems, computations, and data sources. The framework supports multiple tool paradigms through a unified abstraction.

3.2.1 Tool Trait Definition

#\[async_trait\] pub trait Tool: Send + Sync { /// Unique identifier for this tool fn id(&self) -\> &str; /// Human-readable name fn name(&self) -\> &str; /// Description for LLM function calling fn description(&self) -\> &str; /// JSON Schema for input parameters fn input_schema(&self) -\> JsonSchema; /// Execute the tool with given parameters async fn execute(&self, params: Value, ctx: &ToolContext) -\> Result\<ToolOutput\>; /// Optional: Validate parameters before execution fn validate(&self, params: &Value) -\> Result\<()\> { Ok(()) } /// Optional: Estimated execution time for planning fn estimated_duration(&self) -\> Duration { Duration::from_secs(1) } }

3.2.2 Tool Categories

  -----------------------------------------------------------------------------------------------------------
  **Category**           **Implementation**                          **Use Cases**
  ---------------------- ------------------------------------------- ----------------------------------------
  **MCP Tools**          Model Context Protocol via STDIO/SSE        External services, databases, APIs

  **Native Functions**   Rust closures with auto-generated schemas   Computations, data transformations

  **Bash Scripts**       Sandboxed command execution                 File operations, system commands

  **HTTP APIs**          REST/GraphQL with auth handling             Web services, third-party integrations

  **Agent Tools**        Sub-agent invocation as tools               Hierarchical agent orchestration
  -----------------------------------------------------------------------------------------------------------

3.3 Handoffs (H)

Handoffs are specialized control transfer mechanisms that allow agents to delegate tasks to other agents based on capability matching, domain expertise, or workload distribution.

3.3.1 Handoff Protocol

pub struct Handoff { /// Source agent initiating the handoff pub source: AgentId, /// Target agent receiving control pub target: AgentId, /// Reason for handoff (for tracing and debugging) pub reason: String, /// Context to transfer to target agent pub context: HandoffContext, /// Whether to return control after target completes pub return_control: bool, } pub struct HandoffContext { /// Original user query pub original_query: String, /// Accumulated observations from source agent pub observations: Vec\<Observation\>, /// Partial reasoning trace pub trace: ReActTrace, /// Custom metadata for the handoff pub metadata: HashMap\<String, Value\>, } pub enum HandoffStrategy { /// Direct transfer - target takes full control Direct, /// Collaborative - both agents work together Collaborative, /// Supervised - source monitors target\'s progress Supervised { check_interval: Duration }, /// Cascading - target may further delegate Cascading { max_depth: u32 }, }

3.3.2 Handoff Decision Model

Agents decide to hand off based on the following criteria:

1.  **Capability Mismatch:** Current agent lacks required tools or knowledge

2.  **Domain Specialization:** Task requires specialized expertise another agent possesses

3.  **Resource Constraints:** Current agent approaching context limits or timeout

4.  **Policy Requirements:** Certain tasks require specific agent types (e.g., compliance review)

5.  **User Directive:** Explicit user request to involve specific agent

3.4 Patterns (P)

Patterns define the orchestration strategies for multi-agent workflows. The framework provides both built-in patterns and extensibility for custom patterns.

3.4.1 Built-in Workflow Patterns

  --------------------------------------------------------------------------------------------------------------------------------------------
  **Pattern**        **Description**                                                              **Use Case**
  ------------------ ---------------------------------------------------------------------------- --------------------------------------------
  **Sequential**     Agents execute in predetermined order; output of one becomes input to next   Pipeline processing, staged reviews

  **Concurrent**     Agents execute in parallel; results aggregated                               Research tasks, multi-perspective analysis

  **Hierarchical**   Lead agent decomposes tasks and delegates to sub-agents                      Complex projects, divide-and-conquer

  **Debate**         Agents argue different positions; synthesizer produces final output          Decision making, risk assessment

  **Router**         Triage agent routes requests to specialized agents                           Customer support, intent classification

  **Consensus**      Multiple agents must agree before proceeding                                 High-stakes decisions, verification
  --------------------------------------------------------------------------------------------------------------------------------------------

3.4.2 Pattern Configuration

pub trait WorkflowPattern: Send + Sync { /// Execute the pattern with given agents and input async fn execute( &self, agents: &\[Arc\<dyn Agent\>\], input: &str, config: &PatternConfig, ) -\> Result\<PatternOutput\>; /// Validate pattern configuration fn validate_config(&self, config: &PatternConfig) -\> Result\<()\>; } pub struct PatternConfig { /// Maximum total execution time pub timeout: Duration, /// How to handle partial failures pub failure_mode: FailureMode, /// Token budget across all agents pub token_budget: Option\<u64\>, /// Custom pattern-specific parameters pub params: HashMap\<String, Value\>, } pub enum FailureMode { /// Fail entire pattern on any agent failure FailFast, /// Continue with remaining agents FailSafe, /// Retry failed agents with backoff Retry { max_attempts: u32, backoff: Duration }, }

3.5 Turns (T)

Turns manage conversation state and history across agent interactions. The turn manager handles context window optimization, message compaction, and conversation persistence.

3.5.1 Turn Management

pub struct TurnManager { /// Maximum context window tokens max_context_tokens: u64, /// Strategy for context compaction compaction_strategy: CompactionStrategy, /// Persistent storage backend storage: Arc\<dyn TurnStorage\>, } pub struct Turn { /// Unique turn identifier pub id: TurnId, /// Session this turn belongs to pub session_id: SessionId, /// Agent that processed this turn pub agent_id: AgentId, /// User input for this turn pub input: String, /// Agent output for this turn pub output: AgentOutput, /// Timestamp of turn completion pub timestamp: DateTime\<Utc\>, /// Token usage for this turn pub token_usage: TokenUsage, /// ReAct trace for this turn pub trace: ReActTrace, } pub enum CompactionStrategy { /// Remove oldest turns first SlidingWindow { keep_recent: usize }, /// Summarize older turns Summarization { summarize_after: usize }, /// Hybrid approach Hybrid { keep_recent: usize, summarize_middle: usize, }, /// Custom compaction function Custom(Arc\<dyn CompactionFn\>), }

3.5.2 Session Management

Sessions group related turns and maintain long-running conversation context:

pub struct Session { /// Unique session identifier pub id: SessionId, /// User associated with this session pub user_id: Option\<UserId\>, /// Current active agent pub current_agent: AgentId, /// All turns in this session pub turns: Vec\<Turn\>, /// Session-level metadata pub metadata: SessionMetadata, /// Session state (active, paused, completed) pub state: SessionState, } impl TurnManager { /// Start a new session pub async fn create_session(&self, config: SessionConfig) -\> Result\<Session\>; /// Process a new turn within a session pub async fn process_turn( &self, session: &mut Session, input: &str, ) -\> Result\<Turn\>; /// Compact session history to fit context window pub async fn compact(&self, session: &mut Session) -\> Result\<()\>; /// Restore session from storage pub async fn restore_session(&self, id: SessionId) -\> Result\<Session\>; }

3.6 Tracing (T)

The tracing subsystem provides comprehensive observability into agent execution. Every LLM generation, tool call, handoff, and guardrail check is captured with timing, token usage, and contextual metadata.

3.6.1 Trace Structure

pub struct Trace { /// Unique trace identifier pub trace_id: TraceId, /// Human-readable workflow name pub name: String, /// Root span containing all child spans pub root_span: Span, /// Trace-level metadata pub metadata: TraceMetadata, /// Total duration of the trace pub duration: Duration, /// Aggregate token usage pub total_tokens: TokenUsage, } pub struct Span { /// Unique span identifier pub span_id: SpanId, /// Parent span (None for root) pub parent_id: Option\<SpanId\>, /// Span type pub span_type: SpanType, /// Span name pub name: String, /// Start timestamp pub started_at: DateTime\<Utc\>, /// End timestamp pub ended_at: Option\<DateTime\<Utc\>\>, /// Span-specific data pub data: SpanData, /// Child spans pub children: Vec\<Span\>, } pub enum SpanType { AgentRun, LlmGeneration, ToolCall, Handoff, GuardrailCheck, ReActThought, ReActAction, ReActObservation, Custom(String), }

3.6.2 Trace Export

Traces can be exported to multiple backends via the TraceProcessor interface:

-   **Console:** Pretty-printed trace output for development

-   **OpenTelemetry:** Standard OTLP export for observability platforms

-   **JSON Files:** Persistent trace storage for offline analysis

-   **Database:** SQLite/PostgreSQL for queryable trace history

-   **Custom:** User-defined trace processors via trait implementation

3.7 Guardrails (G)

Guardrails provide safety validation layers that run in parallel with or sequential to agent execution. They can be configured to reject inputs, modify outputs, or trigger human review.

3.7.1 Guardrail Types

#\[async_trait\] pub trait InputGuardrail: Send + Sync { /// Unique identifier fn id(&self) -\> &str; /// Check input before agent processing async fn check( &self, input: &str, ctx: &GuardrailContext, ) -\> Result\<GuardrailResult\>; } #\[async_trait\] pub trait OutputGuardrail: Send + Sync { /// Unique identifier fn id(&self) -\> &str; /// Check output after agent processing async fn check( &self, output: &AgentOutput, ctx: &GuardrailContext, ) -\> Result\<GuardrailResult\>; } pub struct GuardrailResult { /// Whether the check passed pub passed: bool, /// Whether to halt execution (tripwire) pub tripwire_triggered: bool, /// Explanation of the result pub reasoning: String, /// Suggested modification (if applicable) pub suggested_modification: Option\<String\>, /// Confidence score (0.0-1.0) pub confidence: f32, }

3.7.2 Built-in Guardrails

  ---------------------------------------------------------------------------------------------
  **Guardrail**         **Function**
  --------------------- -----------------------------------------------------------------------
  ContentModeration     Detects harmful, offensive, or inappropriate content

  PIIDetector           Identifies and optionally redacts personally identifiable information

  PromptInjection       Detects attempts to manipulate agent behavior via input

  TopicClassifier       Ensures queries are on-topic for the agent\'s domain

  FactualAccuracy       Validates output against known facts or sources

  OutputFormat          Ensures output conforms to expected schema or format

  RateLimiter           Enforces per-user, per-session, or global rate limits
  ---------------------------------------------------------------------------------------------

3.8 Human-in-the-Loop (H)

Human-in-the-Loop (HITL) capabilities enable human oversight and intervention at critical decision points. The framework supports synchronous approval workflows, asynchronous review queues, and escalation protocols.

3.8.1 Approval Workflows

pub struct ApprovalRequest { /// Unique request identifier pub id: ApprovalId, /// Agent requesting approval pub agent_id: AgentId, /// Type of action requiring approval pub action_type: ActionType, /// Description of the action pub description: String, /// Detailed context for reviewer pub context: ApprovalContext, /// Urgency level pub priority: Priority, /// Deadline for approval (None = no deadline) pub deadline: Option\<DateTime\<Utc\>\>, /// Suggested approvers pub suggested_approvers: Vec\<UserId\>, } pub enum ApprovalDecision { /// Approve and continue execution Approved { approver: UserId, notes: Option\<String\> }, /// Reject and halt execution Rejected { approver: UserId, reason: String }, /// Request modification before proceeding ModificationRequired { approver: UserId, instructions: String, }, /// Escalate to higher authority Escalated { target: UserId, reason: String }, /// Auto-approved due to timeout AutoApproved { reason: String }, } pub trait ApprovalHandler: Send + Sync { /// Request human approval async fn request_approval( &self, request: ApprovalRequest, ) -\> Result\<ApprovalDecision\>; /// Check status of pending approval async fn check_status(&self, id: ApprovalId) -\> Result\<ApprovalStatus\>; /// Cancel pending approval request async fn cancel(&self, id: ApprovalId) -\> Result\<()\>; }

3.8.2 Intervention Points

The framework defines standard intervention points where human oversight can be injected:

1.  **Pre-Execution:** Before any agent begins processing

2.  **Tool Authorization:** Before executing high-impact tools

3.  **Handoff Approval:** Before delegating to another agent

4.  **Output Review:** Before delivering final output to user

5.  **Error Recovery:** When agent encounters unrecoverable error

6.  **Confidence Threshold:** When agent confidence falls below threshold

4\. OpenRouter Integration

OpenRouter provides unified access to 200+ LLM providers through a single API surface. The SPAI harness integrates with OpenRouter to enable seamless model switching, intelligent routing, and cost optimization.

4.1 Client Configuration

pub struct OpenRouterConfig { /// API key (from env: OPENROUTER_API_KEY) pub api_key: SecretString, /// Base URL (default: https://openrouter.ai/api/v1) pub base_url: Url, /// Default model for agents pub default_model: String, /// Provider preferences for routing pub provider_preferences: ProviderPreferences, /// Fallback models if primary unavailable pub fallback_models: Vec\<String\>, /// Maximum retries on failure pub max_retries: u32, /// Request timeout pub timeout: Duration, } pub struct ProviderPreferences { /// Preferred providers in priority order pub preferred: Vec\<String\>, /// Providers to avoid pub excluded: Vec\<String\>, /// Cost optimization (lower_cost, balanced, performance) pub optimization: OptimizationTarget, } // Recommended model configurations pub mod presets { pub const REASONING: &str = \"anthropic/claude-opus-4\"; pub const BALANCED: &str = \"anthropic/claude-sonnet-4\"; pub const FAST: &str = \"anthropic/claude-haiku-4\"; pub const CODING: &str = \"anthropic/claude-sonnet-4\"; pub const FREE_TIER: &str = \"meta-llama/llama-3.3-70b-instruct:free\"; }

4.2 Streaming Support

impl OpenRouterClient { /// Stream chat completion tokens pub async fn stream_completion( &self, request: CompletionRequest, ) -\> Result\<impl Stream\<Item = Result\<StreamChunk\>\>\> { let response = self.client .post(&format!(\"{}/chat/completions\", self.config.base_url)) .header(\"Authorization\", format!(\"Bearer {}\", self.config.api_key)) .header(\"X-Title\", \"SPAI Agent Harness\") .json(&request.with_stream(true)) .send() .await?; Ok(response.bytes_stream().map(parse_sse_chunk)) } }

5\. swarms-rs Integration

The SPAI harness builds upon swarms-rs as its foundational framework, extending its primitives with the SPAI architecture components.

5.1 Extension Points

-   **Agent Trait:** Extended with ReAct configuration and guardrail hooks

-   **Workflow Types:** Additional patterns beyond Sequential/Concurrent

-   **LLM Provider:** OpenRouter adapter implementing the OpenAI-compatible interface

-   **Persistence:** Enhanced state management for long-running sessions

5.2 Cargo Dependencies

\[dependencies\] swarms-rs = \"0.1.7\" openrouter_api = \"0.4\" tokio = { version = \"1.0\", features = \[\"full\"\] } async-trait = \"0.1\" serde = { version = \"1.0\", features = \[\"derive\"\] } serde_json = \"1.0\" tracing = \"0.1\" tracing-subscriber = { version = \"0.3\", features = \[\"env-filter\"\] } anyhow = \"1.0\" thiserror = \"1.0\" chrono = { version = \"0.4\", features = \[\"serde\"\] } uuid = { version = \"1.0\", features = \[\"v4\", \"serde\"\] } secrecy = \"0.8\" jsonschema = \"0.18\"

6\. Implementation Roadmap

6.1 Phase 1: Core Infrastructure (Weeks 1-4)

1.  OpenRouter client implementation with streaming support

2.  Agent struct with ReAct loop implementation

3.  Tool trait and native function tool implementation

4.  Basic tracing infrastructure

5.  Unit tests for core components

6.2 Phase 2: Orchestration Layer (Weeks 5-8)

1.  Handoff protocol implementation

2.  Sequential and Concurrent workflow patterns

3.  Turn management and session persistence

4.  MCP tool integration

5.  Integration tests for multi-agent workflows

6.3 Phase 3: Safety & Observability (Weeks 9-12)

1.  Guardrail framework and built-in guardrails

2.  Human-in-the-loop approval workflows

3.  OpenTelemetry trace export

4.  Dashboard UI for trace visualization

5.  Performance benchmarks and optimization

6.4 Phase 4: Production Hardening (Weeks 13-16)

1.  Error recovery and retry strategies

2.  Context window compaction strategies

3.  Rate limiting and cost controls

4.  Documentation and examples

5.  Release candidate and beta testing

7\. Example Usage

7.1 Basic Agent with ReAct

use spai::{Agent, OpenRouterClient, Tool, Runner}; use spai::tools::web_search; #\[tokio::main\] async fn main() -\> Result\<()\> { // Initialize OpenRouter client let client = OpenRouterClient::from_env()?; // Create agent with ReAct enabled let researcher = Agent::builder() .name(\"Research Agent\") .model(\"anthropic/claude-sonnet-4\") .system_prompt(\"You are a research assistant. Use web search to find accurate information.\") .tools(vec\![web_search()\]) .react_config(ReActConfig { enable_reasoning_traces: true, reasoning_format: ReasoningFormat::ThoughtAction, max_reasoning_tokens: 1000, expose_reasoning: true, }) .max_loops(5) .build(); // Run the agent let result = Runner::run(&researcher, \"What are the latest developments in quantum computing?\").await?; println!(\"Answer: {}\", result.output); println!(\"Reasoning trace:\\n{}\", result.trace); Ok(()) }

7.2 Multi-Agent Handoff

use spai::{Agent, Handoff, Router}; // Create specialized agents let triage_agent = Agent::builder() .name(\"Triage\") .system_prompt(\"Route requests to the appropriate specialist agent.\") .handoff_targets(vec\![tech_support.id(), billing.id(), general.id()\]) .build(); let tech_support = Agent::builder() .name(\"Tech Support\") .system_prompt(\"You are a technical support specialist.\") .tools(vec\![knowledge_base(), ticket_system()\]) .build(); // Create router pattern let router = Router::new(triage_agent) .add_target(tech_support) .add_target(billing) .add_target(general); // Process request with automatic routing let result = router.run(\"My software keeps crashing\").await?;

7.3 Guardrails and HITL

use spai::{Agent, InputGuardrail, OutputGuardrail, ApprovalRequired}; // Create agent with safety layers let financial_agent = Agent::builder() .name(\"Financial Advisor\") .input_guardrails(vec\![ ContentModeration::new(), TopicClassifier::new(vec\![\"finance\", \"investing\"\]), \]) .output_guardrails(vec\![ PIIDetector::new().with_redaction(), FactualAccuracy::new(), \]) // Require human approval for high-value recommendations .hooks(AgentHooks { on_output: Some(\|output\| { if output.mentions_amount_over(10000.0) { ApprovalRequired::with_priority(Priority::High) } else { Ok(()) } }), ..Default::default() }) .build();

8\. Conclusion

The SPAI Agent Harness represents a comprehensive, production-grade approach to multi-agent orchestration. By combining the performance and safety guarantees of Rust with the proven patterns of the ReAct paradigm and the flexibility of OpenRouter\'s unified LLM access, the framework provides a solid foundation for building sophisticated AI agent systems.

Key differentiators of this architecture:

-   **First-class ReAct implementation** with explicit reasoning traces and action planning

-   **Comprehensive observability** through structured tracing of all agent decisions

-   **Flexible orchestration patterns** supporting diverse multi-agent topologies

-   **Robust safety layer** with input/output guardrails and human oversight

-   **Provider-agnostic design** enabling seamless model switching via OpenRouter

The modular architecture ensures that each component can evolve independently while maintaining clean interfaces between layers. This positions the SPAI harness for long-term maintainability and extensibility as the AI agent landscape continues to mature.

Appendix A: Glossary

  -------------------------------------------------------------------------------------------------------------------------------
  **Term**            **Definition**
  ------------------- -----------------------------------------------------------------------------------------------------------
  **SPAI**        Agents, Tools, Handoffs, Patterns, Turns, Tracing, Guardrails, Human-in-the-Loop

  **ReAct**           Reasoning and Acting - a paradigm for LLM agents that interleaves chain-of-thought reasoning with actions

  **MCP**             Model Context Protocol - a standardized protocol for tool integration

  **Tripwire**        A guardrail condition that immediately halts agent execution when triggered

  **Span**            A single operation within a trace, with start/end times and contextual data

  **Compaction**      The process of reducing conversation history to fit within context window limits
  -------------------------------------------------------------------------------------------------------------------------------

Appendix B: References

1.  Yao et al. (2023). \"ReAct: Synergizing Reasoning and Acting in Language Models.\" [[arXiv:2210.03629]{.underline}](https://arxiv.org/abs/2210.03629)

2.  swarms-rs GitHub Repository. [[The-Swarm-Corporation/swarms-rs]{.underline}](https://github.com/The-Swarm-Corporation/swarms-rs)

3.  OpenRouter API Documentation. [[openrouter.ai/docs]{.underline}](https://openrouter.ai/docs)

4.  OpenAI Agents SDK. [[openai.github.io/openai-agents-python]{.underline}](https://openai.github.io/openai-agents-python/)

5.  Anthropic Claude Agent SDK Documentation. [[platform.claude.com/docs/en/agent-sdk/overview]{.underline}](https://platform.claude.com/docs/en/agent-sdk/overview)

6.  Anthropic Engineering: How we built our multi-agent research system. [[anthropic.com/engineering/multi-agent-research-system]{.underline}](https://www.anthropic.com/engineering/multi-agent-research-system)
