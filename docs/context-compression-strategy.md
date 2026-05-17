# ACP Desktop 上下文压缩策略设计

## 一、问题背景

### 1.1 现象

在 ACP Desktop 多 Agent 会话中，部分 Agent（`前端开发工程师-GLM2号`、`代码审查测试专家`）在运行约 17 轮之后开始出现以下错误：

```
API Error: 400 {"error":{"code":"400","message":"Param Incorrect","param":"Not supported model glm-5.1"}}
API Error: 400 {"error":{"code":"400","message":"Param Incorrect","param":"Not supported model MiniMax-M2.7-highspeed"}}
```

**关键发现**：
- `model: "<synthetic>"` 表明该错误是 Claude Code 本地生成的消息（而非真实 API 响应），但错误内容来自实际 HTTP 400 响应
- 错误格式与千帆/MiniMax 代理的格式一致（`code`/`param` 字段）
- 前 17 轮工作正常（每轮 ~2 分钟），从第 18 轮开始每轮在 1-2 秒内失败
- 第一次错误出现在上下文大小从 ≤10K chars 跳到 14,721 chars 时
- 相同会话中其他 Agent（如 `后端开发工程师-小米员工002` 使用 `mimo-v2.5-pro`）持续正常工作

### 1.2 根本原因分析

**直接原因**：上下文累积到一定规模后，千帆/ MiniMax 代理拒绝 GLM-5.1 和 MiniMax-M2.7-highspeed 的请求组合（可能是模型 + thinking 参数 + context 大小超出了代理的限制）。

**根因**：代理的有效上下文窗口比预期更小，且 Claude Code 的 autoCompact 阈值（`effectiveWindow - 13,000`）对于 3P 代理模型可能设置过高，导致压缩触发太晚，第一次 API 调用就已经超限。

### 1.3 Claude Code 现有压缩架构

Claude Code 已有完整的自动压缩系统：

| 组件 | 文件 | 触发条件 |
|---|---|---|
| `shouldAutoCompact()` | `src/services/compact/autoCompact.ts` | `tokenCountWithEstimation(messages) - snipTokensFreed > autoCompactThreshold` |
| `autoCompactIfNeeded()` | `src/services/compact/autoCompact.ts` | 阈值触发后执行压缩 |
| `compactConversation()` | `src/services/compact/compact.ts` | 分叉同模型生成摘要 |
| `sessionMemoryCompact()` | `src/services/compact/sessionMemoryCompact.ts` | 实验性，基于 session memory 的轻量压缩 |

**关键参数**：
- `AUTOCOMPACT_BUFFER_TOKENS = 13,000` — 触发压缩时预留的 token buffer
- `getEffectiveContextWindowSize(model)` — `模型上下文窗口 - maxOutputTokens - AUTO_COMPACT_WINDOW`
- `CLAUDE_CODE_AUTO_COMPACT_WINDOW` — 环境变量覆盖窗口大小
- `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` — 用百分比覆盖阈值（如设为 70 表示 70% 窗口触发）

**问题**：3P 代理（千帆、MiniMax）的实际有效上下文窗口未知且可能较小，13K buffer 不足以防止超限。

---

## 二、方案概述

在 ACP Desktop 层实现**主动上下文监控 + MiniMax 摘要压缩**，作为 Claude Code 现有 autoCompact 的前置防护层。

### 2.1 设计目标

1. **实时监控**：每次 Agent Turn 发起前，计算当前上下文 Token 数
2. **柔性阈值**：超过阈值（如 80% 窗口）时，使用 MiniMax-M2.7-highspeed 主动摘要压缩历史
3. **硬性阻断**：超过硬性阈值（如 90% 窗口）时，拒绝发起 Turn，提示用户
4. **不修改 Claude Code 核心**：所有逻辑在 ACP Desktop Rust/TypeScript 层实现
5. **与 CC autoCompact 并存**：ACP 先做一层防护，Claude Code 再做一层，双重保障

