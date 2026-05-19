# AI Agent 快速参考指南

## 快速开始

### 1. 安装依赖
```bash
# 确保已安装 Rust (1.70+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆或进入项目目录
cd AIagent
```

### 2. 配置 API Key
```bash
export OPENAI_API_KEY="sk-..."
```

### 3. 运行示例
```bash
# 运行交互式 CLI
cargo run

# 运行基本对话示例
cargo run --example basic_chat

# 运行自定义技能示例
cargo run --example custom_skills

# 运行 Ollama 本地模型示例
cargo run --example ollama_example
```

## 常用代码片段

### 创建基础 Agent
```rust
use ai_agent::{Agent, AgentConfig, llm::LLMConfig};

let config = AgentConfig::new(
    "Assistant".to_string(),
    LLMConfig::openai(api_key, Some("gpt-4".to_string())),
);

let mut agent = Agent::new(config)?;
let response = agent.chat("Hello!").await?;
```

### 使用 Ollama 本地模型
```rust
let config = LLMConfig::ollama(
    "llama2".to_string(),
    Some("http://localhost:11434".to_string())
);
```

### 自定义 Agent 角色
```rust
let config = AgentConfig::new(name, llm_config)
    .with_role("You are an expert programmer.".to_string())
    .with_capabilities("- Coding\n- Debugging\n- Code Review".to_string())
    .with_constraints("- Write clean code\n- Include tests".to_string())
    .with_memory_capacity(100, 1000)
    .with_max_iterations(10);
```

### 添加自定义技能
```rust
use ai_agent::skills::FunctionSkill;

let skill = FunctionSkill::new(
    "my_skill".to_string(),
    "Description of what it does".to_string(),
    serde_json::json!({
        "type": "object",
        "properties": {
            "param1": {"type": "string"}
        },
        "required": ["param1"]
    }),
    |params| {
        let param1 = params["param1"].as_str().unwrap();
        Ok(format!("Result: {}", param1))
    }
);

skill_manager.register_skill(Arc::new(skill));
```

### 使用提示词模板
```rust
use ai_agent::prompts::PromptTemplate;

let template = PromptTemplate::new(
    "my_template".to_string(),
    "Hello {{name}}, you are a {{role}}!".to_string()
);

let mut context = HashMap::new();
context.insert("name", "Alice");
context.insert("role", "hero");

let result = template.render(&context)?;
```

### 查看 Agent 状态
```rust
let stats = agent.get_stats();
println!("Short-term memory: {}", stats.short_term_memory_size);
println!("Long-term memory: {}", stats.long_term_memory_size);
println!("Available skills: {}", stats.available_skills);
println!("Model: {} ({})", stats.model, stats.provider);
```

### 清空记忆
```rust
agent.clear_memory();
```

## 内置技能列表

| 技能名称 | 描述 | 参数 |
|---------|------|------|
| `read_file` | 读取文件 | `path: string` |
| `write_file` | 写入文件 | `path: string`, `content: string` |
| `list_directory` | 列出目录 | `path: string` |
| `http_get` | HTTP GET 请求 | `url: string` |
| `calculate` | 数学计算 | `expression: string` |

## 环境变量

| 变量 | 描述 | 示例 |
|------|------|------|
| `OPENAI_API_KEY` | OpenAI API 密钥 | `sk-...` |
| `ANTHROPIC_API_KEY` | Anthropic API 密钥 | `sk-ant-...` |
| `RUST_LOG` | 日志级别 | `debug`, `info`, `warn` |

## 常见问题

### Q: 如何切换到其他模型？
```rust
// GPT-3.5
LLMConfig::openai(api_key, Some("gpt-3.5-turbo".to_string()))

// Claude 3
LLMConfig::anthropic(api_key, Some("claude-3-sonnet-20240229".to_string()))

// 本地 Llama2
LLMConfig::ollama("llama2".to_string(), None)
```

### Q: 如何禁用某个内置技能？
创建自定义 SkillManager，只注册需要的技能：
```rust
let mut skill_manager = SkillManager::new();
// 不注册某些技能即可禁用
```

### Q: 如何增加上下文长度？
```rust
let config = AgentConfig::new(name, llm_config)
    .with_memory_capacity(200, 2000); // 增加记忆容量
```

### Q: 如何保存和加载记忆？
目前记忆存储在内存中，可以通过序列化保存：
```rust
use serde_json;

// 保存
let memory_data = serde_json::to_string(&agent.get_memory())?;
std::fs::write("memory.json", memory_data)?;

// 加载
let memory_data = std::fs::read_to_string("memory.json")?;
let memory: MemoryManager = serde_json::from_str(&memory_data)?;
```

### Q: 如何调试 Agent 行为？
启用详细日志：
```bash
export RUST_LOG=debug
cargo run
```

查看日志输出，了解：
- 每次迭代的决策
- 工具调用详情
- 记忆存取情况

## 性能调优

### 减少响应时间
```rust
let config = AgentConfig::new(name, llm_config)
    .with_max_iterations(5); // 减少最大迭代次数
```

### 优化记忆检索
```rust
// 定期清理长期记忆
agent.get_mut_memory().long_term.consolidate();
```

### 并发执行工具
（需要修改源码）在 `process_tool_calls` 中使用 `tokio::join!` 并发执行多个工具调用。

## 最佳实践

1. **合理设置记忆容量**: 根据使用场景调整，避免内存占用过高
2. **使用有意义的角色描述**: 详细的角色定义能获得更好的响应
3. **限制危险操作**: 谨慎启用文件写入、命令执行等技能
4. **监控迭代次数**: 设置合理的 max_iterations 防止无限循环
5. **定期清理记忆**: 长时间运行时定期调用 consolidate_memory()

## 故障排除

### 问题：编译失败
```bash
# 更新 Rust 工具链
rustup update

# 清理构建缓存
cargo clean

# 重新构建
cargo build
```

### 问题：API 请求失败
- 检查 API key 是否正确
- 检查网络连接
- 查看日志输出：`RUST_LOG=debug cargo run`

### 问题：工具执行失败
- 检查参数格式是否正确
- 确认权限是否足够
- 查看错误信息定位问题

## 学习路径

1. **入门**: 运行 `cargo run` 体验基本对话
2. **理解**: 阅读 `examples/basic_chat.rs` 了解 API 使用
3. **扩展**: 阅读 `examples/custom_skills.rs` 学习添加技能
4. **深入**: 阅读 `ARCHITECTURE.md` 理解架构设计
5. **贡献**: 查看 GitHub issues 参与项目开发

## 相关资源

- [Rust 官方文档](https://doc.rust-lang.org/book/)
- [Tokio 异步编程](https://tokio.rs/tokio/tutorial)
- [OpenAI API 文档](https://platform.openai.com/docs)
- [Ollama 文档](https://ollama.ai/help)

## 获取帮助

- 查看 README.md 了解完整功能
- 查看 ARCHITECTURE.md 了解架构设计
- 查看 examples/ 目录了解使用示例
- 提交 Issue 反馈问题
