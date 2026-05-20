#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    api::http::{ClientBuilder, ResponseType},
    Manager,
};
use ai_agent::{Agent, AgentConfig, llm::LLMConfig};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

struct AppState {
    agent: Mutex<Option<Agent>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatRequest {
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatResponse {
    response: String,
    stats: Option<AgentStatsResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AgentStatsResponse {
    short_term: usize,
    long_term: usize,
    skills: usize,
}

#[tauri::command]
async fn chat(state: tauri::State<'_, AppState>, request: ChatRequest) -> Result<ChatResponse, String> {
    let mut agent_guard = state.agent.lock().map_err(|e| e.to_string())?;
    
    if agent_guard.is_none() {
        let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "demo".to_string());
        
        let config = AgentConfig::new(
            "Assistant".to_string(),
            LLMConfig::openai(api_key, Some("gpt-4".to_string())),
        )
        .with_role("You are a helpful AI assistant.".to_string())
        .with_memory_capacity(100, 1000);
        
        *agent_guard = Some(Agent::new(config).map_err(|e| e.to_string())?);
    }
    
    let agent = agent_guard.as_mut().ok_or("Agent not initialized")?;
    
    match agent.chat(&request.message).await {
        Ok(response) => {
            let stats = agent.get_stats();
            Ok(ChatResponse {
                response,
                stats: Some(AgentStatsResponse {
                    short_term: stats.short_term_memory_size,
                    long_term: stats.long_term_memory_size,
                    skills: stats.available_skills,
                }),
            })
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
async fn clear_memory(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut agent_guard = state.agent.lock().map_err(|e| e.to_string())?;
    
    if let Some(agent) = agent_guard.as_mut() {
        agent.clear_memory();
    }
    
    Ok(())
}

#[tauri::command]
async fn get_stats(state: tauri::State<'_, AppState>) -> Result<AgentStatsResponse, String> {
    let agent_guard = state.agent.lock().map_err(|e| e.to_string())?;
    
    if let Some(agent) = agent_guard.as_ref() {
        let stats = agent.get_stats();
        Ok(AgentStatsResponse {
            short_term: stats.short_term_memory_size,
            long_term: stats.long_term_memory_size,
            skills: stats.available_skills,
        })
    } else {
        Ok(AgentStatsResponse {
            short_term: 0,
            long_term: 0,
            skills: 5,
        })
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            agent: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![chat, clear_memory, get_stats])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
