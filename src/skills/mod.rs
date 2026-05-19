use crate::llm::ToolDefinition;
use crate::core::ToolResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub code: Option<String>,
    pub command: Option<String>,
    pub timeout_secs: u64,
}

impl SkillConfig {
    pub fn new(name: String, description: String, parameters: serde_json::Value) -> Self {
        Self {
            name,
            description,
            parameters,
            code: None,
            command: None,
            timeout_secs: 30,
        }
    }

    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }

    pub fn with_command(mut self, command: String) -> Self {
        self.command = Some(command);
        self
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }
}

#[async_trait]
pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> &serde_json::Value;
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult>;
    
    fn to_tool_definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            self.name().to_string(),
            self.description().to_string(),
            self.parameters().clone(),
        )
    }
}

pub struct CommandSkill {
    config: SkillConfig,
}

impl CommandSkill {
    pub fn new(config: SkillConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Skill for CommandSkill {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn description(&self) -> &str {
        &self.config.description
    }

    fn parameters(&self) -> &serde_json::Value {
        &self.config.parameters
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        if let Some(command) = &self.config.command {
            let mut cmd = std::process::Command::new("sh");
            cmd.arg("-c").arg(command);
            
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(str_val) = value.as_str() {
                        cmd.env(key, str_val);
                    }
                }
            }
            
            let output = tokio::time::timeout(
                std::time::Duration::from_secs(self.config.timeout_secs),
                tokio::task::spawn_blocking(move || cmd.output())
            )
            .await??
            ?;
            
            let result = if output.status.success() {
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                format!(
                    "Command failed: {}\nStderr: {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                )
            };
            
            Ok(ToolResult {
                tool_call_id: uuid::Uuid::new_v4().to_string(),
                result,
                success: output.status.success(),
            })
        } else {
            anyhow::bail!("Command skill '{}' has no command configured", self.config.name);
        }
    }
}

pub struct CodeSkill {
    config: SkillConfig,
    interpreter: String,
}

impl CodeSkill {
    pub fn new(config: SkillConfig, interpreter: String) -> Self {
        Self { config, interpreter }
    }
}

#[async_trait]
impl Skill for CodeSkill {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn description(&self) -> &str {
        &self.config.description
    }

    fn parameters(&self) -> &serde_json::Value {
        &self.config.parameters
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        if let Some(code) = &self.config.code {
            let temp_file = format!("/tmp/skill_{}.{}", 
                uuid::Uuid::new_v4(),
                match self.interpreter.as_str() {
                    "python" | "python3" => "py",
                    "node" | "nodejs" => "js",
                    _ => "sh"
                }
            );
            
            std::fs::write(&temp_file, code)?;
            
            let mut cmd = std::process::Command::new(&self.interpreter);
            cmd.arg(&temp_file);
            
            if let Some(obj) = params.as_object() {
                for (key, value) in obj {
                    if let Some(str_val) = value.as_str() {
                        cmd.env(key, str_val);
                    }
                }
            }
            
            let output = tokio::time::timeout(
                std::time::Duration::from_secs(self.config.timeout_secs),
                tokio::task::spawn_blocking(move || cmd.output())
            )
            .await??
            ?;
            
            std::fs::remove_file(&temp_file).ok();
            
            let result = if output.status.success() {
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                format!(
                    "Code execution failed: {}\nStderr: {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                )
            };
            
            Ok(ToolResult {
                tool_call_id: uuid::Uuid::new_v4().to_string(),
                result,
                success: output.status.success(),
            })
        } else {
            anyhow::bail!("Code skill '{}' has no code configured", self.config.name);
        }
    }
}

pub struct FunctionSkill<F> {
    name: String,
    description: String,
    parameters: serde_json::Value,
    func: Arc<F>,
}

impl<F> FunctionSkill<F>
where
    F: Fn(serde_json::Value) -> Result<String> + Send + Sync + 'static,
{
    pub fn new(name: String, description: String, parameters: serde_json::Value, func: F) -> Self {
        Self {
            name,
            description,
            parameters,
            func: Arc::new(func),
        }
    }
}

#[async_trait]
impl<F> Skill for FunctionSkill<F>
where
    F: Fn(serde_json::Value) -> Result<String> + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> &serde_json::Value {
        &self.parameters
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let func = Arc::clone(&self.func);
        let result = tokio::task::spawn_blocking(move || func(params))
            .await??;
        
        Ok(ToolResult {
            tool_call_id: uuid::Uuid::new_v4().to_string(),
            result,
            success: true,
        })
    }
}

pub struct SkillManager {
    skills: HashMap<String, Arc<dyn Skill>>,
}

impl SkillManager {
    pub fn new() -> Self {
        let mut manager = Self {
            skills: HashMap::new(),
        };
        
        manager.register_default_skills();
        manager
    }

