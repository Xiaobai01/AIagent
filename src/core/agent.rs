use crate::core::{Message, MessageRole, LLMResponse, ToolCall, ToolResult};
use crate::memory::MemoryManager;
use crate::prompts::PromptManager;
use crate::llm::{LLMProvider, LLMConfig, create_llm_provider, ToolDefinition};
use crate::skills::SkillManager;
use anyhow::{Result, Context};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub name: String,
    pub role: String,
    pub capabilities: String,
    pub constraints: String,
    pub llm_config: LLMConfig,
    pub short_term_memory_capacity: Option<usize>,
    pub long_term_memory_capacity: Option<usize>,
    pub max_iterations: usize,
}

impl AgentConfig {
    pub fn new(name: String, llm_config: LLMConfig) -> Self {
        Self {
            name,
            role: "You are a helpful AI assistant.".to_string(),
            capabilities: "- Answer questions\n- Help with tasks\n- Execute tools when needed".to_string(),
            constraints: "- Be helpful and harmless\n- Don't execute dangerous operations\n- Ask for clarification when needed".to_string(),
            llm_config,
            short_term_memory_capacity: None,
            long_term_memory_capacity: None,
            max_iterations: 10,
        }
    }

    pub fn with_role(mut self, role: String) -> Self {
        self.role = role;
        self
    }

    pub fn with_capabilities(mut self, capabilities: String) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_constraints(mut self, constraints: String) -> Self {
        self.constraints = constraints;
        self
    }

    pub fn with_memory_capacity(mut self, short_term: usize, long_term: usize) -> Self {
        self.short_term_memory_capacity = Some(short_term);
        self.long_term_memory_capacity = Some(long_term);
        self
    }

    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }
}

pub struct Agent {
    config: AgentConfig,
    llm_provider: Box<dyn LLMProvider>,
    memory: MemoryManager,
    prompt_manager: PromptManager,
    skill_manager: SkillManager,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Result<Self> {
        let llm_provider = create_llm_provider(config.llm_config.clone())?;
        
        Ok(Self {
            config,
            llm_provider,
            memory: MemoryManager::new(None, None),
            prompt_manager: PromptManager::new(),
            skill_manager: SkillManager::new(),
        })
    }

    pub fn with_custom_llm(mut self, llm_provider: Box<dyn LLMProvider>) -> Self {
        self.llm_provider = llm_provider;
        self
    }

    pub fn with_prompt_manager(mut self, prompt_manager: PromptManager) -> Self {
        self.prompt_manager = prompt_manager;
        self
    }

    pub fn with_skill_manager(mut self, skill_manager: SkillManager) -> Self {
        self.skill_manager = skill_manager;
        self
    }

    fn create_system_message(&self) -> Message {
        let mut context = std::collections::HashMap::new();
        context.insert("name", &self.config.name);
        context.insert("role", &self.config.role);
        context.insert("capabilities", &self.config.capabilities);
        context.insert("constraints", &self.config.constraints);

        let content = self.prompt_manager
            .render_template("agent_instruction", &context)
            .unwrap_or_else(|_| {
                format!(
                    "You are {}. {}\n\nCapabilities:\n{}\n\nConstraints:\n{}",
                    self.config.name,
                    self.config.role,
                    self.config.capabilities,
                    self.config.constraints
                )
            });

        Message::system(content)
    }

    fn build_context(&self) -> String {
        let (recent_messages, long_term_memories) = self.memory.get_context(10, 5);
        
        let mut context = String::new();
        
        if !long_term_memories.is_empty() {
            context.push_str("Relevant memories:\n");
            for memory in long_term_memories.iter().take(3) {
                context.push_str(&format!("- {}\n", memory.content));
            }
            context.push_str("\n");
        }
        
        if !recent_messages.is_empty() {
            context.push_str("Recent conversation:\n");
            for msg in recent_messages {
                let role = match msg.role {
                    MessageRole::System => "System",
                    MessageRole::User => "User",
                    MessageRole::Assistant => "Assistant",
                    MessageRole::Tool => "Tool",
                };
                context.push_str(&format!("{}: {}\n", role, msg.content));
            }
        }
        
        context
    }

