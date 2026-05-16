# ACP Desktop 功能增强实施计划

## 项目概述

为 ACP Desktop 桌面应用实施 5 项功能增强，涵盖智能体配置优化、UI 交互改进和消息流式渲染。

**技术栈**: React 19 + TypeScript + Zustand + Tauri 2.x + Rust + SQLite

---

## 功能清单与优先级

| # | 功能 | 优先级 | 涉及文件 |
|---|------|--------|----------|
| 1 | Claude CLI 配置显式化 (launcher 选择器) | P1 | 前端 + 后端 |
| 2 | 回复气泡流式显示 (chunk 事件触发 UI 更新) | P1 | 前端 |
| 3 | 智能体创建后可编辑 | P1 | 前端 + 后端 |
| 4 | 智能体新建支持拷贝复制 | P2 | 前端 |
| 5 | 思考模型开关 (thinking_enabled) | P2 | 前端 + 后端 |

---

## 一、Claude CLI 配置显式化

### 问题
当前 `AgentConfig.claude_env_file` 仅支持手动文本输入，用户无法直观选择可用的 launcher。

### 解决方案
在 AgentForm 中添加 launcher 选择器，同时保留自定义输入。

### 修改点

**前端**
- `src/types/agent.ts`: 添加 `claudeLauncher?: string`
- `src/components/Agent/AgentForm.tsx`: 添加 `<select>` 组件
- `src/components/Agent/LLMSettings.tsx`: 显示新字段

**后端**
- `src-tauri/src/models/agent.rs`: 添加 `claude_launcher: Option<String>`
- `src-tauri/src/services/claude_cli.rs`: 修改 `resolve_launcher_command()` 优先读取 `claude_launcher`

### 数据流
```
AgentForm (select) → AgentConfig.claudeLauncher
  → createAgent → Rust → SQLite → claude_cli.rs
```

---

## 二、回复气泡流式显示

### 问题
chunk 事件通过 `setChatMessages()` 更新时，可能因 React 批量渲染导致延迟。

### 解决方案
添加 Zustand action `updateMessageChunk` 实现增量文本追加，确保每次调用都能触发重渲染。

### 修改点

**前端**
- `src/stores/chatStore.ts`: 添加 `updateMessageChunk(messageId, chunk)` action
- `src/components/Message/ChatWindow.tsx`: 修改 chunk 处理逻辑使用新 action
- `src/components/Message/ChatWindow.tsx`: 优化 Loading 状态显示

### 代码
```typescript
// chatStore.ts
updateMessageChunk: (messageId, chunk) =>
  set((state) => ({
    chatMessages: state.chatMessages.map((msg) =>
      msg.id === messageId ? { ...msg, text: msg.text + chunk } : msg
    ),
  })),
```

---

## 三、智能体创建后可编辑

### 问题
`AgentEditor.tsx` 仅显示只读信息，用户无法修改已创建的智能体。

### 解决方案
在 AgentEditor 中添加编辑模式切换，使用 AgentForm 渲染编辑表单。

### 修改点

**前端**
- `src/components/Agent/AgentEditor.tsx`: 添加 `isEditing` 状态和编辑按钮，切换编辑/查看模式
- `src/components/Agent/AgentForm.tsx`: 支持 `onSubmit` 回调用于编辑模式提交

**后端**
- `src-tauri/src/commands/agents.rs`: 确认 `update_agent` 完整支持所有字段更新

### UI 流程
```
AgentList → 点击智能体 → AgentEditor(查看模式) → 点击编辑 → AgentEditor(编辑模式)
→ 提交 → updateAgent → 返回查看模式
```

---

## 四、智能体新建支持拷贝复制

### 问题
用户创建智能体时希望能基于现有配置快速复制。

### 解决方案
在 AgentList 的每个卡片上添加复制按钮，调用 `createAgent` 时复制 config。

### 修改点

**前端**
- `src/components/Agent/AgentList.tsx`: 添加复制按钮和 `handleDuplicate` 函数

### 代码
```typescript
const handleDuplicate = async (e, agent) => {
  e.stopPropagation();
  const duplicateInput = {
    name: `${agent.name} (副本)`,
    description: agent.description,
    driverType: agent.driverType,
    config: { ...agent.config },
  };
  await createAgent(duplicateInput);
};
```

---

## 五、思考模型开关

### 问题
Claude 支持 extended thinking，用户需要控制是否启用。

### 解决方案
在 AgentConfig 中添加 `thinkingEnabled` 字段，在 CLAUDE CLI 调用时添加 `--thinking-enabled` 参数。

### 修改点

**前端**
- `src/types/agent.ts`: 添加 `thinkingEnabled?: boolean`
- `src/components/Agent/AgentForm.tsx`: 添加 checkbox 开关

**后端**
- `src-tauri/src/models/agent.rs`: 添加 `thinking_enabled: Option<bool>`
- `src-tauri/src/services/claude_cli.rs`: 在构建命令时根据此标志添加参数

---

## 文件修改清单

### 前端 (7 个文件)

| 文件 | 修改类型 | 功能 |
|------|----------|------|
| `src/types/agent.ts` | 修改 | 新增字段 |
| `src/components/Agent/AgentForm.tsx` | 修改 | launcher 选择器、thinking 开关 |
| `src/components/Agent/AgentEditor.tsx` | 重构 | 编辑模式 |
| `src/components/Agent/AgentList.tsx` | 修改 | 复制按钮 |
| `src/components/Agent/LLMSettings.tsx` | 修改 | 显示新字段 |
| `src/stores/chatStore.ts` | 修改 | chunk 更新 action |
| `src/components/Message/ChatWindow.tsx` | 修改 | 流式渲染 |

### 后端 (3 个文件)

| 文件 | 修改类型 | 功能 |
|------|----------|------|
| `src-tauri/src/models/agent.rs` | 修改 | 新增字段 |
| `src-tauri/src/services/claude_cli.rs` | 修改 | launcher 解析、thinking 参数 |
| `src-tauri/src/commands/agents.rs` | 审查 | 确认完整更新 |

---

## 实施顺序

1. **思考模型开关** - 独立功能，修改点最少
2. **Claude CLI 配置显式化** - 涉及前后端多个文件
3. **回复气泡流式显示** - 聚焦 store 和 UI 更新
4. **智能体创建后可编辑** - AgentEditor 重构
5. **智能体新建支持拷贝复制** - 简单独立功能

---

## 验收标准

- [ ] AgentForm 支持 launcher 选择器和 thinking 开关
- [ ] 流式消息实时显示，无延迟
- [ ] 可编辑已有智能体配置
- [ ] 可复制智能体配置创建副本
- [ ] thinking_enabled 正确传递到 CLI 调用
