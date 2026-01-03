//! Agent implementation with ReAct loop

use crate::config::ModelConfig;
use crate::error::{Error, Result};
use crate::guardrails::{GuardrailContext, InputGuardrail, OutputGuardrail};
use crate::llm_client::LlmClient;
use crate::openrouter::{CompletionRequest, Message};
use crate::react::{Action, Observation, ReActConfig, ReActTrace, Thought};
use crate::tools::{Tool, ToolContext};
use crate::types::{AgentId, TokenUsage};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Agent structure
pub struct Agent<TContext = ()> {
    /// Unique identifier for this agent instance
    pub id: AgentId,
    /// Human-readable name for tracing and debugging
    pub name: String,
    /// System prompt defining agent persona and capabilities
    pub system_prompt: String,
    /// LLM model configuration
    pub model: ModelConfig,
    /// Available tools this agent can invoke
    pub tools: Vec<Arc<dyn Tool>>,
    /// Agents this agent can hand off to
    pub handoff_targets: Vec<AgentId>,
    /// Input guardrails (run before processing)
    pub input_guardrails: Vec<Arc<dyn InputGuardrail>>,
    /// Output guardrails (run on final output)
    pub output_guardrails: Vec<Arc<dyn OutputGuardrail>>,
    /// Maximum reasoning loops before forcing completion
    pub max_loops: u32,
    /// Temperature for LLM sampling
    pub temperature: f32,
    /// ReAct configuration for this agent
    pub react_config: ReActConfig,
    /// Shared context accessible across agent runs
    pub context: Arc<RwLock<TContext>>,
    /// Agent lifecycle hooks
    pub hooks: AgentHooks,
    /// LLM client (OpenRouter, vLLM, etc.)
    client: Arc<dyn LlmClient>,
}

impl Agent<()> {
    /// Create a new agent builder with unit context
    pub fn builder() -> AgentBuilder<()> {
        AgentBuilder::new()
    }
}