    async fn process_tool_calls(&self, tool_calls: Vec<ToolCall>) -> Result<Vec<ToolResult>> {
        let mut results = Vec::new();
        
        for tool_call in tool_calls {
            tracing::info!("Executing tool: {} with args: {}", 
                tool_call.name, 
                tool_call.arguments
            );
            
            let params = if let Some(obj) = tool_call.arguments.as_object() {
                serde_json::json!(obj)
            } else {
                serde_json::json!({})
            };
            
            match self.skill_manager.execute_skill(&tool_call.name, params).await {
                Ok(result) => {
                    tracing::info!("Tool {} executed successfully", tool_call.name);
                    results.push(result);
                }
                Err(e) => {
                    tracing::error!("Tool {} failed: {}", tool_call.name, e);
                    results.push(ToolResult {
                        tool_call_id: tool_call.id,
                        result: format!("Error: {}", e),
                        success: false,
                    });
                }
            }
        }
        
        Ok(results)
    }

    pub async fn chat(&mut self, user_input: &str) -> Result<String> {
        let user_message = Message::user(user_input.to_string());
        self.memory.add_message(user_message);

        let system_message = self.create_system_message();
        let context = self.build_context();
        
        let context_message = Message::user(format!(
            "Context:\n{}\n\nUser input: {}",
            context,
            user_input
        ));

        let messages = vec![system_message, context_message];
        let tools = self.skill_manager.get_tool_definitions();

        let mut iteration = 0;
        let mut current_messages = messages.clone();

        while iteration < self.config.max_iterations {
            iteration += 1;
            tracing::info!("Agent iteration {}", iteration);

            let response = self.llm_provider
                .chat_with_tools(current_messages.clone(), tools.clone())
                .await?;

            match response {
                LLMResponse::Text(text) => {
                    let assistant_message = Message::assistant(text.clone());
                    self.memory.add_message(assistant_message);
                    
                    if let Some(memory_items) = self.extract_memorable_content(&text, user_input) {
                        for item in memory_items {
                            self.memory.long_term.add_content(item, None);
                        }
                    }
                    
                    return Ok(text);
                }
                LLMResponse::ToolCall(tool_call) => {
                    current_messages.push(Message::assistant(format!(
                        "Calling tool: {} with arguments: {}",
                        tool_call.name,
                        tool_call.arguments
                    )));
                    
                    let results = self.process_tool_calls(vec![tool_call]).await?;
                    
                    for result in results {
                        let tool_message = Message::tool(format!(
                            "Tool result: {}",
                            result.result
                        ));
                        current_messages.push(tool_message);
                        self.memory.add_message(Message::tool(result.result));
                    }
                }
                LLMResponse::ToolCalls(tool_calls) => {
                    current_messages.push(Message::assistant(format!(
                        "Calling {} tools",
                        tool_calls.len()
                    )));
                    
                    let results = self.process_tool_calls(tool_calls).await?;
                    
                    for result in results {
                        let tool_message = Message::tool(format!(
                            "Tool result: {}",
                            result.result
                        ));
                        current_messages.push(tool_message);
                        self.memory.add_message(Message::tool(result.result));
                    }
                }
            }
        }

        anyhow::bail!("Agent reached maximum iterations without a final response")
    }

    fn extract_memorable_content(&self, response: &str, user_input: &str) -> Option<Vec<String>> {
        if user_input.len() > 20 || response.len() > 100 {
            Some(vec![
                format!("User asked: {}", user_input),
                format!("Assistant responded: {}", response),
            ])
        } else {
            None
        }
    }

    pub fn get_memory(&self) -> &MemoryManager {
        &self.memory
    }

    pub fn get_mut_memory(&mut self) -> &mut MemoryManager {
        &mut self.memory
    }

    pub fn get_skill_manager(&self) -> &SkillManager {
        &self.skill_manager
    }

    pub fn get_prompt_manager(&self) -> &PromptManager {
        &self.prompt_manager
    }

    pub fn clear_memory(&mut self) {
        self.memory.short_term.clear();
        self.memory.long_term.clear();
    }

    pub fn get_stats(&self) -> AgentStats {
        AgentStats {
            short_term_memory_size: self.memory.short_term.len(),
            long_term_memory_size: self.memory.long_term.len(),
            available_skills: self.skill_manager.list_skills().len(),
            model: self.llm_provider.get_model().to_string(),
            provider: self.llm_provider.get_provider().to_string(),
        }
    }
}

#[derive(Debug)]
pub struct AgentStats {
    pub short_term_memory_size: usize,
    pub long_term_memory_size: usize,
    pub available_skills: usize,
    pub model: String,
    pub provider: String,
}

impl std::fmt::Display for AgentStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Agent Statistics:")?;
        writeln!(f, "  Short-term memory: {} messages", self.short_term_memory_size)?;
        writeln!(f, "  Long-term memory: {} items", self.long_term_memory_size)?;
        writeln!(f, "  Available skills: {}", self.available_skills)?;
        writeln!(f, "  Model: {} ({})", self.model, self.provider)
    }
}