### 2.2 整体流程

```
ACP Desktop 发起 Agent Turn
    │
    ▼
1. 读取 session .jsonl 文件，累计 token 数
    │
    ├── token <= SOFT_THRESHOLD
    │   └── 直接发起 Turn（正常流程）
    │
    ├── SOFT_THRESHOLD < token <= HARD_THRESHOLD
    │   ├── 调用 MiniMax API 摘要历史
    │   ├── 写入 compact boundary + 摘要 到 session
    │   └── 继续发起 Turn
    │
    └── token > HARD_THRESHOLD
        └── 中止 Turn，返回错误信息让用户手动压缩
```

---

## 三、详细设计

### 3.1 Token 计数模块

**实现位置**：`src/utils/tokenCounter.ts`

**方案**：使用 `tiktoken` 库在 ACP Desktop Node 层计数，不依赖模型 API，无额外成本。

```typescript
import tiktoken from 'tiktoken'

const MODEL_ENCODING_MAP: Record<string, string> = {
  'glm-5.1': 'cl100k_base',
  'glm-4': 'cl100k_base',
  'MiniMax-M2.7-highspeed': 'cl100k_base',
  'MiniMax-M2.7': 'cl100k_base',
  'mimo-v2.5-pro': 'cl100k_base',
}

const SYSTEM_PROMPT_TOKENS = 350  // 估算的系统前缀 token 数

/**
 * 计算 session 文件的累计 token 数
 * @param sessionPath .jsonl 文件路径
 * @param model 当前使用的模型名
 * @returns 估算的总 token 数
 */
export async function countSessionTokens(
  sessionPath: string,
  model: string,
): Promise<number> {
  const encName = MODEL_ENCODING_MAP[model] ?? 'cl100k_base'
  const enc = tiktoken.for_model(encName)

  const lines = await readLines(sessionPath)
  let total = SYSTEM_PROMPT_TOKENS

  for (const line of lines) {
    const msg = JSON.parse(line) as TranscriptMessage
    if (msg.type === 'user' || msg.type === 'assistant') {
      const text = extractTextContent(msg)
      if (text) {
        total += enc.encode(text).length
      }
    }
  }

  enc.free()
  return total
}

/**
 * 从 TranscriptMessage 中提取纯文本内容
 */
export function extractTextContent(msg: TranscriptMessage): string {
  const content = msg.message?.content ?? msg.content ?? []
  if (typeof content === 'string') return content
  if (!Array.isArray(content)) return ''

  return content
    .filter(b => b.type === 'text')
    .map(b => (b as TextBlock).text)
    .join('\n')
}
```

### 3.2 阈值配置

**实现位置**：`src/services/contextManager.ts`

```typescript
export interface ContextThreshold {
  /** 软性阈值（Token），超过时触发 MiniMax 压缩 */
  soft: number
  /** 硬性阈值（Token），超过时拒绝发起 Turn */
  hard: number
  /** 模型上下文窗口上限 */
  window: number
}

/**
 * 各模型/代理的阈值配置
 * 这些值需要根据实际测试调整
 */
export const CONTEXT_THRESHOLDS: Record<string, ContextThreshold> = {
  // 已知问题的模型 — 使用保守阈值
  'glm-5.1': { soft: 24000, hard: 28000, window: 32000 },
  'MiniMax-M2.7-highspeed': { soft: 20000, hard: 24000, window: 32000 },

  // 标准 Anthropic 模型
  'claude-opus-4-6': { soft: 140000, hard: 160000, window: 200000 },
  'claude-sonnet-4-6': { soft: 140000, hard: 160000, window: 200000 },
  'claude-haiku-4-5': { soft: 140000, hard: 160000, window: 200000 },

  // 其他 3P 模型
  'MiniMax-M2.7': { soft: 140000, hard: 160000, window: 200000 },
  'glm-4': { soft: 80000, hard: 100000, window: 128000 },
  'mimo-v2.5-pro': { soft: 80000, hard: 100000, window: 128000 },
}

export function getThresholds(model: string): ContextThreshold {
  return (
    CONTEXT_THRESHOLDS[model] ?? {
      soft: 140000,
      hard: 160000,
      window: 200000,
    }
  )
}
```

