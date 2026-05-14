import { useRef, useEffect, useMemo } from 'react';
import { useAgentStore, useChatStore, useMessageStore, useUIStore } from '../../stores';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { ACPMessage } from '../../types';
import type { ChatMessage } from '../../types/chat';

interface GroupChatResult {
  conversationId: string;
  messages: ACPMessage[];
}

interface GroupChatStreamEvent {
  requestId: string;
  conversationId: string;
  messageId?: string | null;
  agentId?: string | null;
  speaker?: string | null;
  round?: number | null;
  chunk?: string | null;
  message?: ACPMessage | null;
  status: 'start' | 'chunk' | 'message' | 'done';
}

const userAddress = 'agent://local/user';

function getMessageText(message: ACPMessage): string {
  const paramsText = message.content.parameters?.text;
  const result = message.content.result;
  const resultText =
    result && typeof result === 'object' && 'text' in result
      ? (result as { text?: unknown }).text
      : undefined;

  if (typeof paramsText === 'string') return paramsText;
  if (typeof resultText === 'string') return resultText;
  if (typeof message.content.reason === 'string') return message.content.reason;
  return JSON.stringify(message.content.result ?? message.content.parameters ?? {});
}

function getSpeakerName(message: ACPMessage): string {
  const speaker = message.metadata?.speaker;
  if (typeof speaker === 'string') return speaker;
  return message.sender.replace('agent://local/', '');
}

function toChatMessage(message: ACPMessage): ChatMessage {
  const speaker = getSpeakerName(message);
  return {
    id: message.id,
    role: message.sender === userAddress ? 'user' : 'agent',
    sender: speaker === 'user' ? 'You' : speaker,
    text: getMessageText(message),
    timestamp: message.timestamp,
    round: typeof message.metadata?.round === 'number' ? message.metadata.round : undefined,
    acpMessage: message,
  };
}

