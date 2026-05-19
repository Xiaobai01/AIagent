use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::{Result, Context};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub name: String,
    pub template: String,
    pub variables: Vec<String>,
    pub description: Option<String>,
}

impl PromptTemplate {
    pub fn new(name: String, template: String) -> Self {
        let variables = Self::extract_variables(&template);
        Self {
            name,
            template,
            variables,
            description: None,
        }
    }

    fn extract_variables(template: &str) -> Vec<String> {
        let mut variables = Vec::new();
        let mut in_variable = false;
        let mut current_var = String::new();

        for ch in template.chars() {
            if ch == '{' {
                in_variable = true;
                current_var.clear();
            } else if ch == '}' && in_variable {
                in_variable = false;
                if !current_var.is_empty() {
                    variables.push(current_var.clone());
                }
            } else if in_variable {
                current_var.push(ch);
            }
        }

        variables
    }

    pub fn render(&self, context: &HashMap<String, String>) -> Result<String> {
        let mut reg = Handlebars::new();
        reg.register_template_string(&self.name, &self.template)?;
        
        let json_context: serde_json::Value = serde_json::to_value(context)?;
        Ok(reg.render(&self.name, &json_context)?)
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

#[derive(Debug, Clone)]
pub struct PromptManager {
    templates: HashMap<String, PromptTemplate>,
    system_prompts: HashMap<String, String>,
}

impl PromptManager {
    pub fn new() -> Self {
        let mut manager = Self {
            templates: HashMap::new(),
            system_prompts: HashMap::new(),
        };
        
        manager.register_default_templates();
        manager
    }

    fn register_default_templates(&mut self) {
        let default_templates = vec![
            PromptTemplate::new(
                "agent_instruction".to_string(),
                "You are an AI assistant named {{name}}. {{role}}\n\nCapabilities:\n{{capabilities}}\n\nConstraints:\n{{constraints}}".to_string()
            ).with_description("Main system prompt for agent behavior".to_string()),
            
            PromptTemplate::new(
                "task_planning".to_string(),
                "Given the following goal: {{goal}}\n\nAvailable tools: {{tools}}\n\nCreate a step-by-step plan to achieve this goal.".to_string()
            ).with_description("Task planning prompt".to_string()),
            
            PromptTemplate::new(
                "tool_selection".to_string(),
                "Based on the current context: {{context}}\n\nAnd the goal: {{goal}}\n\nSelect the most appropriate tool and provide the parameters.".to_string()
            ).with_description("Tool selection prompt".to_string()),
            
            PromptTemplate::new(
                "memory_query".to_string(),
                "Previous conversation context:\n{{conversation_history}}\n\nRelevant memories:\n{{memories}}\n\nCurrent user input: {{input}}".to_string()
            ).with_description("Memory-enhanced query prompt".to_string()),
            
            PromptTemplate::new(
                "code_generation".to_string(),
                "Generate code for the following task: {{task}}\n\nProgramming language: {{language}}\n\nRequirements:\n{{requirements}}".to_string()
            ).with_description("Code generation prompt".to_string()),
            
            PromptTemplate::new(
                "code_review".to_string(),
                "Review the following code:\n\n```{{language}}\n{{code}}\n```\n\nFocus on: {{focus_areas}}".to_string()
            ).with_description("Code review prompt".to_string()),
            
            PromptTemplate::new(
                "summarization".to_string(),
                "Summarize the following content in {{length}} words:\n\n{{content}}".to_string()
            ).with_description("Text summarization prompt".to_string()),
            
            PromptTemplate::new(
                "question_answering".to_string(),
                "Based on the following context:\n{{context}}\n\nAnswer the question: {{question}}".to_string()
            ).with_description("Question answering prompt".to_string()),
        ];

        for template in default_templates {
            self.templates.insert(template.name.clone(), template);
        }

        let default_system_prompts = vec![
            ("default".to_string(), "You are a helpful AI assistant.".to_string()),
            ("coding".to_string(), "You are an expert programmer. Write clean, efficient, and well-documented code.".to_string()),
            ("creative".to_string(), "You are a creative writing assistant. Help users with storytelling, poetry, and other creative endeavors.".to_string()),
            ("analytical".to_string(), "You are an analytical assistant. Break down complex problems and provide logical, data-driven insights.".to_string()),
        ];

        for (name, prompt) in default_system_prompts {
            self.system_prompts.insert(name, prompt);
        }
    }

    pub fn register_template(&mut self, template: PromptTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    pub fn register_system_prompt(&mut self, name: String, prompt: String) {
        self.system_prompts.insert(name, prompt);
    }

    pub fn get_template(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(name)
    }

    pub fn get_system_prompt(&self, name: &str) -> Option<&String> {
        self.system_prompts.get(name)
    }

    pub fn render_template(&self, name: &str, context: &HashMap<String, String>) -> Result<String> {
        let template = self.get_template(name)
            .with_context(|| format!("Template '{}' not found", name))?;
        template.render(context)
    }

    pub fn create_prompt(&self, template_name: &str, context: &HashMap<String, String>) -> Result<String> {
        self.render_template(template_name, context)
    }

    pub fn list_templates(&self) -> Vec<&PromptTemplate> {
        self.templates.values().collect()
    }

    pub fn list_system_prompts(&self) -> Vec<(&String, &String)> {
        self.system_prompts.iter().collect()
    }
}

impl Default for PromptManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_extraction() {
        let template = "Hello {{name}}, you are a {{role}}";
        let vars = PromptTemplate::extract_variables(template);
        assert_eq!(vars, vec!["name", "role"]);
    }

    #[test]
    fn test_template_rendering() {
        let template = PromptTemplate::new(
            "test".to_string(),
            "Hello {{name}}, welcome to {{place}}!".to_string()
        );
        
        let mut context = HashMap::new();
        context.insert("name".to_string(), "Alice".to_string());
        context.insert("place".to_string(), "Wonderland".to_string());
        
        let result = template.render(&context).unwrap();
        assert_eq!(result, "Hello Alice, welcome to Wonderland!");
    }
}