### 3.3 MiniMax 摘要逻辑

**实现位置**：`src/services/thirdPartySummarizer.ts`

**设计考量**：使用独立的 MiniMax-M2.7-highspeed API 做摘要，不消耗主模型的 token budget，且 MiniMax 在 ACP Desktop 中已配置可用。

```typescript
import { getAnthropicClient } from './api/client.js'
import { countSessionTokens } from '../utils/tokenCounter.js'

const SUMMARIZER_MODEL = 'MiniMax-M2.7-highspeed'

const SUMMARY_SYSTEM_PROMPT = `你是一个对话历史摘要助手。请简洁地总结以下对话历史，保留：
1. 用户的主要需求和目标
2. 已完成的工作和关键决策
3. 未解决的问题和待办事项
4. 重要的代码变更或文件路径
5. 当前会话的状态和进度
保持摘要简洁，控制在 800 tokens 以内。摘要使用中文。`

/**
 * 使用 MiniMax 摘要一组消息
 */
export async function summarizeWithMiniMax(
  messages: TranscriptMessage[],
  customInstructions?: string,
): Promise<string> {
  const conversation = formatMessagesForSummary(messages)
  const prompt = `${SUMMARY_SYSTEM_PROMPT}\n\n=== 对话历史 ===\n${conversation}`

  const anthropic = await getAnthropicClient({
    maxRetries: 2,
    model: SUMMARIZER_MODEL,
    source: 'compact_summarizer',
  })

  const response = await anthropic.beta.messages.create({
    model: SUMMARIZER_MODEL,
    max_tokens: 1024,
    messages: [{ role: 'user', content: prompt }],
  })

  if (response.content[0].type === 'text') {
    return response.content[0].text
  }
  return ''
}

/**
 * 将消息数组格式化为摘要输入
 */
function formatMessagesForSummary(messages: TranscriptMessage[]): string {
  return messages
    .filter(m => m.type === 'user' || m.type === 'assistant')
    .map(m => {
      const role = m.type === 'user' ? '【用户】' : '【助手】'
      const text = extractTextContent(m)
      const truncated = text.length > 2000 ? text.slice(0, 2000) + '...' : text
      return `${role}\n${truncated}`
    })
    .join('\n\n')
}

export interface CompactResult {
  summary: string
  compactAt: number      // 在第几条消息处压缩
  tokenCount: number     // 压缩前的 token 数
  preservedCount: number // 保留的尾部消息数
}

/**
 * 对 session 文件执行 MiniMax 压缩
 * @param sessionPath .jsonl 文件路径
 * @param keepLastN 保留最近 N 条消息（完整不压缩）
 * @returns 压缩结果
 */
export async function compactSessionWithMiniMax(
  sessionPath: string,
  keepLastN: number = 10,
): Promise<CompactResult> {
  const allMessages = await readSessionFile(sessionPath)

  // 分离：压缩头部 + 保留尾部
  const toCompact = allMessages.slice(0, -keepLastN)
  const toKeep = allMessages.slice(-keepLastN)

  if (toCompact.length === 0) {
    return { summary: '', compactAt: 0, tokenCount: 0, preservedCount: 0 }
  }

  const summary = await summarizeWithMiniMax(toCompact)

  // 构建 compact boundary 消息（兼容 Claude Code session 格式）
  const compactMarker: TranscriptMessage = {
    type: 'system',
    subtype: 'compact_boundary',
    uuid: generateUUID(),
    timestamp: new Date().toISOString(),
    message: {
      role: 'system',
      content: `[Earlier conversation summarized]\n\n${summary}`,
    },
    isCompactBoundary: true,
  }

  // 写入压缩后的 session
  await writeCompactSession(sessionPath, [compactMarker, ...toKeep])

  const tokenCount = await countSessionTokens(sessionPath, SUMMARIZER_MODEL)

  return {
    summary,
    compactAt: toCompact.length,
    tokenCount,
    preservedCount: keepLastN,
  }
}
```