    fn register_default_skills(&mut self) {
        let file_read_skill = FunctionSkill::new(
            "read_file".to_string(),
            "Read the contents of a file".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to read"
                    }
                },
                "required": ["path"]
            }),
            |params| {
                let path = params["path"].as_str()
                    .context("Missing 'path' parameter")?;
                let content = std::fs::read_to_string(path)
                    .context(format!("Failed to read file: {}", path))?;
                Ok(content)
            }
        );
        
        let file_write_skill = FunctionSkill::new(
            "write_file".to_string(),
            "Write content to a file".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
            |params| {
                let path = params["path"].as_str()
                    .context("Missing 'path' parameter")?;
                let content = params["content"].as_str()
                    .context("Missing 'content' parameter")?;
                std::fs::write(path, content)
                    .context(format!("Failed to write file: {}", path))?;
                Ok(format!("Successfully wrote to {}", path))
            }
        );
        
        let list_dir_skill = FunctionSkill::new(
            "list_directory".to_string(),
            "List contents of a directory".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The path to the directory to list"
                    }
                },
                "required": ["path"]
            }),
            |params| {
                let path = params["path"].as_str()
                    .context("Missing 'path' parameter")?;
                let entries = std::fs::read_dir(path)
                    .context(format!("Failed to read directory: {}", path))?;
                
                let mut files = Vec::new();
                for entry in entries {
                    if let Ok(entry) = entry {
                        files.push(entry.file_name().to_string_lossy().to_string());
                    }
                }
                
                Ok(files.join("\n"))
            }
        );
        
        let http_get_skill = FunctionSkill::new(
            "http_get".to_string(),
            "Make an HTTP GET request".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to request"
                    }
                },
                "required": ["url"]
            }),
            |params| {
                let url = params["url"].as_str()
                    .context("Missing 'url' parameter")?;
                
                let client = reqwest::blocking::Client::new();
                let response = client.get(url).send()
                    .context(format!("Failed to request URL: {}", url))?;
                
                let status = response.status();
                let body = response.text()
                    .context("Failed to read response body")?;
                
                Ok(format!("Status: {}\nBody: {}", status, body))
            }
        );
        
        let calculator_skill = FunctionSkill::new(
            "calculate".to_string(),
            "Perform mathematical calculations".to_string(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "The mathematical expression to evaluate"
                    }
                },
                "required": ["expression"]
            }),
            |params| {
                let expression = params["expression"].as_str()
                    .context("Missing 'expression' parameter")?;
                
                let result = meval::eval_str(expression)
                    .context(format!("Failed to evaluate expression: {}", expression))?;
                
                Ok(result.to_string())
            }
        );
        
        self.skills.insert(file_read_skill.name().to_string(), Arc::new(file_read_skill));
        self.skills.insert(file_write_skill.name().to_string(), Arc::new(file_write_skill));
        self.skills.insert(list_dir_skill.name().to_string(), Arc::new(list_dir_skill));
        self.skills.insert(http_get_skill.name().to_string(), Arc::new(http_get_skill));
        self.skills.insert(calculator_skill.name().to_string(), Arc::new(calculator_skill));
    }

    pub fn register_skill(&mut self, skill: Arc<dyn Skill>) {
        self.skills.insert(skill.name().to_string(), skill);
    }

    pub fn get_skill(&self, name: &str) -> Option<Arc<dyn Skill>> {
        self.skills.get(name).cloned()
    }

    pub fn list_skills(&self) -> Vec<&Arc<dyn Skill>> {
        self.skills.values().collect()
    }

    pub fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.skills.values()
            .map(|skill| skill.to_tool_definition())
            .collect()
    }

    pub async fn execute_skill(&self, name: &str, params: serde_json::Value) -> Result<ToolResult> {
        let skill = self.get_skill(name)
            .with_context(|| format!("Skill '{}' not found", name))?;
        skill.execute(params).await
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}
