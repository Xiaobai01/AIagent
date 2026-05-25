use serde::{Serialize, Deserialize};
use crate::llm::{LLMProvider, LLMConfig, create_llm_provider};
use crate::core::Message;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub id: String,
    pub content: String,
    pub confidence: f64,
    pub suggestions: Vec<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionSummary {
    pub successes: Vec<String>,
    pub failures: Vec<String>,
    pub improvements: Vec<String>,
    pub key_insights: Vec<String>,
}

pub struct Reflector {
    llm_provider: Box<dyn LLMProvider>,
    reflections: Vec<Reflection>,
}

impl Reflector {
    pub fn new(llm_config: LLMConfig) -> Result<Self> {
        let llm_provider = create_llm_provider(llm_config)?;
        Ok(Self {
            llm_provider,
            reflections: Vec::new(),
        })
    }

    pub async fn reflect_on_conversation(&mut self, messages: &[Message]) -> Result<Reflection> {
        let conversation = messages.iter()
            .map(|m| format!("{}: {}", 
                match m.role {
                    crate::core::MessageRole::System => "System",
                    crate::core::MessageRole::User => "User",
                    crate::core::MessageRole::Assistant => "Assistant",
                    crate::core::MessageRole::Tool => "Tool",
                },
                m.content
            ))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            r#"
            You are a reflective AI assistant. Analyze the following conversation and provide a thoughtful reflection.
            
            Conversation:
            {}
            
            Please provide:
            1. A summary of what was accomplished
            2. Any mistakes or errors made
            3. Suggestions for improvement
            4. Key insights gained
            
            Format your response as JSON:
            {{
                "summary": "...",
                "confidence": 0.0-1.0,
                "suggestions": ["...", "..."],
                "key_insights": ["...", "..."]
            }}
            "#,
            conversation
        );

        let messages = vec![crate::core::Message::user(prompt.clone())];
        let response = self.llm_provider.chat(messages).await?;
        let response_text = match response {
            crate::core::LLMResponse::Text(text) => text,
            _ => "{\"summary\": \"\", \"confidence\": 0.5, \"suggestions\": [], \"key_insights\": []}".to_string(),
        };
        let reflection_data: serde_json::Value = serde_json::from_str(&response_text)?;
        
        let reflection = Reflection {
            id: uuid::Uuid::new_v4().to_string(),
            content: reflection_data["summary"].as_str().unwrap_or("").to_string(),
            confidence: reflection_data["confidence"].as_f64().unwrap_or(0.5),
            suggestions: reflection_data["suggestions"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|s| s.as_str().unwrap_or("").to_string())
                .collect(),
            timestamp: chrono::Utc::now(),
        };
        
        self.reflections.push(reflection.clone());
        Ok(reflection)
    }

    pub async fn improve_response(&self, original_response: &str, user_input: &str) -> Result<String> {
        let prompt = format!(
            r#"
            You are an AI assistant helping to improve responses.
            
            User input: {}
            Original response: {}
            
            Please improve this response by:
            1. Making it more clear and concise
            2. Adding more relevant details
            3. Fixing any errors
            4. Making it more helpful
            
            Provide only the improved response.
            "#,
            user_input,
            original_response
        );
        
        let messages = vec![crate::core::Message::user(prompt.clone())];
        let response = self.llm_provider.chat(messages).await?;
        match response {
            crate::core::LLMResponse::Text(text) => Ok(text),
            _ => Ok(original_response.to_string()),
        }
    }

    pub async fn summarize_session(&self, messages: &[Message]) -> Result<ReflectionSummary> {
        let conversation = messages.iter()
            .map(|m| format!("{}: {}", 
                match m.role {
                    crate::core::MessageRole::System => "System",
                    crate::core::MessageRole::User => "User",
                    crate::core::MessageRole::Assistant => "Assistant",
                    crate::core::MessageRole::Tool => "Tool",
                },
                m.content
            ))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            r#"
            You are a summarization AI. Analyze the following conversation and provide a comprehensive summary.
            
            Conversation:
            {}
            
            Please provide:
            1. Successes: What was accomplished
            2. Failures: What didn't work
            3. Improvements: How to do better next time
            4. Key insights: Important learnings
            
            Format as JSON:
            {{
                "successes": ["...", "..."],
                "failures": ["...", "..."],
                "improvements": ["...", "..."],
                "key_insights": ["...", "..."]
            }}
            "#,
            conversation
        );

        let messages = vec![crate::core::Message::user(prompt.clone())];
        let response = self.llm_provider.chat(messages).await?;
        let response_text = match response {
            crate::core::LLMResponse::Text(text) => text,
            _ => "{\"successes\": [], \"failures\": [], \"improvements\": [], \"key_insights\": []}".to_string(),
        };
        let summary: ReflectionSummary = serde_json::from_str(&response_text)?;
        Ok(summary)
    }

    pub fn get_reflections(&self) -> &[Reflection] {
        &self.reflections
    }
}
