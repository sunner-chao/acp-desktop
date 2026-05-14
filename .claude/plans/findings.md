# ACP Desktop 代码库研究结果

## 项目结构

```
acp-desktop/
├── src/                          # React 前端
│   ├── components/
│   │   ├── Agent/
│   │   │   ├── AgentForm.tsx     # 智能体创建/编辑表单
│   │   │   ├── AgentEditor.tsx   # 智能体详情查看/编辑
│   │   │   ├── AgentList.tsx     # 智能体列表
│   │   │   └── LLMSettings.tsx   # LLM 配置展示
│   │   ├── Message/
│   │   │   └── ChatWindow.tsx    # 多智能体对话窗口 (含流式处理)
│   │   └── Layout/
│   ├── stores/
│   │   ├── agentStore.ts         # 智能体状态管理
│   │   ├── chatStore.ts          # 聊天消息状态管理
│   │   ├── messageStore.ts
│   │   └── uiStore.ts
│   └── types/
│       └── agent.ts              # AgentConfig 类型定义
├── src-tauri/
│   └── src/
│       ├── models/
│       │   └── agent.rs          # Rust AgentConfig 数据模型
│       ├── commands/
│       │   └── agents.rs         # Tauri CRUD 命令
│       └── services/
│           └── claude_cli.rs     # Claude CLI 调用逻辑
```

---

## 现有类型定义

### TypeScript AgentConfig (`src/types/agent.ts`)

```typescript
export interface AgentConfig {
  apiFormat?: ApiFormat;           // 'openai' | 'anthropic'
  endpoint?: string;
  apiKey?: string;
  model?: string;
  temperature?: number;
  maxTokens?: number;
  scriptPath?: string;
  scriptType?: 'node' | 'python' | 'bash';
  claudeProjectDir?: string;       // Claude CLI 项目目录
  claudeEnvFile?: string;           // 环境文件 (.env.dsv4)
  claudeModel?: string;            // 模型名
}
```

### Rust AgentConfig (`src-tauri/src/models/agent.rs`)

```rust
pub struct AgentConfig {
    pub api_format: Option<String>,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
    pub script_path: Option<String>,
    pub script_type: Option<String>,
    pub claude_project_dir: Option<String>,  // camelCase in serde
    pub claude_env_file: Option<String>,
    pub claude_model: Option<String>,
}
```

---

## 流式消息处理现状

### ChatWindow.tsx (第 171-238 行)

当前 chunk 处理逻辑:
```typescript
if (payload.status === 'chunk' && payload.messageId && payload.chunk) {
  const current = useChatStore.getState().chatMessages.filter(...);
  // 使用 setChatMessages 函数式更新
  setChatMessages(current.map(...));
}
```

**问题**: 函数式更新依赖外部状态，可能导致批渲染延迟

### chatStore.ts

现有 action:
- `setChatMessages(messages)` - 全量替换
- `appendChatMessages(messages)` - 追加

**缺失**: 增量更新单个消息的 action

---

## AgentEditor.tsx 现状

当前实现:
- **创建模式**: 打开 `modalOpen === 'create-agent'` 时显示 AgentForm
- **查看模式**: 显示选中智能体的只读信息

**缺失**: 编辑模式，用户无法修改已创建的智能体

### AgentForm.tsx 现状

```typescript
interface AgentFormProps {
  onClose: () => void;
  initialValues?: CreateAgentInput;  // 支持编辑时传入初始值
}
```

已支持 `initialValues`，只需添加编辑提交回调即可。

---

## Claude CLI launcher 解析

### claude_cli.rs `resolve_launcher_command()` (第 499-517 行)

```rust
fn resolve_launcher_command(config: &AgentConfig, default_env_file: &str) -> String {
    // 1. 环境变量 ACP_CLAUDE_LAUNCHER
    // 2. config.claude_env_file → env_to_launcher()
    // 3. default_env_file → env_to_launcher()
}
```

**逻辑**:
- `.env.dsv4` → `claude-haha-dsv4`
- `.env.glm51` → `claude-haha-glm51`
- `.env.minimax27` → `claude-haha-minimax27`

**可用的 launcher**: dsv4, glm51, minimax27

---

## 关键发现

1. **数据一致性**: TypeScript 和 Rust 类型需同步更新
2. **向后兼容**: 新增字段使用 `Option<T>` / `?: T`
3. **流式渲染**: 需要增量更新 action 避免批量延迟
4. **AgentForm 复用性**: 已支持 initialValues，可扩展编辑模式

---

## 已验证文件清单

- [x] src/types/agent.ts
- [x] src/components/Agent/AgentForm.tsx
- [x] src/components/Agent/AgentEditor.tsx
- [x] src/components/Agent/AgentList.tsx
- [x] src/components/Agent/LLMSettings.tsx
- [x] src/stores/chatStore.ts
- [x] src/components/Message/ChatWindow.tsx
- [x] src-tauri/src/models/agent.rs
- [x] src-tauri/src/services/claude_cli.rs
- [x] src-tauri/src/commands/agents.rs
