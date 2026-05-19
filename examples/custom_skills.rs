/// 示例：使用自定义 Skills
/// 
/// 展示如何扩展 Agent 的技能系统

use ai_agent::{
    Agent, AgentConfig,
    llm::LLMConfig,
    skills::{SkillManager, FunctionSkill},
};
use anyhow::Result;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    ai_agent::utils::init_logging_with_level(tracing::Level::INFO);

    println!("🛠️  Custom Skills Example\n");

    // 创建自定义技能：天气查询
    let weather_skill = FunctionSkill::new(
        "get_weather".to_string(),
        "Get current weather for a city".to_string(),
        serde_json::json!({
            "type": "object",
            "properties": {
                "city": {
                    "type": "string",
                    "description": "City name"
                },
                "unit": {
                    "type": "string",
                    "description": "Temperature unit (celsius or fahrenheit)",
                    "enum": ["celsius", "fahrenheit"]
                }
            },
            "required": ["city"]
        }),
        |params| {
            let city = params["city"].as_str().unwrap_or("Unknown");
            let unit = params["unit"].as_str().unwrap_or("celsius");
            
            // 模拟天气数据
            let temp = match unit {
                "fahrenheit" => 77,
                _ => 25,
            };
            
            Ok(format!(
                "Weather in {}: {}°{}, Sunny, Humidity: 45%",
                city, temp, if unit == "fahrenheit" { "F" } else { "C" }
            ))
        }
    );

    // 创建自定义技能：时间查询
    let time_skill = FunctionSkill::new(
        "get_current_time".to_string(),
        "Get current time in a specific timezone".to_string(),
        serde_json::json!({
            "type": "object",
            "properties": {
                "timezone": {
                    "type": "string",
                    "description": "Timezone (e.g., UTC, America/New_York, Asia/Shanghai)"
                }
            },
            "required": ["timezone"]
        }),
        |params| {
            let timezone = params["timezone"].as_str().unwrap_or("UTC");
            let now = chrono::Utc::now();
            
            // 简化的时区处理（实际使用应该用时区库）
            Ok(format!(
                "Current time in {}: {}",
                timezone,
                now.format("%Y-%m-%d %H:%M:%S UTC")
            ))
        }
    );

    // 创建技能管理器并注册自定义技能
    let mut skill_manager = SkillManager::new();
    skill_manager.register_skill(Arc::new(weather_skill));
    skill_manager.register_skill(Arc::new(time_skill));

    // 创建 Agent 并使用自定义技能管理器
    let config = AgentConfig::new(
        "Assistant".to_string(),
        LLMConfig::openai(
            std::env::var("OPENAI_API_KEY").unwrap_or_else(|_| "demo".to_string()),
            Some("gpt-4".to_string())
        ),
    );

    let mut agent = Agent::new(config)?
        .with_skill_manager(skill_manager);

    println!("Available skills:");
    for skill in agent.get_skill_manager().list_skills() {
        println!("  - {}: {}", skill.name(), skill.description());
    }
    println!();

    // 测试自定义技能
    println!("Testing custom skills:\n");
    
    // 直接执行技能
    let weather_result = agent.get_skill_manager()
        .execute_skill("get_weather", serde_json::json!({
            "city": "Beijing",
            "unit": "celsius"
        }))
        .await?;
    
    println!("Weather skill result: {}", weather_result.result);

    let time_result = agent.get_skill_manager()
        .execute_skill("get_current_time", serde_json::json!({
            "timezone": "UTC"
        }))
        .await?;
    
    println!("Time skill result: {}\n", time_result.result);

    // 注意：要让 LLM 自动调用这些技能，需要在对话中提及相关内容
    // 例如："What's the weather like in Shanghai?"

    Ok(())
}
