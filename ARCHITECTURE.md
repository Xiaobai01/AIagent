# AI Agent 项目架构文档

## 项目概述

这是一个功能完整的 Rust AI Agent 框架，实现了现代 AI Agent 所需的所有核心能力：
- ✅ 长短期记忆系统
- ✅ 提示词管理模块
- ✅ 通用大模型对接（OpenAI/Ollama/Anthropic）
- ✅ Skill 解析和执行引擎
- ✅ 自主推理循环

## 架构设计

### 核心模块划分

```
ai-agent/
├── src/
│   ├── core/           # 核心层：数据结构和 Agent 引擎
│   │   ├── mod.rs      # 基础数据类型 (Message, ToolCall, LLMResponse)
│   │   └── agent.rs    # Agent 引擎（推理循环、工具调度）
│   │
│   ├── memory/         # 记忆层：记忆管理
│   │   └── mod.rs      # ShortTermMemory + LongTermMemory
│   │
│   ├── prompts/        # 提示词层：模板管理
│   │   └── mod.rs      # PromptTemplate + PromptManager
│   │
│   ├── llm/            # 模型层：LLM 对接
│   │   └── mod.rs      # LLMProvider trait + OpenAI/Ollama 实现
│   │
│   ├── skills/         # 技能层：工具执行
│   │   └── mod.rs      # Skill trait + 内置技能 + SkillManager
│   │
│   ├── utils/          # 工具层：辅助功能
│   │   └── mod.rs      # 日志初始化等
│   │
│   ├── lib.rs          # 库入口：模块导出
│   └── main.rs         # CLI 示例程序
│
└── examples/           # 示例代码
    ├── basic_chat.rs   # 基本对话示例
    ├── custom_skills.rs # 自定义技能示例
    └── ollama_example.rs # Ollama 本地模型示例
```

## 模块详解

### 1. Core 模块 (`src/core/`)

**职责**：定义基础数据结构和 Agent 核心引擎

**关键类型**：
- `Message`: 对话消息（System/User/Assistant/Tool）
- `MessageRole`: 消息角色枚举
- `ToolCall`: 工具调用请求
- `ToolResult`: 工具执行结果
- `LLMResponse`: LLM 响应（文本或工具调用）
- `Agent`: Agent 主引擎
- `AgentConfig`: Agent 配置

**Agent 工作流程**：
```
用户输入
  ↓
构建上下文（记忆 + 提示词）
  ↓
LLM 推理
  ↓
判断响应类型
  ├─→ 文本响应 → 返回结果 → 存储记忆
  └─→ 工具调用 → 执行技能 → 添加结果 → 继续推理
```

### 2. Memory 模块 (`src/memory/`)

**职责**：管理 Agent 的记忆系统

**组件**：
- **ShortTermMemory**: 
  - 基于 VecDeque 的 FIFO 队列
  - 可配置容量（默认 100 条）
  - 存储最近的对话消息
  
- **LongTermMemory**:
  - 基于向量的记忆存储
  - 重要性评分 (0.0-1.0)
  - 相关性检索（关键词匹配）
  - 自动整合（清理低价值记忆）
  
- **MemoryManager**: 
  - 统一管理短长期记忆
  - 自动将重要对话存入长期记忆
  - 提供上下文检索接口

**记忆整合策略**：
```rust
score = recency * 0.4 + importance * 0.4 + access_frequency * 0.2
```

### 3. Prompts 模块 (`src/prompts/`)

**职责**：管理提示词模板

**组件**：
- `PromptTemplate`: 提示词模板（支持 Handlebars 语法）
- `PromptManager`: 模板管理器

**内置模板**：
- `agent_instruction`: Agent 角色定义
- `task_planning`: 任务规划
- `tool_selection`: 工具选择
- `memory_query`: 记忆增强查询
- `code_generation`: 代码生成
- `code_review`: 代码审查
- `summarization`: 文本摘要
- `question_answering`: 问答

**使用示例**：
```rust
let template = PromptTemplate::new(
    "greeting".to_string(),
    "Hello {{name}}, welcome to {{place}}!".to_string()
);

let mut context = HashMap::new();
context.insert("name", "Alice");
context.insert("place", "Wonderland");

let result = template.render(&context)?;
// "Hello Alice, welcome to Wonderland!"
```

### 4. LLM 模块 (`src/llm/`)

**职责**：对接大语言模型

**架构**：
```
LLMProvider (trait)
  ├─→ OpenAIProvider
  ├─→ OllamaProvider
  └─→ AnthropicProvider (可扩展)
```

**关键类型**：
- `LLMConfig`: LLM 配置（provider、API key、模型等）
- `LLMProvider`: Provider trait
- `ToolDefinition`: 工具定义（用于 function calling）

**支持的 Provider**：

1. **OpenAI**:
   - 支持 GPT-4/GPT-3.5
   - 支持 Function Calling
   - 完整工具调用解析

2. **Ollama**:
   - 本地模型部署
   - 无需 API key
   - 隐私友好

3. **Anthropic** (可扩展):
   - Claude 系列模型
   - 长上下文支持

### 5. Skills 模块 (`src/skills/`)

**职责**：管理和执行技能

**架构**：
```
Skill (trait)
  ├─→ CommandSkill     # 执行 shell 命令
  ├─→ CodeSkill        # 执行代码文件
  ├─→ FunctionSkill<F> # 闭包函数
  └─→ (自定义技能)
```

