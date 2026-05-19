/// 示例：使用 Ollama 本地模型
/// 
/// 需要先安装并运行 Ollama:
/// https://ollama.ai
/// 
/// ```bash
/// ollama serve
/// ollama run llama2
/// ```

use ai_agent::{Agent, AgentConfig, llm::LLMConfig};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    ai_agent::utils::init_logging_with_level(tracing::Level::INFO);

    println!("🦙 Ollama Local Model Example\n");
    println!("Make sure Ollama is running: ollama serve\n");

    // 配置 Ollama
    let config = AgentConfig::new(
        "LocalAssistant".to_string(),
        LLMConfig::ollama(
            "llama2".to_string(),
            Some("http://localhost:11434".to_string())
        ),
    )
    .with_role("You are a helpful assistant.".to_string())
    .with_memory_capacity(50, 200);

    let mut agent = Agent::new(config)?;

    println!("Agent initialized with Ollama model\n");

    // 简单对话
    let questions = vec![
        "Hello! Who are you?",
        "What can you help me with?",
    ];

    for question in questions {
        println!("Q: {}", question);
        
        match agent.chat(question).await {
            Ok(response) => {
                println!("A: {}\n", response);
            }
            Err(e) => {
                println!("Error: {}\n", e);
                println!("Make sure Ollama is running and llama2 model is available.");
                break;
            }
        }
    }

    Ok(())
}
