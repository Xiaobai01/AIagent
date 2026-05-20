use ai_agent::{
    Agent, AgentConfig,
    llm::LLMConfig,
    utils,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Write},
    sync::Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
};

#[derive(Debug, Serialize, Deserialize)]
struct ChatRequest {
    message: String,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    response: String,
    stats: Option<AgentStatsResponse>,
}

#[derive(Debug, Serialize)]
struct AgentStatsResponse {
    short_term: usize,
    long_term: usize,
    skills: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    utils::init_logging_with_level(tracing::Level::INFO);
    
    let args: Vec<String> = std::env::args().collect();
    
    let api_key = std::env::var("OPENAI_API_KEY")
        .unwrap_or_else(|_| "your-api-key".to_string());
    
    let config = AgentConfig::new(
        "Assistant".to_string(),
        LLMConfig::openai(api_key, Some("gpt-4".to_string())),
    )
    .with_role("You are a helpful AI assistant with access to various tools and skills.".to_string())
    .with_capabilities(
        "- Answer questions and provide information\n\
         - Execute tools like file operations, HTTP requests, calculations\n\
         - Help with coding and technical tasks\n\
         - Remember conversation context and important information"
            .to_string()
    )
    .with_constraints(
        "- Be helpful, harmless, and honest\n\
         - Don't execute dangerous or destructive operations\n\
         - Ask for clarification when instructions are ambiguous\n\
         - Use tools when they can help accomplish tasks"
            .to_string()
    )
    .with_memory_capacity(100, 1000)
    .with_max_iterations(10);
    
    let agent = Agent::new(config)?;
    
    if args.len() > 1 && args[1] == "server" {
        run_server(agent).await?;
        return Ok(());
    }
    
    println!("🤖 AI Agent Framework");
    println!("====================\n");
    
    println!("Agent initialized successfully!");
    println!("{}\n", agent.get_stats());
    
    println!("Enter your messages below (type 'quit' to exit, 'stats' to see statistics, 'clear' to clear memory):\n");
    
    let mut agent = agent;
    
    loop {
        print!("👤 You: ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        match input.to_lowercase().as_str() {
            "quit" | "exit" | "q" => {
                println!("\nGoodbye!");
                break;
            }
            "stats" => {
                println!("\n{}", agent.get_stats());
                continue;
            }
            "clear" => {
                agent.clear_memory();
                println!("Memory cleared!\n");
                continue;
            }
            _ => {}
        }
        
        match agent.chat(input).await {
            Ok(response) => {
                println!("\n🤖 Agent: {}\n", response);
            }
            Err(e) => {
                println!("\n❌ Error: {}\n", e);
            }
        }
    }
    
    Ok(())
}

async fn run_server(agent: Agent) -> Result<()> {
    let shared_agent = Arc::new(Mutex::new(agent));
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    println!("🚀 Server running on http://0.0.0.0:8080");
    println!("📡 Frontend can connect to this endpoint");
    
    loop {
        let (socket, _) = listener.accept().await?;
        let agent_clone = Arc::clone(&shared_agent);
        
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, agent_clone).await {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
}

async fn handle_connection(
    socket: tokio::net::TcpStream,
    agent: Arc<Mutex<Agent>>,
) -> Result<()> {
    let (mut reader, mut writer) = tokio::io::split(socket);
    
    let mut buffer = [0; 4096];
    let n = reader.read(&mut buffer).await?;
    
    if n == 0 {
        return Ok(());
    }
    
    let request = String::from_utf8_lossy(&buffer[..n]);
    
    let (path, body) = parse_request(&request);
    
    let response = match path {
        "/chat" => {
            if let Ok(req) = serde_json::from_str::<ChatRequest>(&body) {
                let mut agent_guard = agent.lock().await;
                let response = agent_guard.chat(&req.message).await;
                let stats = agent_guard.get_stats();
                
                match response {
                    Ok(response) => {
                        let resp = ChatResponse {
                            response,
                            stats: Some(AgentStatsResponse {
                                short_term: stats.short_term_memory_size,
                                long_term: stats.long_term_memory_size,
                                skills: stats.available_skills,
                            }),
                        };
                        format_response(200, &serde_json::to_string(&resp)?)
                    }
                    Err(e) => {
                        let resp = ChatResponse {
                            response: format!("Error: {}", e),
                            stats: None,
                        };
                        format_response(500, &serde_json::to_string(&resp)?)
                    }
                }
            } else {
                format_response(400, "{\"error\": \"Invalid request\"}")
            }
        }
        "/clear" => {
            agent.lock().await.clear_memory();
            format_response(200, "{\"message\": \"Memory cleared\"}")
        }
        "/stats" => {
            let agent_guard = agent.lock().await;
            let stats = agent_guard.get_stats();
            let resp = AgentStatsResponse {
                short_term: stats.short_term_memory_size,
                long_term: stats.long_term_memory_size,
                skills: stats.available_skills,
            };
            format_response(200, &serde_json::to_string(&resp)?)
        }
        "/health" => format_response(200, "{\"status\": \"OK\"}"),
        _ => format_response(404, "{\"error\": \"Not found\"}"),
    };
    
    writer.write_all(response.as_bytes()).await?;
    writer.flush().await?;
    
    Ok(())
}

fn parse_request(request: &str) -> (&str, &str) {
    let lines: Vec<&str> = request.split("\r\n").collect();
    
    if lines.is_empty() {
        return ("/", "");
    }
    
    let first_line = lines[0];
    let parts: Vec<&str> = first_line.split_whitespace().collect();
    
    let path = if parts.len() > 1 { parts[1] } else { "/" };
    
    let body_start = request.find("\r\n\r\n");
    let body = if let Some(pos) = body_start {
        &request[pos + 4..]
    } else {
        ""
    };
    
    (path, body)
}

fn format_response(status: u16, body: &str) -> String {
    format!(
        "HTTP/1.1 {} OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: {}\r\n\r\n{}",
        status,
        body.len(),
        body
    )
}
