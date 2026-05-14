import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import type { ChatMessage } from '../types/chat';

interface ChatState {
  chatMessages: ChatMessage[];
  selectedAgentIds: string[];
  inputText: string;
  rounds: number;
  conversationId: string | null;
  isSending: boolean;
  isStopping: boolean;
  autoContinue: boolean;

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
  updateMessageChunk: (messageId: string, chunk: string) => void;
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
      setAutoContinue: (autoContinue) => set({ autoContinue }),
      updateMessageChunk: (messageId, chunk) =>
        set((state) => ({
          chatMessages: state.chatMessages.map((msg) =>
            msg.id === messageId
              ? { ...msg, text: msg.text + chunk }
              : msg
          ),
        })),
      resetTransientState: () =>
        set({
          chatMessages: [],
          inputText: '',
          rounds: 1,
          conversationId: null,
          isSending: false,
          isStopping: false,
          autoContinue: false,
        }),
    }),
    {
      name: 'acp-desktop-chat-state',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        chatMessages: state.chatMessages,
        selectedAgentIds: state.selectedAgentIds,
        inputText: state.inputText,
        rounds: state.rounds,
        conversationId: state.conversationId,
        autoContinue: state.autoContinue,
      }),
    }
  )
);