export default function ChatWindow() {
  const { agents, fetchAgents, startAgentSession } = useAgentStore();
  const { messages, fetchMessages } = useMessageStore();
  const {
    chatMessages,
    selectedAgentIds,
    inputText,
    rounds,
    conversationId,
    isSending,
    isStopping,
    autoContinue,
    resetTransientState,
    setChatMessages,
    appendChatMessages,
    setSelectedAgentIds,
    toggleSelectedAgent,
    setInputText,
    setRounds,
    setConversationId,
    setIsSending,
    setIsStopping,
    setAutoContinue,
  } = useChatStore();
  const { agentDrawerCollapsed, toggleAgentDrawer } = useUIStore();
  const chatEndRef = useRef<HTMLDivElement>(null);
  const stopRequestedRef = useRef(false);
  const activeRequestIdRef = useRef<string | null>(null);

  const ccAgents = useMemo(
    () => agents.filter((agent) => agent.config.apiFormat === 'anthropic'),
    [agents]
  );
  const selectedAgents = ccAgents.filter((agent) => selectedAgentIds.includes(agent.id));
  const canSend = selectedAgents.length > 0 && inputText.trim().length > 0 && !isSending;

  useEffect(() => {
    fetchAgents();
    fetchMessages();
  }, [fetchAgents, fetchMessages]);

  useEffect(() => {
    if (selectedAgentIds.length === 0 && ccAgents.length > 0) {
      setSelectedAgentIds(ccAgents.slice(0, 3).map((agent) => agent.id));
    }
  }, [ccAgents, selectedAgentIds.length]);

  useEffect(() => {
    if (conversationId) {
      const history = messages
        .filter((message) => message.conversationId === conversationId)
        .map(toChatMessage);
      if (history.length > 0) {
        setChatMessages(history);
      }
    }
  }, [conversationId, messages, setChatMessages]);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [chatMessages]);

  const toggleAgent = (agentId: string) => {
    toggleSelectedAgent(agentId);
  };

  const handleResetChat = () => {
    resetTransientState();
    setChatMessages([]);
  };

  const handleSend = async () => {
    if (!canSend) return;

    const prompt = inputText.trim();
    setInputText('');
    stopRequestedRef.current = false;
    setIsSending(true);
    setIsStopping(false);

    const loadingId = crypto.randomUUID();
    appendChatMessages([
      {
        id: loadingId,
        role: 'system',
        sender: 'ACP',
        text: `${selectedAgents.length} 个智能体正在接入本轮对话...`,
        timestamp: new Date().toISOString(),
        isLoading: true,
      },
    ]);

    try {
      for (const agent of selectedAgents) {
        if (!agent.isOnline) {
          await startAgentSession(agent.id);
        }
      }

      await fetchAgents();
      const readyAgents = useAgentStore
        .getState()
        .agents.filter((agent) => selectedAgentIds.includes(agent.id));

      let activeConversationId = conversationId;
      let nextPrompt = prompt;

      while (!stopRequestedRef.current) {
        const requestId = crypto.randomUUID();
        activeRequestIdRef.current = requestId;

        const unlisten = await listen<GroupChatStreamEvent>('group-chat-stream', (event) => {
          const payload = event.payload;
          if (payload.requestId !== requestId) return;

          if (payload.conversationId) {
            setConversationId(payload.conversationId);
          }

          if (payload.status === 'start' && payload.messageId && payload.speaker) {
            setChatMessages([
              ...useChatStore.getState().chatMessages.filter((message) => message.id !== loadingId),
              {
                id: payload.messageId,
                role: 'agent',
                sender: payload.speaker,
                text: '',
                timestamp: new Date().toISOString(),
                round: payload.round ?? undefined,
                isLoading: true,
              },
            ]);
          }

          if (payload.status === 'chunk' && payload.messageId && payload.chunk) {
            const current = useChatStore.getState().chatMessages.filter((message) => message.id !== loadingId);
            const existing = current.find((message) => message.id === payload.messageId);
            if (existing) {
              setChatMessages(
                current.map((message) =>
                  message.id === payload.messageId
                    ? {
                        ...message,
                        text: `${message.text}${payload.chunk}`,
                        isLoading: true,
                      }
                    : message
                )
              );
            } else {
              setChatMessages([
                ...current,
                {
                  id: payload.messageId,
                  role: 'agent',
                  sender: payload.speaker ?? 'Agent',
                  text: payload.chunk,
                  timestamp: new Date().toISOString(),
                  round: payload.round ?? undefined,
                  isLoading: true,
                },
              ]);
            }
          }

          if (payload.status === 'message' && payload.message) {
            const chatMessage = toChatMessage(payload.message);
            const current = useChatStore.getState().chatMessages.filter((message) => message.id !== loadingId);
            if (current.some((message) => message.id === chatMessage.id)) {
              setChatMessages(
                current.map((message) =>
                  message.id === chatMessage.id ? { ...chatMessage, isLoading: false } : message
                )
              );
            } else {
              setChatMessages([...current, chatMessage]);
            }
          }
        });

        let result: GroupChatResult | null = null;
        try {
          result = await invoke<GroupChatResult>('invoke_agent_group_chat_stream', {
            input: {
              agents: readyAgents.map((agent) => ({
                id: agent.id,
                name: agent.name,
                config: agent.config,
                description: agent.description ?? null,
                address: agent.address,
              })),
              message: nextPrompt,
              rounds,
              conversationId: activeConversationId,
              requestId,
            },
          });
        } finally {
          unlisten();
          if (activeRequestIdRef.current === requestId) {
            activeRequestIdRef.current = null;
          }
        }

        if (stopRequestedRef.current) {
          setChatMessages(
            useChatStore.getState().chatMessages.map((message) =>
              message.isLoading
                ? {
                    ...message,
                    isLoading: false,
                  }
                : message
            )
          );
          break;
        }

        if (!result) {
          break;
        }

        activeConversationId = result.conversationId;
        setConversationId(result.conversationId);
        setChatMessages([
          ...useChatStore.getState().chatMessages.filter((message) => message.id !== loadingId),
          ...result.messages.map(toChatMessage),
        ]);
        await fetchMessages();
        await fetchAgents();

        if (!autoContinue) {
          break;
        }

        nextPrompt = '继续这段多智能体对话，基于上一轮内容自然推进，不要结束。';
        await new Promise((resolve) => setTimeout(resolve, 600));
      }
    } catch (error) {
      setChatMessages(
        useChatStore.getState().chatMessages.map((message) =>
          message.id === loadingId
            ? {
                ...message,
                text: `[错误] ${String(error)}`,
                isLoading: false,
              }
            : message
        )
      );
    } finally {
      setIsSending(false);
      setIsStopping(false);
      stopRequestedRef.current = false;
      activeRequestIdRef.current = null;
    }
  };

  const handleStop = async () => {
    stopRequestedRef.current = true;
    setIsStopping(true);
    const requestId = activeRequestIdRef.current;
    if (!requestId) return;
    try {
      await invoke('stop_agent_group_chat', { requestId });
    } catch (error) {
      console.warn('停止当前对话失败', error);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="flex h-full gap-4">
      <div
        className={`flex-shrink-0 bg-gray-800 rounded-lg border border-gray-700 flex flex-col transition-all ${
          agentDrawerCollapsed ? 'w-14' : 'w-72'
        }`}
      >
        <div className="p-3 border-b border-gray-700 flex items-center justify-between gap-2">
          {!agentDrawerCollapsed ? (
            <div>
              <h3 className="font-semibold text-sm">参与智能体</h3>
              <div className="mt-1 text-xs text-gray-500">{selectedAgents.length} 个已选择</div>
            </div>
          ) : (
            <div className="text-xs text-gray-500">A</div>
          )}
          <button
            onClick={toggleAgentDrawer}
            className="h-7 w-7 rounded-md hover:bg-gray-700 text-gray-400 hover:text-gray-200"
            title={agentDrawerCollapsed ? '展开' : '折叠'}
          >
            {agentDrawerCollapsed ? '›' : '‹'}
          </button>
        </div>
        <div className="flex-1 overflow-auto p-2 space-y-1">
          {agentDrawerCollapsed ? (
            <div className="text-center text-xs text-gray-500 py-3">{selectedAgents.length}</div>
          ) : (
            <>
              {ccAgents.map((agent) => {
                const checked = selectedAgentIds.includes(agent.id);
                return (
                  <button
                    key={agent.id}
                    onClick={() => toggleAgent(agent.id)}
                    className={`w-full text-left px-3 py-2 rounded-lg text-sm transition-colors ${
                      checked ? 'bg-primary-600 text-white' : 'hover:bg-gray-700 text-gray-300'
                    }`}
                  >
                    <div className="flex items-center gap-2">
                      <span
                        className={`h-4 w-4 rounded border flex items-center justify-center text-[10px] ${
                          checked ? 'border-white bg-white text-primary-700' : 'border-gray-500'
                        }`}
                      >
                        {checked ? '✓' : ''}
                      </span>
                      <div className={`w-2 h-2 rounded-full ${agent.isOnline ? 'bg-green-400' : 'bg-gray-500'}`} />
                      <span className="truncate">{agent.name}</span>
                    </div>
                    {agent.description && (
                      <div className="mt-1 pl-8 text-xs opacity-70 line-clamp-2">{agent.description}</div>
                    )}
                  </button>
                );
              })}
              {ccAgents.length === 0 && (
                <div className="px-3 py-6 text-sm text-gray-500">暂无 Anthropic/Claude 智能体</div>
              )}
            </>
          )}
        </div>
      </div>

      <div className="flex-1 bg-gray-800 rounded-lg border border-gray-700 flex flex-col min-w-0">
        <div className="p-3 border-b border-gray-700 flex items-center justify-between gap-3">
          <div className="min-w-0">
            <div className="font-semibold">多智能体对话</div>
            <div className="text-xs text-gray-500 truncate">
              {conversationId ? `会话 ${conversationId}` : '你作为引导者启动对话，智能体按选择顺序发言'}
            </div>
          </div>
          <div className="flex items-center gap-2 text-sm">
            <button
              onClick={handleResetChat}
              className="h-8 px-3 rounded-md bg-gray-700 hover:bg-gray-600 text-gray-200 text-xs"
            >
              清空当前
            </button>
            <span className="text-gray-400">回合</span>
            <input
              type="number"
              min={1}
              max={6}
              value={rounds}
              onChange={(e) => setRounds(Math.min(6, Math.max(1, Number(e.target.value) || 1)))}
              className="w-16 bg-gray-900 border border-gray-700 rounded-lg px-2 py-1 text-gray-100 focus:border-primary-500 focus:outline-none"
            />
            <label className="flex items-center gap-2 ml-2 text-xs text-gray-300">
              <input
                type="checkbox"
                checked={autoContinue}
                onChange={(e) => setAutoContinue(e.target.checked)}
              />
              自动续聊
            </label>
          </div>
        </div>

        <div className="flex-1 overflow-auto p-4 space-y-4">
          {chatMessages.length === 0 && (
            <div className="flex items-center justify-center h-full text-gray-500">
              <div className="text-center">
                <div className="text-4xl mb-4">💬</div>
                <div>选择多个智能体，然后输入一段引导语</div>
                <div className="text-sm mt-2">每一轮会按左侧顺序依次发言</div>
              </div>
            </div>
          )}

          {chatMessages.map((msg) => (
            <div
              key={msg.id}
              className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}
            >
              <div
                className={`max-w-[78%] rounded-lg p-3 ${
                  msg.role === 'user'
                    ? 'bg-primary-600 text-white'
                    : msg.role === 'system'
                      ? 'bg-gray-900 border border-gray-700 text-gray-300'
                      : 'bg-gray-700 text-gray-100'
                }`}
              >
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-xs font-semibold opacity-75">{msg.sender}</span>
                  {msg.round !== undefined && msg.round > 0 && (
                    <span className="text-xs opacity-50">R{msg.round}</span>
                  )}
                  <span className="text-xs opacity-50">{new Date(msg.timestamp).toLocaleTimeString()}</span>
                </div>
                {msg.isLoading ? (
                  <div className="flex items-center gap-2 text-sm opacity-75">
                    <span className="animate-pulse">●</span>
                    {msg.text}
                  </div>
                ) : (
                  <div className="text-sm whitespace-pre-wrap">{msg.text}</div>
                )}
              </div>
            </div>
          ))}
          <div ref={chatEndRef} />
        </div>

        <div className="p-3 border-t border-gray-700">
          <div className="flex gap-2">
            <textarea
              value={inputText}
              onChange={(e) => setInputText(e.target.value)}
              onKeyDown={handleKeyDown}
              disabled={isSending}
              placeholder={
                selectedAgents.length > 0
                  ? `引导 ${selectedAgents.length} 个智能体开始讨论...`
                  : '请先选择至少一个智能体'
              }
              className="flex-1 bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none resize-none text-sm"
              rows={2}
            />
            <button
              onClick={isSending ? handleStop : handleSend}
              disabled={isStopping || (!isSending && !canSend)}
              className={`px-4 py-2 rounded-lg transition-colors text-sm self-end ${
                isSending
                  ? 'bg-red-600 hover:bg-red-700 text-white'
                  : 'bg-primary-600 hover:bg-primary-700 disabled:bg-gray-600 disabled:cursor-not-allowed'
              }`}
            >
              {isStopping ? '停止中...' : isSending ? '停止' : '启动'}
            </button>
          </div>
          <div className="mt-2 text-xs text-gray-500">
            CLI 配置来自 agent 配置或环境变量 ACP_CLAUDE_PROJECT_DIR / ACP_CLAUDE_ENV_FILE / ACP_CLAUDE_COMMAND
          </div>
        </div>
      </div>
    </div>
  );
}