impl<TContext> Agent<TContext>
where
    TContext: Send + Sync + 'static,
{
    /// Create a new agent builder with custom context
    pub fn builder_with_context() -> AgentBuilder<TContext>
    where
        TContext: Default,
    {
        AgentBuilder::new()
    }

    /// Execute the ReAct loop for the given input
    pub async fn react_loop(&self, input: &str) -> Result<AgentOutput> {
        // Check input guardrails
        let guardrail_ctx = GuardrailContext::new(self.id);
        for guardrail in &self.input_guardrails {
            let result = guardrail.check(input, &guardrail_ctx).await?;
            if !result.passed {
                return Err(Error::guardrail_violation(
                    guardrail.id(),
                    result.reasoning,
                ));
            }
        }

        let mut trace = ReActTrace::new();
        let mut messages = vec![
            Message::system(&self.system_prompt),
            Message::user(input),
        ];

        for _iteration in 0..self.max_loops {
            // THOUGHT: Generate reasoning about current state
            let thought = self.generate_thought(&messages).await?;
            trace.add_thought(thought.clone());

            // Parse the thought to determine the next action
            let action = self.decide_action(&thought, &messages).await?;
            trace.add_action(action.clone());

            match action {
                Action::ToolCall { tool_id, params, .. } => {
                    // Execute tool and capture observation
                    let observation = self.execute_tool(&tool_id, params).await?;
                    trace.add_observation(observation.clone());

                    // Add tool result to messages
                    messages.push(Message::assistant(&thought.content));
                    messages.push(Message::user(&observation.content));
                }
                Action::Handoff { target_agent, reason, .. } => {
                    // TODO: Implement handoff to another agent
                    return Err(Error::handoff(format!(
                        "Handoff to agent {} not yet implemented: {}",
                        target_agent, reason
                    )));
                }
                Action::FinalAnswer { answer, .. } => {
                    // Complete the loop with final output
                    trace.complete();
                    let output = AgentOutput {
                        agent_id: self.id,
                        content: answer,
                        trace,
                        metadata: serde_json::json!({}),
                    };

                    // Check output guardrails
                    for guardrail in &self.output_guardrails {
                        let result = guardrail.check(&output, &guardrail_ctx).await?;
                        if !result.passed {
                            return Err(Error::guardrail_violation(
                                guardrail.id(),
                                result.reasoning,
                            ));
                        }
                    }

                    return Ok(output);
                }
            }
        }

        // Max loops exceeded - synthesize best-effort response
        trace.complete();
        Err(Error::MaxLoopsExceeded(self.max_loops))
    }

    /// Generate a thought based on the current state
    async fn generate_thought(&self, messages: &[Message]) -> Result<Thought> {
        let request = CompletionRequest::new(&self.model.model, messages.to_vec())
            .with_temperature(self.temperature)
            .with_max_tokens(self.react_config.max_reasoning_tokens);

        let response = self.client.complete(request).await?;

        let content = response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .unwrap_or_default();

        let tokens = TokenUsage::from(response.usage);

        Ok(Thought::new(content).with_tokens(tokens))
    }

    /// Decide the next action based on the thought
    async fn decide_action(&self, thought: &Thought, _messages: &[Message]) -> Result<Action> {
        // Simple parsing logic - in production, this would be more sophisticated
        let content = thought.content.to_lowercase();

        // Check for final answer
        if content.contains("final answer:") || content.contains("answer:") {
            // Extract the answer after "final answer:" or "answer:"
            let answer = if let Some(idx) = content.find("final answer:") {
                thought.content[idx + 13..].trim().to_string()
            } else if let Some(idx) = content.find("answer:") {
                thought.content[idx + 7..].trim().to_string()
            } else {
                thought.content.clone()
            };

            return Ok(Action::final_answer(answer));
        }

        // Check for tool calls
        if content.contains("action:") {
            // Simple parsing - in production, would use function calling
            if let Some(tool) = self.tools.first() {
                return Ok(Action::tool_call(
                    tool.id(),
                    serde_json::json!({ "message": "test" }),
                ));
            }
        }

        // Default to final answer if no action detected
        Ok(Action::final_answer(&thought.content))
    }

    /// Execute a tool with the given parameters
    async fn execute_tool(&self, tool_id: &str, params: serde_json::Value) -> Result<Observation> {
        let tool = self
            .tools
            .iter()
            .find(|t| t.id() == tool_id)
            .ok_or_else(|| Error::tool_execution(tool_id, "Tool not found"))?;

        let ctx = ToolContext::new(self.id);
        let output = tool.execute(params, &ctx).await?;

        if output.success {
            Ok(Observation::new(&output.content))
        } else {
            Ok(Observation::error(
                output.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }

    /// Perform a handoff to another agent
    async fn perform_handoff(&self, _target_agent: AgentId, _trace: &ReActTrace) -> Result<AgentOutput> {
        // TODO: Implement handoff logic
        Err(Error::handoff("Handoff not yet implemented"))
    }
}

/// Agent builder
pub struct AgentBuilder<TContext = ()> {
    name: Option<String>,
    system_prompt: Option<String>,
    model: Option<String>,
    tools: Vec<Arc<dyn Tool>>,
    handoff_targets: Vec<AgentId>,
    input_guardrails: Vec<Arc<dyn InputGuardrail>>,
    output_guardrails: Vec<Arc<dyn OutputGuardrail>>,
    max_loops: u32,
    temperature: f32,
    react_config: Option<ReActConfig>,
    context: Option<Arc<RwLock<TContext>>>,
    hooks: AgentHooks,
    client: Option<Arc<dyn LlmClient>>,
}

impl<TContext> AgentBuilder<TContext>
where
    TContext: Send + Sync + Default + 'static,
{
    /// Create a new agent builder
    pub fn new() -> Self {
        Self {
            name: None,
            system_prompt: None,
            model: None,
            tools: Vec::new(),
            handoff_targets: Vec::new(),
            input_guardrails: Vec::new(),
            output_guardrails: Vec::new(),
            max_loops: 10,
            temperature: 0.7,
            react_config: None,
            context: None,
            hooks: AgentHooks::default(),
            client: None,
        }
    }

    /// Set the agent name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the system prompt
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Set the model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add a tool
    pub fn tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add multiple tools
    pub fn tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Add a handoff target
    pub fn handoff_target(mut self, target: AgentId) -> Self {
        self.handoff_targets.push(target);
        self
    }

    /// Add an input guardrail
    pub fn input_guardrail(mut self, guardrail: Arc<dyn InputGuardrail>) -> Self {
        self.input_guardrails.push(guardrail);
        self
    }

    /// Add an output guardrail
    pub fn output_guardrail(mut self, guardrail: Arc<dyn OutputGuardrail>) -> Self {
        self.output_guardrails.push(guardrail);
        self
    }

    /// Set the maximum loops
    pub fn max_loops(mut self, max_loops: u32) -> Self {
        self.max_loops = max_loops;
        self
    }

    /// Set the temperature
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set the ReAct configuration
    pub fn react_config(mut self, config: ReActConfig) -> Self {
        self.react_config = Some(config);
        self
    }

    /// Set the context
    pub fn context(mut self, context: Arc<RwLock<TContext>>) -> Self {
        self.context = Some(context);
        self
    }

    /// Set the hooks
    pub fn hooks(mut self, hooks: AgentHooks) -> Self {
        self.hooks = hooks;
        self
    }

    /// Set the LLM client (OpenRouter, vLLM, etc.)
    pub fn client(mut self, client: Arc<dyn LlmClient>) -> Self {
        self.client = Some(client);
        self
    }

    /// Build the agent
    pub fn build(self) -> Result<Agent<TContext>> {
        let name = self.name.ok_or_else(|| Error::config("Agent name is required"))?;
        let system_prompt = self
            .system_prompt
            .ok_or_else(|| Error::config("System prompt is required"))?;
        let model_name = self.model.unwrap_or_else(|| crate::config::presets::BALANCED.to_string());

        let client = self
            .client
            .or_else(|| {
                // Try OpenRouter as default
                crate::openrouter::OpenRouterClient::from_env()
                    .ok()
                    .map(|c| Arc::new(c) as Arc<dyn LlmClient>)
            })
            .ok_or_else(|| Error::config("LLM client not configured (set OPENROUTER_API_KEY or VLLM_BASE_URL)"))?;

        Ok(Agent {
            id: AgentId::new(),
            name,
            system_prompt,
            model: ModelConfig::new(model_name),
            tools: self.tools,
            handoff_targets: self.handoff_targets,
            input_guardrails: self.input_guardrails,
            output_guardrails: self.output_guardrails,
            max_loops: self.max_loops,
            temperature: self.temperature,
            react_config: self.react_config.unwrap_or_default(),
            context: self.context.unwrap_or_else(|| Arc::new(RwLock::new(TContext::default()))),
            hooks: self.hooks,
            client,
        })
    }
}

impl<TContext> Default for AgentBuilder<TContext>
where
    TContext: Send + Sync + Default + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Agent output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// Agent that produced this output
    pub agent_id: AgentId,
    /// Output content
    pub content: String,
    /// ReAct trace
    pub trace: ReActTrace,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

impl AgentOutput {
    /// Create a new agent output
    pub fn new(agent_id: AgentId, content: impl Into<String>, trace: ReActTrace) -> Self {
        Self {
            agent_id,
            content: content.into(),
            trace,
            metadata: serde_json::json!({}),
        }
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Agent lifecycle hooks
#[derive(Clone, Default)]
pub struct AgentHooks {
    /// Hook called before agent starts processing
    pub on_start: Option<Arc<dyn Fn(&str) -> Result<()> + Send + Sync>>,
    /// Hook called after agent completes processing
    pub on_complete: Option<Arc<dyn Fn(&AgentOutput) -> Result<()> + Send + Sync>>,
    /// Hook called on agent error
    pub on_error: Option<Arc<dyn Fn(&Error) -> Result<()> + Send + Sync>>,
}

impl std::fmt::Debug for AgentHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentHooks")
            .field("on_start", &self.on_start.is_some())
            .field("on_complete", &self.on_complete.is_some())
            .field("on_error", &self.on_error.is_some())
            .finish()
    }
}
