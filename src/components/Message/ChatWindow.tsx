import { useRef, useEffect, useMemo } from 'react';
import { useAgentStore, useChatStore, useConversationStore, useMessageStore, useUIStore } from '../../stores';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { ACPMessage } from '../../types';
import type { ChatMessage, ToolCallInfo } from '../../types/chat';
import ConversationSidebar from './ConversationSidebar';

const INVOKE_TIMEOUT_MS = 620_000; // Slightly longer than backend CLI_TIMEOUT_SECS (600s)

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
  status: 'start' | 'chunk' | 'message' | 'tool_use' | 'turn_complete' | 'round_complete' | 'done' | 'cancelled' | 'tool_use_complete';
  tool_calls?: ToolCallInfo[] | null;
  phase?: string | null;
}

const userAddress = 'agent://local/user';

/** Render message text with thinking content styled differently (collapsible) */
function renderMessageWithThinking(text: string) {
  const thinkTagRegex = /<thought[^>]*>([\s\S]*?)<\/thought>/gi;
  const parts = text.split(thinkTagRegex);

  if (parts.length === 1) {
    // No thinking tags found, render as normal
    return (
      <div className="text-sm whitespace-pre-wrap">{text}</div>
    );
  }

  // Has thinking content - render with styled thinking blocks
  return (
    <div className="text-sm whitespace-pre-wrap">
      {parts.map((part, idx) => {
        if (idx % 2 === 1) {
          // This is thinking content (odd indices after split)
          return (
            <details key={idx} className="mt-1 mb-2 rounded bg-gray-800 border border-gray-600">
              <summary className="cursor-pointer px-2 py-1 text-xs text-gray-400 hover:text-gray-300 flex items-center gap-1">
                <span className="opacity-50">🤔</span>
                <span>Thinking...</span>
              </summary>
              <div className="px-3 py-2 text-xs text-gray-400 italic border-t border-gray-700">
                {part.trim()}
              </div>
            </details>
          );
        }
        // Regular text content
        return part ? <span key={idx}>{part}</span> : null;
      })}
    </div>
  );
}

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
  const { fetchMessages } = useMessageStore();
  const {
    chatMessages,
    selectedAgentIds,
    inputText,
    rounds,
    conversationId,
    isSending,
    isStopping,
    autoContinue,
    autoContinueDelay,
    chatMode,
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
    setAutoContinueDelay,
    setChatMode,
    switchConversation,
    removeConversationFromCache,
  } = useChatStore();
  const { conversations: convList, createConversation, deleteConversation, updateConversation } = useConversationStore();
  const { agentDrawerCollapsed, toggleAgentDrawer, conversationSidebarCollapsed, toggleConversationSidebar } = useUIStore();
  const chatEndRef = useRef<HTMLDivElement>(null);
  const stopRequestedRef = useRef(false);
  const activeRequestIdRef = useRef<string | null>(null);

  const ccAgents = useMemo(
    () => agents.filter((agent) => agent.config.apiFormat === 'anthropic'),
    [agents]
  );
  const selectedAgents = ccAgents.filter((agent) => selectedAgentIds.includes(agent.id));
  const canSend = selectedAgents.length > 0 && inputText.trim().length > 0 && !isSending;

  // Load agents and conversations on mount
  useEffect(() => {
    fetchAgents();
    fetchMessages();
    if (convList.length === 0) {
      useConversationStore.getState().fetchConversations();
    }
  }, [fetchAgents, fetchMessages, convList.length]);

  // Auto-select agents if none selected
  useEffect(() => {
    if (selectedAgentIds.length === 0 && ccAgents.length > 0) {
      setSelectedAgentIds(ccAgents.slice(0, 3).map((agent) => agent.id));
    }
  }, [ccAgents, selectedAgentIds.length]);

  // Auto-load most recent conversation on first load
  useEffect(() => {
    if (!conversationId && convList.length > 0) {
      handleSwitchConversation(convList[0].id);
    }
  }, [convList.length > 0 && !conversationId]); // eslint-disable-line react-hooks/exhaustive-deps

  // Auto-scroll on new messages
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

  // Switch to an existing conversation
  const handleSwitchConversation = async (targetId: string) => {
    switchConversation(targetId);
    await fetchMessages({ conversationId: targetId });
    const filtered = useMessageStore.getState().messages.filter(
      (m) => m.conversationId === targetId
    );
    setChatMessages(filtered.map(toChatMessage));
    // Restore agent selection from conversation metadata
    const conv = useConversationStore.getState().conversations.find((c) => c.id === targetId);
    if (conv?.selectedAgentIds.length) {
      setSelectedAgentIds(conv.selectedAgentIds);
    }
  };

  // Create a new conversation
  const handleNewConversation = async (title: string) => {
    switchConversation(null); // clean slate
    try {
      const conv = await createConversation({ title, selectedAgentIds });
      setConversationId(conv.id);
      setChatMessages([]);
    } catch (err) {
      console.error('创建会话失败:', err);
    }
  };

  // Delete a conversation
  const handleDeleteConversation = async (id: string) => {
    try {
      await deleteConversation(id);
      removeConversationFromCache(id);
      if (conversationId === id) {
        switchConversation(null);
        setChatMessages([]);
      }
      await useConversationStore.getState().fetchConversations();
    } catch (err) {
      console.error('删除会话失败:', err);
    }
  };

  // Rename a conversation
  const handleRenameConversation = async (id: string, newTitle: string) => {
    try {
      await updateConversation({ id, title: newTitle });
      await useConversationStore.getState().fetchConversations();
    } catch (err) {
      console.error('重命名会话失败:', err);
    }
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
                phase: 'thinking',
                toolCalls: null,
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
                        phase: payload.phase ?? message.phase,
                        toolCalls: payload.tool_calls ?? message.toolCalls,
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
                  phase: payload.phase ?? null,
                  toolCalls: payload.tool_calls ?? null,
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

          if (payload.status === 'cancelled') {
            // Stop was pressed and backend confirmed — stop the while loop immediately
            stopRequestedRef.current = true;
            console.debug('[cancelled] Stop confirmed by backend, breaking loop');
          }

          if (payload.status === 'tool_use_complete') {
            // Agent finished thinking + tool execution (chunk streaming is done)
            // Turn is now fully complete — this fires BEFORE turn_complete
            console.debug(`[tool_use_complete] ${payload.speaker} finished thinking + actions`);
            // Update phase to "acting". Use messageId if available, otherwise speaker.
            setChatMessages(
              useChatStore.getState().chatMessages.map((message) =>
                (payload.messageId ? message.id === payload.messageId : message.sender === payload.speaker) &&
                message.isLoading
                  ? { ...message, phase: 'acting', toolCalls: payload.tool_calls ?? message.toolCalls }
                  : message
              )
            );
          }

          if (payload.status === 'tool_use') {
            setChatMessages(
              useChatStore.getState().chatMessages.map((message) =>
                (payload.messageId ? message.id === payload.messageId : message.sender === payload.speaker) &&
                message.isLoading
                  ? {
                      ...message,
                      phase: 'acting',
                      toolCalls: [
                        ...(message.toolCalls ?? []),
                        ...(payload.tool_calls ?? []),
                      ],
                    }
                  : message
              )
            );
          }

          if (payload.status === 'turn_complete') {
            // Agent turn fully done (including tool results incorporated into transcript)
            console.debug(`[turn_complete] ${payload.speaker} finished round ${payload.round}`);
            // Use messageId if available, otherwise fall back to speaker match
            setChatMessages(
              useChatStore.getState().chatMessages.map((message) =>
                (payload.messageId ? message.id === payload.messageId : message.sender === payload.speaker) &&
                message.isLoading
                  ? { ...message, phase: 'speaking', isLoading: false }
                  : message
              )
            );
          }

          if (payload.status === 'round_complete') {
            // Last agent in the round completed — all n agents are done
            console.debug(`[round_complete] ${payload.speaker} finished round ${payload.round} (last agent)`);
            // Use messageId if available, otherwise fall back to speaker match
            setChatMessages(
              useChatStore.getState().chatMessages.map((message) =>
                (payload.messageId ? message.id === payload.messageId : message.sender === payload.speaker) &&
                message.isLoading
                  ? { ...message, phase: 'speaking', isLoading: false }
                  : message
              )
            );
          }

          if (payload.status === 'done') {
            setChatMessages(
              useChatStore.getState().chatMessages.map((message) =>
                message.isLoading ? { ...message, isLoading: false, phase: null } : message
              )
            );
          }
        });

        let result: GroupChatResult | null = null;
        try {
          const { rounds: invokeRounds, chatMode: invokeChatMode } = useChatStore.getState();
          console.debug(`[invoke] rounds=${invokeRounds}, autoContinue=${useChatStore.getState().autoContinue}, mode=${invokeChatMode}`);
          const invokePromise = invoke<GroupChatResult>('invoke_agent_group_chat_stream', {
            input: {
              agents: readyAgents.map((agent) => ({
                id: agent.id,
                name: agent.name,
                config: agent.config,
                description: agent.description ?? null,
                address: agent.address,
              })),
              message: nextPrompt,
              rounds: invokeRounds,
              conversationId: activeConversationId,
              requestId,
              chatMode: invokeChatMode,
            },
          });
          result = await Promise.race([
            invokePromise,
            new Promise<null>((_, reject) =>
              setTimeout(() => reject(new Error('请求超时，后端响应时间过长')), INVOKE_TIMEOUT_MS)
            ),
          ]) as GroupChatResult | null;
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

        // Finalize loading messages and remove placeholder, keep streaming messages
        setChatMessages(
          useChatStore.getState().chatMessages
            .filter((message) => message.id !== loadingId)
            .map((message) => ({ ...message, isLoading: false }))
        );
        await fetchMessages();
        await fetchAgents();
        // Update conversation metadata and refresh list
        try {
          await updateConversation({
            id: result.conversationId,
            selectedAgentIds,
          });
          await useConversationStore.getState().fetchConversations();
        } catch (_) { /* ignore */ }

        const {
          autoContinue: shouldAutoContinue,
          autoContinueDelay: nextAutoContinueDelay,
        } = useChatStore.getState();
        if (!shouldAutoContinue) {
          break;
        }

        nextPrompt = '继续这段多智能体对话，基于上一轮内容自然推进，不要结束。';
        console.debug(`[auto-continue] Triggering next round in ${nextAutoContinueDelay}ms...`);
        await new Promise((resolve) => setTimeout(resolve, nextAutoContinueDelay));
      }
    } catch (error) {
      setChatMessages(
        useChatStore.getState().chatMessages
          .filter((message) => message.id !== loadingId)
          .map((message) =>
            message.isLoading
              ? { ...message, isLoading: false }
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

  const activeConvTitle = conversationId
    ? convList.find((c) => c.id === conversationId)?.title ?? '未命名会话'
    : '新对话';

  return (
    <div className="flex h-full">
      {/* Conversation Sidebar */}
      <ConversationSidebar
        conversations={convList}
        activeConversationId={conversationId}
        onSelect={handleSwitchConversation}
        onNew={handleNewConversation}
        onDelete={handleDeleteConversation}
        onRename={handleRenameConversation}
        collapsed={conversationSidebarCollapsed}
        onToggleCollapse={toggleConversationSidebar}
      />

      {/* Agent Drawer */}
      <div
        className={`flex-shrink-0 bg-gray-800 border-r border-l border-gray-700 flex flex-col transition-all ${
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

      {/* Chat Panel */}
      <div className="flex-1 bg-gray-800 flex flex-col min-w-0">
        {/* Simplified Header */}
        <div className="px-4 py-2.5 border-b border-gray-700 flex items-center justify-between gap-3">
          <div className="min-w-0">
            <div className="font-semibold text-sm">{activeConvTitle}</div>
            <div className="text-xs text-gray-500">
              {conversationId
                ? `${selectedAgents.length} 个智能体参与`
                : '选择智能体并开始对话，会话将自动创建'}
            </div>
          </div>
          <div className="flex items-center gap-3 text-sm">
            <button
              onClick={handleResetChat}
              className="h-7 px-3 rounded-md bg-gray-700 hover:bg-gray-600 text-gray-200 text-xs"
            >
              清空
            </button>
            <div className="flex items-center gap-1.5 text-xs text-gray-400">
              <span>回合</span>
              <input
                type="number"
                min={1}
                max={6}
                value={rounds}
                onChange={(e) => setRounds(Math.min(6, Math.max(1, Number(e.target.value) || 1)))}
                className="w-14 bg-gray-900 border border-gray-700 rounded px-1.5 py-0.5 text-gray-100 text-center focus:border-primary-500 focus:outline-none"
              />
            </div>
            <label className="flex items-center gap-1.5 text-xs text-gray-300 cursor-pointer">
              <input
                type="checkbox"
                checked={autoContinue}
                onChange={(e) => setAutoContinue(e.target.checked)}
                className="rounded border-gray-600"
              />
              自动续聊
            </label>
            <div className="flex items-center gap-1 text-xs text-gray-400">
              <span>间隔</span>
              <input
                type="range"
                min={0}
                max={10000}
                step={500}
                value={autoContinueDelay}
                onChange={(e) => setAutoContinueDelay(Number(e.target.value))}
                className="w-20 accent-primary-500"
              />
              <span className="w-10 text-right">{autoContinueDelay}ms</span>
            </div>
            <div className="flex items-center gap-1 text-xs text-gray-400">
              <span>模式</span>
              <select
                value={chatMode}
                onChange={(e) => setChatMode(e.target.value as 'sequential' | 'parallel' | 'debate')}
                className="bg-gray-900 border border-gray-700 rounded px-1.5 py-0.5 text-gray-100 focus:border-primary-500 focus:outline-none"
              >
                <option value="sequential">顺序</option>
                <option value="parallel">并行</option>
                <option value="debate">辩论</option>
              </select>
            </div>
          </div>
        </div>

        {/* Messages */}
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
                  <div className="flex flex-col gap-1 text-sm">
                    <div className="flex items-center gap-2">
                      <span className="animate-pulse">●</span>
                      {msg.text}
                    </div>
                    {msg.phase && (
                      <div className="text-xs opacity-60 italic">
                        {msg.phase === 'thinking' && '🤔 思考中...'}
                        {msg.phase === 'acting' && '⚡ 行动中...'}
                        {msg.phase === 'speaking' && '💬 发言中...'}
                      </div>
                    )}
                    {msg.toolCalls && msg.toolCalls.length > 0 && (
                      <div className="flex flex-col gap-0.5 mt-1">
                        {msg.toolCalls.map((tool, i) => (
                          <div key={i} className="text-xs bg-gray-800 rounded px-2 py-1 font-mono">
                            🔧 {tool.name}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                ) : (
                  renderMessageWithThinking(msg.text)
                )}
              </div>
            </div>
          ))}
          <div ref={chatEndRef} />
        </div>

        {/* Input */}
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