### 3.4 Rust 层拦截逻辑

**实现位置**：`src-tauri/src/services/context_manager.rs`

在 Agent Turn 发起前拦截，检查并触发压缩：

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CompactResult {
    pub summary: String,
    pub compact_at: usize,
    pub token_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContextThreshold {
    pub soft: usize,
    pub hard: usize,
    pub window: usize,
}

// 固定阈值配置（可扩展为从配置文件读取）
const CONTEXT_THRESHOLDS: phf::Map<&'static str, ContextThreshold> = phf::map! {
    "glm-5.1" => ContextThreshold { soft: 24000, hard: 28000, window: 32000 },
    "MiniMax-M2.7-highspeed" => ContextThreshold { soft: 20000, hard: 24000, window: 32000 },
    // ... 其他模型
};

fn get_threshold(model: &str) -> ContextThreshold {
    CONTEXT_THRESHOLDS
        .get(model)
        .copied()
        .unwrap_or(ContextThreshold {
            soft: 140000,
            hard: 160000,
            window: 200000,
        })
}

/// 在 Agent Turn 发起前调用，返回是否需要压缩以及压缩摘要
pub async fn check_and_compact_if_needed(
    session_path: &Path,
    model: &str,
) -> Result<Option<CompactResult>, AgentError> {
    let threshold = get_threshold(model);

    // 调用 Node 层计数的占位（实际实现中通过 command 调用 TS 代码）
    let token_count = count_session_tokens_sync(session_path).await?;

    if token_count > threshold.hard {
        return Err(AgentError::ContextTooLarge {
            current: token_count,
            limit: threshold.hard,
            message: format!(
                "上下文已达 {} tokens（硬性限制 {}），请先使用 /compact 压缩后再继续",
                token_count, threshold.hard
            ),
        });
    }

    if token_count > threshold.soft {
        // 调用 MiniMax 压缩
        let result = compact_session_with_minimax(session_path, 10).await?;
        log::info!(
            "上下文压缩完成: {} tokens -> {} tokens, 摘要: {}",
            result.token_count,
            result.token_count, // 压缩后 token 数需重新计算
            result.summary.chars().take(100).collect::<String>()
        );
        return Ok(Some(result));
    }

    Ok(None)
}
```

### 3.5 Session 文件读写

**实现位置**：`src/services/sessionCompactor.ts`

```typescript
import { readFile, writeFile, appendFile } from 'node:fs/promises'
import type { TranscriptMessage } from '../types/message.js'

/**
 * 读取 session .jsonl 文件
 */
export async function readSessionFile(
  sessionPath: string,
): Promise<TranscriptMessage[]> {
  const content = await readFile(sessionPath, 'utf-8')
  return content
    .split('\n')
    .filter(line => line.trim())
    .map(line => JSON.parse(line) as TranscriptMessage)
}

/**
 * 写入压缩后的 session 文件
 * 保留 compact boundary + 尾部保留消息
 */
export async function writeCompactSession(
  sessionPath: string,
  messages: TranscriptMessage[],
): Promise<void> {
  const lines = messages.map(m => JSON.stringify(m)).join('\n') + '\n'
  await writeFile(sessionPath, lines, 'utf-8')
}

/**
 * 在 session 文件末尾追加一条消息（不重写整个文件）
 */
