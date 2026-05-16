import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import type { ChatMessage, ConversationSnapshot } from '../types/chat';

interface ChatState {
  chatMessages: ChatMessage[];
  selectedAgentIds: string[];
  inputText: string;
  rounds: number;
  conversationId: string | null;
  isSending: boolean;
  isStopping: boolean;
  autoContinue: boolean;
  autoContinueDelay: number;
  chatMode: 'sequential' | 'parallel' | 'debate';
  conversationCache: Record<string, ConversationSnapshot>;

  setChatMessages: (messages: ChatMessage[]) => void;
  appendChatMessages: (messages: ChatMessage[]) => void;
  setSelectedAgentIds: (ids: string[]) => void;
  toggleSelectedAgent: (agentId: string) => void;
  setInputText: (text: string) => void;
  setRounds: (rounds: number) => void;
  setConversationId: (conversationId: string | null) => void;
  setIsSending: (isSending: boolean) => void;
  setIsStopping: (isStopping: boolean) => void;
  setAutoContinue: (autoContinue: boolean) => void;
  setAutoContinueDelay: (delay: number) => void;
  setChatMode: (mode: 'sequential' | 'parallel' | 'debate') => void;
  updateMessageChunk: (messageId: string, chunk: string) => void;
  switchConversation: (targetId: string | null) => void;
  removeConversationFromCache: (id: string) => void;
  resetTransientState: () => void;
}

export const useChatStore = create<ChatState>()(
  persist(
    (set) => ({
      chatMessages: [],
      selectedAgentIds: [],
      inputText: '',
      rounds: 1,
      conversationId: null,
      isSending: false,
      isStopping: false,
      autoContinue: false,
      autoContinueDelay: 3000, // 3 seconds between agent turns in sequential mode
      chatMode: 'sequential',
      conversationCache: {},

      setChatMessages: (messages) => set({ chatMessages: messages }),
      appendChatMessages: (messages) =>
        set((state) => ({ chatMessages: [...state.chatMessages, ...messages] })),
      setSelectedAgentIds: (ids) => set({ selectedAgentIds: ids }),
      toggleSelectedAgent: (agentId) =>
        set((state) => ({
          selectedAgentIds: state.selectedAgentIds.includes(agentId)
            ? state.selectedAgentIds.filter((id) => id !== agentId)
            : [...state.selectedAgentIds, agentId],
        })),
      setInputText: (text) => set({ inputText: text }),
      setRounds: (rounds) => set({ rounds }),
      setConversationId: (conversationId) => set({ conversationId }),
      setIsSending: (isSending) => set({ isSending }),
      setIsStopping: (isStopping) => set({ isStopping }),
      setAutoContinue: (autoContinue: boolean) => set({ autoContinue }),
      setAutoContinueDelay: (delay: number) => set({ autoContinueDelay: delay }),
      setChatMode: (mode: 'sequential' | 'parallel' | 'debate') => set({ chatMode: mode }),
      updateMessageChunk: (messageId, chunk) =>
        set((state) => ({
          chatMessages: state.chatMessages.map((msg) =>
            msg.id === messageId
              ? { ...msg, text: msg.text + chunk }
              : msg
          ),
        })),

      switchConversation: (targetId) =>
        set((state) => {
          const nextCache = { ...state.conversationCache };
          // Save current conversation UI state to cache
          if (state.conversationId && state.conversationId !== targetId) {
            nextCache[state.conversationId] = {
              selectedAgentIds: state.selectedAgentIds,
              inputText: state.inputText,
              rounds: state.rounds,
              autoContinue: state.autoContinue,
            };
          }

          // Restore target conversation's UI state from cache, or defaults
          const snapshot = targetId ? nextCache[targetId] : null;
          return {
            conversationId: targetId,
            chatMessages: [], // Messages loaded from DB by caller
            selectedAgentIds: snapshot?.selectedAgentIds ?? [],
            inputText: snapshot?.inputText ?? '',
            rounds: snapshot?.rounds ?? 1,
            autoContinue: snapshot?.autoContinue ?? false,
            conversationCache: nextCache,
          };
        }),

      removeConversationFromCache: (id) =>
        set((state) => {
          const nextCache = { ...state.conversationCache };
          delete nextCache[id];
          return { conversationCache: nextCache };
        }),

      resetTransientState: () =>
        set({
          chatMessages: [],
          inputText: '',
          rounds: 1,
          conversationId: null,
          isSending: false,
          isStopping: false,
          autoContinue: false,
          autoContinueDelay: 3000,
          chatMode: 'sequential',
          conversationCache: {},
        }),
    }),
    {
      name: 'acp-desktop-chat-state',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        selectedAgentIds: state.selectedAgentIds,
        inputText: state.inputText,
        rounds: state.rounds,
        conversationId: state.conversationId,
        autoContinue: state.autoContinue,
        autoContinueDelay: state.autoContinueDelay,
        chatMode: state.chatMode,
        conversationCache: state.conversationCache,
        // chatMessages NOT persisted — loaded from DB
      }),
    }
  )
);