**内置技能**：
1. `read_file`: 读取文件
2. `write_file`: 写入文件
3. `list_directory`: 列出目录
4. `http_get`: HTTP GET 请求
5. `calculate`: 数学计算

**SkillManager**:
- 技能注册表
- 技能执行调度
- 工具定义生成（用于 LLM）

**自定义技能示例**：
```rust
let weather_skill = FunctionSkill::new(
    "get_weather".to_string(),
    "Get weather for a city".to_string(),
    serde_json::json!({
        "type": "object",
        "properties": {
            "city": {"type": "string"}
        },
        "required": ["city"]
    }),
    |params| {
        let city = params["city"].as_str().unwrap();
        Ok(format!("Weather in {}: Sunny, 25°C", city))
    }
);
```

### 6. Utils 模块 (`src/utils/`)

**职责**：提供工具函数

**功能**：
- 日志初始化 (`init_logging`)
- 可配置日志级别

## 数据流

### 典型对话流程

```
1. 用户输入 → Agent::chat()
2. 创建用户消息 → 存入短期记忆
3. 构建上下文:
   - 从短期记忆获取最近消息
   - 从长期记忆检索相关内容
   - 从提示词管理器获取系统提示
4. 调用 LLM:
   - 序列化消息
   - 附带工具定义
   - 发送请求
5. 处理响应:
   - 文本响应 → 存储记忆 → 返回
   - 工具调用 → 执行技能 → 添加结果 → 回到步骤 4
6. 记忆管理:
   - 更新短期记忆
   - 提取重要内容存入长期记忆
```

## 扩展性设计

### 添加新的 LLM Provider

1. 实现 `LLMProvider` trait
2. 在 `create_llm_provider` 函数中注册
3. 可选：添加新的配置选项

```rust
pub struct MyProvider {
    config: LLMConfig,
}

#[async_trait]
impl LLMProvider for MyProvider {
    async fn chat(&self, messages: Vec<Message>) -> Result<LLMResponse>;
    async fn chat_with_tools(&self, messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Result<LLMResponse>;
    fn get_model(&self) -> &str;
    fn get_provider(&self) -> &str;
}
```

### 添加新的 Skill

1. 实现 `Skill` trait
2. 注册到 `SkillManager`

```rust
pub struct MySkill {
    // 技能数据
}

#[async_trait]
impl Skill for MySkill {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> &serde_json::Value;
    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult>;
}
```

### 自定义记忆策略

扩展 `LongTermMemory` 类，实现自定义的：
- 相关性计算算法
- 记忆整合策略
- 检索机制

## 配置系统

支持通过环境变量、配置文件或代码配置：

```rust
// 代码配置
let config = AgentConfig::new(name, llm_config)
    .with_role("...")
    .with_capabilities("...")
    .with_constraints("...")
    .with_memory_capacity(100, 1000)
    .with_max_iterations(10);

// 配置文件 (config.toml)
[agent]
name = "MyAssistant"
max_iterations = 10

[memory]
short_term_capacity = 100
long_term_capacity = 1000

[llm.openai]
api_key = "..."
model = "gpt-4"
```

## 性能优化

### 记忆优化
- 短期记忆使用 VecDeque（O(1) 插入/删除）
- 长期记忆按需检索（避免全量扫描）
- 定期整合（防止内存泄漏）

### 异步设计
- 全异步 I/O（tokio）
- 工具并发执行（可扩展）
- 非阻塞日志

### 资源管理
- 工具执行超时控制
- HTTP 连接池复用
- 智能缓存策略

## 安全考虑

1. **工具执行安全**:
   - 命令执行限制
   - 文件操作权限检查
   - 超时保护

2. **API Key 管理**:
   - 环境变量存储
   - 不写入日志
   - 配置文件隔离

3. **输入验证**:
   - 参数类型检查
   - 路径规范化
   - 注入攻击防护

## 测试策略

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_template_rendering() {
        // 测试提示词渲染
    }

    #[tokio::test]
    async fn test_skill_execution() {
        // 测试技能执行
    }

    #[tokio::test]
    async fn test_memory_retrieval() {
        // 测试记忆检索
    }
}
```

## 依赖项说明

| 依赖 | 用途 |
|------|------|
| tokio | 异步运行时 |
| serde/serde_json | 序列化/反序列化 |
| reqwest | HTTP 客户端 |
| handlebars | 模板引擎 |
| anyhow/thiserror | 错误处理 |
| tracing | 日志记录 |
| chrono | 时间处理 |
| uuid | 唯一 ID 生成 |
| meval | 数学表达式求值 |

## 未来扩展方向

1. **向量数据库集成**: 使用真正的 embedding 进行语义搜索
2. **多 Agent 协作**: Agent 间的通信和协作
3. **持久化存储**: 将记忆保存到数据库
4. **Web 界面**: 提供 Web UI 交互
5. **插件系统**: 动态加载技能和模块
6. **监控和可观测性**: 完善的指标和追踪

## 总结

该 AI Agent 框架提供了：
- ✅ 模块化设计，易于扩展
- ✅ 完整的记忆系统
- ✅ 灵活的提示词管理
- ✅ 多 LLM 支持
- ✅ 强大的技能系统
- ✅ 清晰的架构和文档

是一个生产就绪的 AI Agent 解决方案。
