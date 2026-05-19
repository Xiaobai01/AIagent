use ai_agent::{
    Agent, AgentConfig,
    llm::LLMConfig,
    utils,
};
use anyhow::Result;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<()> {
    utils::init_logging_with_level(tracing::Level::INFO);
    
    println!("🤖 AI Agent Framework");
    println!("====================\n");
    
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
    
    let mut agent = Agent::new(config)?;
    
    println!("Agent initialized successfully!");
    println!("{}\n", agent.get_stats());
    
    println!("Enter your messages below (type 'quit' to exit, 'stats' to see statistics, 'clear' to clear memory):\n");
    
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
