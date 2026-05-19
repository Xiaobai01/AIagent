/// 示例：基本的 Agent 对话
/// 
/// 运行此示例需要设置 OPENAI_API_KEY 环境变量
/// 
/// ```bash
/// export OPENAI_API_KEY="your-api-key"
/// cargo run --example basic_chat
/// ```

use ai_agent::{Agent, AgentConfig, llm::LLMConfig};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    ai_agent::utils::init_logging_with_level(tracing::Level::INFO);

    println!("🤖 Basic Chat Example\n");

    // 创建 Agent 配置
    let config = AgentConfig::new(
        "CodeAssistant".to_string(),
        LLMConfig::openai(
            std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "demo".to_string()),
            Some("gpt-4".to_string())
        ),
    )
    .with_role("You are an expert programming assistant.".to_string())
    .with_capabilities(
        "- Help with coding tasks\n\
         - Explain code and concepts\n\
         - Debug issues\n\
         - Suggest best practices"
            .to_string()
    )
    .with_constraints(
        "- Write clean, efficient code\n\
         - Include explanations\n\
         - Follow language conventions"
            .to_string()
    )
    .with_memory_capacity(50, 500);

    // 创建 Agent
    let mut agent = Agent::new(config)?;

    println!("Agent initialized: {}\n", agent.get_stats());

    // 示例对话
    let questions = vec![
        "什么是 Rust 的所有权系统？",
        "能给我一个示例代码吗？",
        "所有权和借用有什么区别？",
    ];

    for (i, question) in questions.iter().enumerate() {
        println!("Question {}: {}\n", i + 1, question);
        
        match agent.chat(question).await {
            Ok(response) => {
                println!("Answer:\n{}\n", response);
            }
            Err(e) => {
                println!("Error: {}\n", e);
            }
        }
    }

    // 查看记忆统计
    println!("Final stats: {}", agent.get_stats());

    Ok(())
}
