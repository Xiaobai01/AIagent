use crate::core::{Message, MessageRole, LLMResponse, ToolCall};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: String,
    pub api_key: String,
    pub model: String,
    pub base_url: Option<String>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub timeout_secs: u64,
}

impl LLMConfig {
    pub fn openai(api_key: String, model: Option<String>) -> Self {
        Self {
            provider: "openai".to_string(),
            api_key,
            model: model.unwrap_or("gpt-4".to_string()),
            base_url: Some("https://api.openai.com/v1".to_string()),
            temperature: 0.7,
            max_tokens: Some(2048),
            timeout_secs: 30,
        }
    }

    pub fn anthropic(api_key: String, model: Option<String>) -> Self {
        Self {
            provider: "anthropic".to_string(),
            api_key,
            model: model.unwrap_or("claude-3-sonnet-20240229".to_string()),
            base_url: Some("https://api.anthropic.com".to_string()),
            temperature: 0.7,
            max_tokens: Some(2048),
            timeout_secs: 30,
        }
    }

    pub fn ollama(model: String, base_url: Option<String>) -> Self {
        Self {
            provider: "ollama".to_string(),
            api_key: String::new(),
            model,
            base_url: base_url.or_else(|| Some("http://localhost:11434".to_string())),
            temperature: 0.7,
            max_tokens: Some(2048),
            timeout_secs: 60,
        }
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: Vec<Message>) -> Result<LLMResponse>;
    async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Result<LLMResponse>;
    fn get_model(&self) -> &str;
    fn get_provider(&self) -> &str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    pub fn new(name: String, description: String, parameters: serde_json::Value) -> Self {
        Self {
            name,
            description,
            parameters,
        }
    }
}

pub struct OpenAIProvider {
    config: LLMConfig,
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(config: LLMConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()?;
        
        Ok(Self { config, client })
    }

    async fn send_request(&self, payload: serde_json::Value) -> Result<LLMResponse> {
        let base_url = self.config.base_url.as_ref()
            .context("Base URL not configured")?;
        
        let url = format!("{}/chat/completions", base_url);
        
        let response = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("OpenAI API error: {}", error_text);
        }

        let result: serde_json::Value = response.json().await?;
        
        if let Some(choices) = result["choices"].as_array() {
            if let Some(first_choice) = choices.first() {
                if let Some(content) = first_choice["message"]["content"].as_str() {
                    return Ok(LLMResponse::Text(content.to_string()));
                }
                
                if let Some(tool_calls) = first_choice["message"]["tool_calls"].as_array() {
                    let calls: Vec<ToolCall> = tool_calls.iter()
                        .filter_map(|tc| {
                            let id = tc["id"].as_str()?.to_string();
                            let name = tc["function"]["name"].as_str()?.to_string();
                            let arguments = tc["function"]["arguments"].clone();
                            Some(ToolCall { id, name, arguments })
                        })
                        .collect();
                    
                    if calls.len() == 1 {
                        return Ok(LLMResponse::ToolCall(calls[0].clone()));
                    } else {
                        return Ok(LLMResponse::ToolCalls(calls));
                    }
                }
            }
        }

        anyhow::bail!("Unexpected response format from OpenAI API");
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat(&self, messages: Vec<Message>) -> Result<LLMResponse> {
        let openai_messages: Vec<serde_json::Value> = messages.iter().map(|msg| {
            let role = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
            };
            
            serde_json::json!({
                "role": role,
                "content": msg.content
            })
        }).collect();

        let payload = serde_json::json!({
            "model": self.config.model,
            "messages": openai_messages,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens
        });

        self.send_request(payload).await
    }

    async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Result<LLMResponse> {
        let openai_messages: Vec<serde_json::Value> = messages.iter().map(|msg| {
            let role = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
            };
            
            serde_json::json!({
                "role": role,
                "content": msg.content
            })
        }).collect();

        let tools_json: Vec<serde_json::Value> = tools.iter().map(|tool| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters
                }
            })
        }).collect();

        let payload = serde_json::json!({
            "model": self.config.model,
            "messages": openai_messages,
            "temperature": self.config.temperature,
            "max_tokens": self.config.max_tokens,
            "tools": tools_json
        });

        self.send_request(payload).await
    }

    fn get_model(&self) -> &str {
        &self.config.model
    }

    fn get_provider(&self) -> &str {
        &self.config.provider
    }
}

pub struct OllamaProvider {
    config: LLMConfig,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(config: LLMConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()?;
        
        Ok(Self { config, client })
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn chat(&self, messages: Vec<Message>) -> Result<LLMResponse> {
        let base_url = self.config.base_url.as_ref()
            .context("Base URL not configured")?;
        
        let url = format!("{}/api/chat", base_url);
        
        let ollama_messages: Vec<serde_json::Value> = messages.iter().map(|msg| {
            let role = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "user",
            };
            
            serde_json::json!({
                "role": role,
                "content": msg.content
            })
        }).collect();

        let payload = serde_json::json!({
            "model": self.config.model,
            "messages": ollama_messages,
            "stream": false,
            "options": {
                "temperature": self.config.temperature,
                "num_predict": self.config.max_tokens.unwrap_or(2048)
            }
        });

        let response = self.client.post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Ollama API error: {}", error_text);
        }

        let result: serde_json::Value = response.json().await?;
        
        if let Some(content) = result["message"]["content"].as_str() {
            Ok(LLMResponse::Text(content.to_string()))
        } else {
            anyhow::bail!("Unexpected response format from Ollama API");
        }
    }

    async fn chat_with_tools(&self, messages: Vec<Message>, _tools: Vec<ToolDefinition>) -> Result<LLMResponse> {
        self.chat(messages).await
    }

    fn get_model(&self) -> &str {
        &self.config.model
    }

    fn get_provider(&self) -> &str {
        &self.config.provider
    }
}

pub fn create_llm_provider(config: LLMConfig) -> Result<Box<dyn LLMProvider>> {
    match config.provider.as_str() {
        "openai" => Ok(Box::new(OpenAIProvider::new(config)?)),
        "ollama" => Ok(Box::new(OllamaProvider::new(config)?)),
        _ => anyhow::bail!("Unsupported LLM provider: {}", config.provider),
    }
}