export async function appendSessionMessage(
  sessionPath: string,
  message: TranscriptMessage,
): Promise<void> {
  const line = JSON.stringify(message) + '\n'
  await appendFile(sessionPath, line, 'utf-8')
}
```

### 3.6 与 Claude Code 的协同

```
Timeline:

Turn N+1 发起
    │
    ├── ACP Desktop 层检查 (check_and_compact_if_needed)
    │   └── token_count = 25,000 > SOFT(24,000)
    │       └── MiniMax 压缩 → session 变为 8,000 tokens
    │
    Claude Code 内部检查 (shouldAutoCompact)
    │   └── token_count = 8,000 < threshold(18,000)
    │       └── 不触发压缩，直接发起 API 调用
    │
    API 调用成功
    │
Turn N+2 发起
    │
    ├── ACP Desktop 层检查
    │   └── token_count = 27,000 > SOFT(24,000)
    │       └── MiniMax 再次压缩
    │
    Claude Code 内部检查
    │   └── 可能触发也可能不触发，看阈值
```

**双层防护的优势**：
1. ACP Desktop 层使用更保守的阈值，尽早触发 MiniMax 压缩
2. Claude Code 层的压缩使用同模型摘要，语义一致性更好
3. 两者互补：ACP 层防止代理超限，CC 层提供语义级别的会话压缩

---

## 四、文件清单

| 文件路径 | 作用 |
|---|---|
| `src/utils/tokenCounter.ts` | tiktoken Token 计数实现 |
| `src/services/thirdPartySummarizer.ts` | MiniMax 摘要 API 调用 |
| `src/services/sessionCompactor.ts` | session 文件读写、compact marker 写入 |
| `src/services/contextManager.ts` | 阈值配置、检查入口 |
| `src-tauri/src/services/context_manager.rs` | Rust 层拦截逻辑、Agent Turn 调度增强 |

---

## 五、配置项

### 5.1 环境变量

```bash
# 是否启用 ACP 层上下文压缩（默认 true）
ACP_CONTEXT_COMPACT_ENABLED=true

# 是否使用 MiniMax 摘要（默认 true，需要 MiniMax API 已配置）
ACP_USE_MINIMAX_SUMMARIZER=true

# 保留尾部消息数（默认 10）
ACP_COMPACT_KEEP_LAST_N=10

# 是否在日志中输出 token 计数（默认 false）
ACP_LOG_TOKEN_COUNT=false
```

### 5.2 每个 Agent 的阈值覆盖

在 `agents.json` 中扩展配置：

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440001",
  "name": "前端开发工程师-GLM2号",
  "config": {
    "contextThresholds": {
      "soft": 24000,
      "hard": 28000,
      "window": 32000
    }
  }
}
```

---

## 六、快速替代方案（不改代码）

在 `.env.glm51` 中添加 Claude Code 环境变量，降低千帆代理上 GLM-5.1 的压缩阈值：

```bash
# 强制 GLM-5.1 在 70% 窗口时触发压缩（而非默认的 ~93%）
CLAUDE_AUTOCOMPACT_PCT_OVERRIDE=70

# 或直接设置窗口大小
CLAUDE_CODE_AUTO_COMPACT_WINDOW=28000

# 禁用 thinking（如果 thinking 参数是触发 400 的原因）
CLAUDE_CODE_DISABLE_THINKING=true
```

---

## 七、待验证事项

1. **GLM-5.1 在千帆上的实际上下文窗口**：通过逐步增大 prompt 测试 400 错误出现的精确临界点
2. **MiniMax-M2.7-highspeed 是否有相同问题**：测试上下文累积到多大时出现错误
3. **thinking 参数是否是触发因素**：对比开启/关闭 thinking 时的错误触发阈值
4. **MiniMax 摘要的 token 消耗**：估算每次压缩的 MiniMax API 成本
5. **Compact boundary 格式兼容性**：确保 ACP 层写入的 compact marker 与 Claude Code 自身的 compact boundary 格式兼容，避免 session 恢复时出错