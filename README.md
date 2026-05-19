# AI Agent Framework

一个功能完整的 Rust AI Agent 框架，具备长短期记忆、提示词管理、大模型对接和 Skill 执行等核心能力。

## 特性

### 核心模块

1. **记忆模块 (Memory)**
   - 短期记忆 (Short-term Memory): 基于消息队列的上下文管理
   - 长期记忆 (Long-term Memory): 支持重要性评分和相关性检索
   - 记忆整合 (Memory Consolidation): 自动清理和压缩记忆

2. **提示词模块 (Prompts)**
   - 模板化提示词管理
   - 内置常用提示词模板
   - 支持自定义提示词和变量替换

3. **大模型对接模块 (LLM)**
   - 支持 OpenAI API (GPT-4, GPT-3.5)
   - 支持 Ollama (本地模型)
   - 支持 Anthropic API (Claude)
   - 统一的 Provider 接口

4. **Skill 模块 (Skills)**
   - 内置技能：文件读写、目录列表、HTTP 请求、计算器
   - 支持自定义命令技能
   - 支持代码执行技能
   - 支持函数技能

5. **Agent 核心引擎**
   - 自主推理循环
   - 工具调用和执行
   - 多轮对话管理
   - 记忆自动存储

## 项目结构

```
ai-agent/
├── src/
│   ├── core/           # 核心数据结构和 Agent 引擎
│   │   ├── mod.rs
│   │   └── agent.rs
│   ├── memory/         # 记忆模块
│   │   └── mod.rs
│   ├── prompts/        # 提示词模块
│   │   └── mod.rs
│   ├── llm/            # 大模型对接模块
│   │   └── mod.rs
│   ├── skills/         # Skill 模块
│   │   └── mod.rs
│   ├── utils/          # 工具函数
│   │   └── mod.rs
│   ├── lib.rs          # 库入口
│   └── main.rs         # 示例程序
├── Cargo.toml
└── README.md
```

## 快速开始

### 1. 配置环境变量

```bash
export OPENAI_API_KEY="your-api-key-here"
```

### 2. 运行示例程序

```bash
cargo run
```

### 3. 编程使用

```rust
use ai_agent::{Agent, AgentConfig, llm::LLMConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建配置
    let config = AgentConfig::new(
        "MyAssistant".to_string(),
        LLMConfig::openai(
            "your-api-key".to_string(),
            Some("gpt-4".to_string())
        ),
    )
    .with_role("You are a helpful coding assistant.".to_string())
    .with_memory_capacity(100, 1000);

    // 创建 Agent
    let mut agent = Agent::new(config)?;

    // 对话
    let response = agent.chat("帮我写一个快速排序算法").await?;
    println!("{}", response);

    Ok(())
}
```

## 使用示例

### 使用不同的 LLM Provider

#### OpenAI
```rust
let config = LLMConfig::openai(api_key, Some("gpt-4".to_string()));
```

#### Ollama (本地模型)
```rust
let config = LLMConfig::ollama(
    "llama2".to_string(),
    Some("http://localhost:11434".to_string())
);
```

#### Anthropic
```rust
let config = LLMConfig::anthropic(api_key, Some("claude-3-sonnet-20240229".to_string()));
```

### 自定义 Skill

```rust
use ai_agent::skills::{SkillManager, FunctionSkill};
use std::sync::Arc;

// 创建自定义技能
let weather_skill = FunctionSkill::new(
    "get_weather".to_string(),
    "Get weather information for a city".to_string(),
    serde_json::json!({
        "type": "object",
        "properties": {
            "city": {
                "type": "string",
                "description": "The city name"
            }
        },
        "required": ["city"]
    }),
    |params| {
        let city = params["city"].as_str().unwrap();
        Ok(format!("The weather in {} is sunny, 25°C", city))
    }
);

// 注册技能
let mut skill_manager = SkillManager::new();
skill_manager.register_skill(Arc::new(weather_skill));
```

### 自定义提示词模板

```rust
use ai_agent::prompts::{PromptManager, PromptTemplate};

let mut prompt_manager = PromptManager::new();

// 添加自定义模板
let template = PromptTemplate::new(
    "code_review".to_string(),
    "Review this {{language}} code:\n{{code}}\n\nFocus on: {{focus}}".to_string()
);

prompt_manager.register_template(template);

// 使用模板
let mut context = std::collections::HashMap::new();
context.insert("language", "Rust");
context.insert("code", "fn main() { println!(\"Hello\"); }");
context.insert("focus", "performance and safety");

let prompt = prompt_manager.render_template("code_review", &context)?;
```

## 内置 Skills

框架预置了以下技能：

1. **read_file**: 读取文件内容
2. **write_file**: 写入文件内容
3. **list_directory**: 列出目录内容
4. **http_get**: 发送 HTTP GET 请求
5. **calculate**: 执行数学计算

## 记忆系统

### 短期记忆
- 存储最近的对话消息
- 可配置容量（默认 100 条）
- FIFO 队列管理

### 长期记忆
- 存储重要的对话内容
- 支持重要性评分 (0.0-1.0)
- 基于相关性的智能检索
- 自动整合和清理

## 配置选项

```rust
let config = AgentConfig::new(name, llm_config)
    .with_role("自定义角色描述")
    .with_capabilities("自定义能力列表")
    .with_constraints("自定义约束条件")
    .with_memory_capacity(100, 1000)  // 短长期记忆容量
    .with_max_iterations(10);         // 最大推理迭代次数
```

## 依赖项

主要依赖：
- tokio: 异步运行时
- serde/serde_json: 序列化
- reqwest: HTTP 客户端
- handlebars: 模板引擎
- anyhow/thiserror: 错误处理
- tracing: 日志
- chrono: 时间处理
- uuid: ID 生成

## 开发

### 编译
```bash
cargo build
```

### 测试
```bash
cargo test
```

### 格式化
```bash
cargo fmt
```

### 检查
```bash
cargo clippy
```

## 日志

设置日志级别：
```bash
export RUST_LOG=debug
cargo run
```

## 扩展

### 添加新的 LLM Provider

实现 `LLMProvider` trait:

```rust
use ai_agent::llm::{LLMProvider, ToolDefinition};
use ai_agent::core::{Message, LLMResponse};

struct MyProvider { /* ... */ }

#[async_trait]
impl LLMProvider for MyProvider {
    async fn chat(&self, messages: Vec<Message>) -> anyhow::Result<LLMResponse>;
    async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>) -> anyhow::Result<LLMResponse>;
    fn get_model(&self) -> &str;
    fn get_provider(&self) -> &str;
}
```

### 添加新的 Skill

实现 `Skill` trait:

```rust
use ai_agent::skills::Skill;
use ai_agent::core::ToolResult;

struct MySkill { /* ... */ }

#[async_trait]
impl Skill for MySkill {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> &serde_json::Value;
    async fn execute(&self, params: serde_json::Value) -> anyhow::Result<ToolResult>;
}
```

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
