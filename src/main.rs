use ai_agent::{
    Agent, AgentConfig,
    llm::LLMConfig,
    skills::SkillConfig,
    cron::{CronManager, ScheduledTask, TaskType, CronSchedule},
    utils,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    path::Path,
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

#[derive(Debug, Deserialize)]
struct PlanRequest {
    goal: String,
}

#[derive(Debug, Deserialize)]
struct AddSkillRequest {
    name: String,
    description: String,
    command: Option<String>,
    code: Option<String>,
    interpreter: Option<String>,
    parameters: serde_json::Value,
    timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct CreateTaskRequest {
    name: String,
    description: String,
    schedule: String,
    task_type: TaskTypeRequest,
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct TaskTypeRequest {
    #[serde(rename = "type")]
    task_type: String,
    message: Option<String>,
    skill_name: Option<String>,
    params: Option<serde_json::Value>,
    command: Option<String>,
}

fn create_llm_config() -> LLMConfig {
    let provider = std::env::var("LLM_PROVIDER")
        .unwrap_or_else(|_| "ollama".to_string());

    match provider.as_str() {
        "openai" => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .unwrap_or_else(|_| "your-api-key".to_string());
            let model = std::env::var("OPENAI_MODEL")
                .ok();
            println!("🔄 使用 OpenAI (模型: {:?})", model.as_ref().unwrap_or(&"gpt-4".to_string()));
            LLMConfig::openai(api_key, model)
        }
        "ollama" => {
            let model = std::env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "llama3".to_string());
            let base_url = std::env::var("OLLAMA_BASE_URL").ok();
            println!("🔄 使用 Ollama (模型: {}, 地址: {})",
                model,
                base_url.as_ref().unwrap_or(&"http://localhost:11434".to_string())
            );
            LLMConfig::ollama(model, base_url)
        }
        _ => {
            println!("⚠️ 未知 provider '{}', 使用默认 Ollama", provider);
            let model = std::env::var("OLLAMA_MODEL")
                .unwrap_or_else(|_| "llama3".to_string());
            LLMConfig::ollama(model, None)
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    utils::init_logging_with_level(tracing::Level::INFO);

    let args: Vec<String> = std::env::args().collect();

    let llm_config = create_llm_config();

    let config = AgentConfig::new(
        "Assistant".to_string(),
        llm_config,
    )
    .with_role("You are a helpful AI assistant with access to various tools and skills.".to_string())
    .with_capabilities(
        "- Answer questions and provide information\n\
         - Execute tools like file operations, HTTP requests, calculations\n\
         - Help with coding and technical tasks\n\
         - Remember conversation context and important information\n\
         - Support scheduled tasks and automation"
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
            "skills" => {
                let skills = agent.get_skills();
                println!("\nAvailable Skills:");
                for skill in skills {
                    println!("- {}: {}", skill.name, skill.description);
                }
                println!();
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
    let agent_for_cron = Arc::clone(&shared_agent);

    let cron_manager = Arc::new(CronManager::new(move |task| {
        let agent_clone = Arc::clone(&agent_for_cron);
        let task_clone = task.clone();
        Box::pin(async move {
            match &task_clone.task_type {
                TaskType::ChatMessage { message } => {
                    let mut agent_guard = agent_clone.lock().await;
                    agent_guard.chat(message).await
                }
                TaskType::SkillExecution { skill_name, params } => {
                    let agent_guard = agent_clone.lock().await;
                    agent_guard.get_skill_manager().execute_skill(skill_name, params.clone()).await
                        .map(|result| result.result)
                }
                TaskType::CustomCommand { command } => {
                    let output = tokio::process::Command::new("sh")
                        .arg("-c")
                        .arg(command)
                        .output()
                        .await?;
                    if output.status.success() {
                        Ok(String::from_utf8_lossy(&output.stdout).to_string())
                    } else {
                        Ok(format!(
                            "Command failed: {}\nStderr: {}",
                            String::from_utf8_lossy(&output.stdout),
                            String::from_utf8_lossy(&output.stderr)
                        ))
                    }
                }
            }
        })
    }));

    cron_manager.start();
    println!("⏰ Cron scheduler started");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    println!("🚀 Server running on http://0.0.0.0:8080");
    println!("📡 Frontend can connect to this endpoint");

    loop {
        let (socket, _) = listener.accept().await?;
        let agent_clone = Arc::clone(&shared_agent);
        let cron_clone = Arc::clone(&cron_manager);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, agent_clone, cron_clone).await {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
}

async fn handle_connection(
    socket: tokio::net::TcpStream,
    agent: Arc<Mutex<Agent>>,
    cron_manager: Arc<CronManager>,
) -> Result<()> {
    let (mut reader, mut writer) = tokio::io::split(socket);

    let mut buffer = [0; 4096];
    let n = reader.read(&mut buffer).await?;

    if n == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..n]);

    let (method, path, body) = parse_request(&request);

    if method == "OPTIONS" {
        writer.write_all(cors_response().as_bytes()).await?;
        writer.flush().await?;
        return Ok(());
    }

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
        "/models" => {
            let result = get_ollama_models().await;
            match result {
                Ok(models) => format_response(200, &models),
                Err(e) => format_response(500, &format!("{{\"error\": \"{}\"}}", e)),
            }
        }
        "/skills" => {
            let agent_guard = agent.lock().await;
            let skills = agent_guard.get_skills();
            format_response(200, &serde_json::to_string(&skills)?)
        }
        "/add_skill" => {
            if let Ok(req) = serde_json::from_str::<AddSkillRequest>(&body) {
                let mut config = SkillConfig::new(req.name, req.description, req.parameters);
                
                if let Some(timeout) = req.timeout_secs {
                    config = config.with_timeout(timeout);
                }
                
                if let Some(command) = req.command {
                    config = config.with_command(command);
                } else if let Some(code) = req.code {
                    config = config.with_code(code);
                } else {
                    return Ok(writer.write_all(format_response(400, "{\"error\": \"Either command or code must be provided\"}").as_bytes()).await?);
                }
                
                let mut agent_guard = agent.lock().await;
                match agent_guard.add_skill(config).await {
                    Ok(_) => format_response(200, "{\"message\": \"Skill added successfully\"}"),
                    Err(e) => format_response(500, &format!("{{\"error\": \"{}\"}}", e)),
                }
            } else {
                format_response(400, "{\"error\": \"Invalid skill config\"}")
            }
        }
        "/plan" => {
            let mut agent_guard = agent.lock().await;
            if let Ok(request) = serde_json::from_str::<PlanRequest>(&body) {
                match agent_guard.create_plan(&request.goal).await {
                    Ok(plan) => format_response(200, &serde_json::to_string(&plan)?),
                    Err(e) => format_response(500, &format!("{{\"error\": \"{}\"}}", e)),
                }
            } else {
                format_response(400, "{\"error\": \"Invalid plan request\"}")
            }
        }
        "/reflect" => {
            let mut agent_guard = agent.lock().await;
            match agent_guard.reflect().await {
                Ok(reflection) => format_response(200, &serde_json::to_string(&reflection)?),
                Err(e) => format_response(500, &format!("{{\"error\": \"{}\"}}", e)),
            }
        }
        "/summarize" => {
            let agent_guard = agent.lock().await;
            match agent_guard.summarize().await {
                Ok(summary) => format_response(200, &serde_json::to_string(&summary)?),
                Err(e) => format_response(500, &format!("{{\"error\": \"{}\"}}", e)),
            }
        }
        "/tasks" => {
            let tasks = cron_manager.list_tasks();
            format_response(200, &serde_json::to_string(&tasks)?)
        }
        "/add_task" => {
            if let Ok(req) = serde_json::from_str::<CreateTaskRequest>(&body) {
                let schedule = match CronSchedule::parse(&req.schedule) {
                    Ok(s) => s,
                    Err(e) => return Ok(writer.write_all(format_response(400, &format!("{{\"error\": \"{}\"}}", e)).as_bytes()).await?),
                };

                let task_type = match req.task_type.task_type.as_str() {
                    "chat" => {
                        let message = req.task_type.message.ok_or_else(|| anyhow::anyhow!("message is required for chat task"))?;
                        TaskType::ChatMessage { message }
                    }
                    "skill" => {
                        let skill_name = req.task_type.skill_name.ok_or_else(|| anyhow::anyhow!("skill_name is required for skill task"))?;
                        let params = req.task_type.params.unwrap_or(serde_json::json!({}));
                        TaskType::SkillExecution { skill_name, params }
                    }
                    "command" => {
                        let command = req.task_type.command.ok_or_else(|| anyhow::anyhow!("command is required for command task"))?;
                        TaskType::CustomCommand { command }
                    }
                    _ => return Ok(writer.write_all(format_response(400, "{\"error\": \"Invalid task type\"}").as_bytes()).await?),
                };

                let task = ScheduledTask {
                    id: format!("task-{}", uuid::Uuid::new_v4()),
                    name: req.name,
                    description: req.description,
                    schedule,
                    task_type,
                    parameters: serde_json::json!({}),
                    enabled: req.enabled,
                    last_run: None,
                    next_run: None,
                    run_count: 0,
                    last_result: None,
                };

                match cron_manager.add_task(task) {
                    Ok(_) => format_response(200, "{\"message\": \"Task added successfully\"}"),
                    Err(e) => format_response(500, &format!("{{\"error\": \"{}\"}}", e)),
                }
            } else {
                format_response(400, "{\"error\": \"Invalid task request\"}")
            }
        }
        "/remove_task" => {
            let task_id: Result<serde_json::Value, _> = serde_json::from_str(&body);
            if let Ok(json) = task_id {
                if let Some(id) = json["task_id"].as_str() {
                    match cron_manager.remove_task(id) {
                        Ok(_) => format_response(200, "{\"message\": \"Task removed successfully\"}"),
                        Err(e) => format_response(404, &format!("{{\"error\": \"{}\"}}", e)),
                    }
                } else {
                    format_response(400, "{\"error\": \"Missing task_id\"}")
                }
            } else {
                format_response(400, "{\"error\": \"Invalid request\"}")
            }
        }
        "/update_task" => {
            if let Ok(task) = serde_json::from_str::<ScheduledTask>(&body) {
                match cron_manager.update_task(task) {
                    Ok(_) => format_response(200, "{\"message\": \"Task updated successfully\"}"),
                    Err(e) => format_response(500, &format!("{{\"error\": \"{}\"}}", e)),
                }
            } else {
                format_response(400, "{\"error\": \"Invalid task data\"}")
            }
        }
        "/task_info" => {
            let task_id: Result<serde_json::Value, _> = serde_json::from_str(&body);
            if let Ok(json) = task_id {
                if let Some(id) = json["task_id"].as_str() {
                    match cron_manager.get_task(id) {
                        Some(task) => format_response(200, &serde_json::to_string(&task)?),
                        None => format_response(404, "{\"error\": \"Task not found\"}"),
                    }
                } else {
                    format_response(400, "{\"error\": \"Missing task_id\"}")
                }
            } else {
                format_response(400, "{\"error\": \"Invalid request\"}")
            }
        }
        path if path.starts_with("/static/") => {
            let file_path = &path[1..];
            if let Ok(content) = fs::read_to_string(file_path) {
                let content_type = if file_path.ends_with(".html") {
                    "text/html"
                } else if file_path.ends_with(".css") {
                    "text/css"
                } else if file_path.ends_with(".js") {
                    "application/javascript"
                } else if file_path.ends_with(".json") {
                    "application/json"
                } else {
                    "text/plain"
                };
                format_response_with_content_type(200, &content, content_type)
            } else {
                format_response(404, "{\"error\": \"File not found\"}")
            }
        }
        "/" => {
            if let Ok(content) = fs::read_to_string("static/index.html") {
                format_response_with_content_type(200, &content, "text/html")
            } else {
                format_response(404, "{\"error\": \"Index not found\"}")
            }
        }
        _ => format_response(404, "{\"error\": \"Not found\"}"),
    };

    writer.write_all(response.as_bytes()).await?;
    writer.flush().await?;

    Ok(())
}

fn parse_request(request: &str) -> (&str, &str, &str) {
    let lines: Vec<&str> = request.split("\r\n").collect();

    if lines.is_empty() {
        return ("GET", "/", "");
    }

    let first_line = lines[0];
    let parts: Vec<&str> = first_line.split_whitespace().collect();

    let method = if parts.len() > 0 { parts[0] } else { "GET" };
    let path = if parts.len() > 1 { parts[1] } else { "/" };

    let body_start = request.find("\r\n\r\n");
    let body = if let Some(pos) = body_start {
        &request[pos + 4..]
    } else {
        ""
    };

    (method, path, body)
}

fn cors_response() -> String {
    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: 0\r\n\r\n".to_string()
}

async fn get_ollama_models() -> Result<String> {
    let client = reqwest::Client::new();
    let response = client.get("http://localhost:11434/api/tags")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch models from Ollama");
    }

    let result: serde_json::Value = response.json().await?;
    Ok(serde_json::to_string(&result)?)
}

fn format_response(status: u16, body: &str) -> String {
    format_response_with_content_type(status, body, "application/json")
}

fn format_response_with_content_type(status: u16, body: &str, content_type: &str) -> String {
    format!(
        "HTTP/1.1 {} OK\r\nContent-Type: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: {}\r\n\r\n{}",
        status,
        content_type,
        body.len(),
        body
    )
}
