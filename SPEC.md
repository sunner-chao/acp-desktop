# ACP Multi-Agent Communication Desktop Application

## 1. 项目概述

### 项目名称
**ACP Desktop Agent Hub** - 基于 Tauri 2.x 的跨平台多智能体通信桌面应用

### 核心功能摘要
支持创建/管理多个虚拟智能体(Agent)，通过 ACP (Agent Communication Protocol) 协议实现智能体间的消息通信，支持 Script 和 LLM 两种驱动类型。

### 目标用户
- AI 研究人员
- 多智能体系统开发者
- 需要本地运行和调试多智能体通信的团队

---

## 2. 技术栈

| 组件 | 技术 |
|------|------|
| 桌面框架 | Tauri 2.x |
| 前端框架 | React 19 + TypeScript |
| 状态管理 | Zustand |
| 后端语言 | Rust |
| 数据库 | SQLite (via rusqlite) |
| 构建工具 | Vite |
| 样式方案 | Tailwind CSS |

---

## 3. 功能规格

### 3.1 智能体管理

#### 属性
| 字段 | 类型 | 说明 |
|------|------|------|
| id | string | UUID，自动生成 |
| name | string | 唯一名称 |
| description | string | 描述 |
| driverType | enum | `script` 或 `llm` |
| address | string | 通信地址 `agent://local/{name}` |
| isOnline | boolean | 在线状态 |
| createdAt | string | ISO 8601 时间戳 |

#### LLM 配置 (driverType=llm)
| 字段 | 类型 | 说明 |
|------|------|------|
| apiFormat | enum | `openai` 或 `anthropic` |
| endpoint | string | API 端点 URL |
| apiKey | string | API 密钥 |
| model | string | 模型名称 |
| temperature | number | 采样温度 (0-2) |
| maxTokens | number | 最大 token 数 |

#### Script 配置 (driverType=script)
| 字段 | 类型 | 说明 |
|------|------|------|
| scriptPath | string | 脚本文件路径 |
| scriptType | enum | `node`, `python`, `bash` |

### 3.2 消息通信

#### ACP 消息格式 (FIPA ACL 风格)
```typescript
interface ACPMessage {
  id: string
  performative: 'request' | 'inform' | 'query' | 'agree' | 'refuse'
  sender: string      // agent://local/{name}
  receiver: string    // agent://local/{name}
  content: ACPContent
  conversation_id: string
  timestamp: string   // ISO 8601
}

interface ACPContent {
  action?: string
  parameters?: Record<string, unknown>
  result?: unknown
  reason?: string
}
```

### 3.3 消息历史
- SQLite 持久化存储
- 按 conversation_id 会话分组
- 支持按智能体/类型/时间过滤

---

## 4. 项目结构

```
acp-desktop/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── agents.rs
│       │   └── messages.rs
│       ├── models/
│       │   ├── mod.rs
│       │   ├── agent.rs
│       │   └── message.rs
│       └── services/
│           ├── mod.rs
│           ├── db.rs
│           ├── agent_manager.rs
│           ├── message_router.rs
│           └── llm_driver.rs
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   ├── hooks/
│   ├── stores/
│   ├── types/
│   └── utils/
├── package.json
├── tsconfig.json
├── vite.config.ts
├── tailwind.config.js
└── SPEC.md
```

---

## 5. 验收标准

### 智能体管理
- [ ] 可以创建新的智能体 (script/llm 驱动)
- [ ] 可以编辑已有智能体配置
- [ ] 可以删除智能体
- [ ] 可以导入/导出智能体配置 (JSON)
- [ ] 智能体列表实时显示在线状态

### 消息通信
- [ ] 可以发送 ACP 消息 (5 种 performative)
- [ ] 消息格式符合 ACP 标准
- [ ] 地址格式正确解析
- [ ] 消息发送有成功/失败反馈

### 消息历史
- [ ] 所有消息自动记录到数据库
- [ ] 可以按会话分组查看历史
- [ ] 可以按智能体/类型/时间过滤

### LLM 集成
- [ ] 支持 OpenAI Chat Completions API 格式
- [ ] 支持 Anthropic Claude Messages API 格式
- [ ] 可以配置端点、API Key、模型参数

---

## 6. 实现计划

### Phase 1: 基础框架
- 初始化 Tauri 2.x 项目
- 配置 React + TypeScript 前端
- 设置 Zustand store
- 实现 SQLite 数据库层

### Phase 2: 智能体管理
- 实现 Agent 数据模型
- 创建 AgentManager Rust 服务
- 实现 Tauri Commands (CRUD)
- 开发 AgentList 和 AgentEditor 组件

### Phase 3: 消息通信
- 实现 ACPMessage 数据模型
- 创建 MessageRouter Rust 服务
- 实现消息发送/接收 Commands
- 开发 MessagePanel 组件

### Phase 4: 消息历史
- 实现消息持久化
- 创建消息历史查询 API
- 开发 MessageHistory 组件

### Phase 5: LLM 驱动
- 实现 LLMDriver 服务
- 适配 OpenAI API 格式
- 适配 Anthropic API 格式