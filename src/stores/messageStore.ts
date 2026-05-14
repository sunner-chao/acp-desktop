import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { ACPMessage, SendMessageInput, MessageFilter } from '../types/message';

interface MessageState {
  messages: ACPMessage[];
  conversations: string[];
  selectedConversationId: string | null;
  isLoading: boolean;
  error: string | null;

  fetchMessages: (filter?: MessageFilter) => Promise<void>;
  fetchConversations: () => Promise<void>;
  sendMessage: (input: SendMessageInput) => Promise<ACPMessage>;
  selectConversation: (id: string | null) => void;
  clearMessages: () => Promise<void>;
  clearConversationMessages: (conversationId: string) => Promise<void>;
  getConversationMessages: (conversationId: string) => ACPMessage[];
}

export const useMessageStore = create<MessageState>((set, get) => ({
  messages: [],
  conversations: [],
  selectedConversationId: null,
  isLoading: false,
  error: null,

  fetchMessages: async (filter) => {
    set({ isLoading: true, error: null });
    try {
      const messages = await invoke<ACPMessage[]>('get_messages', { filter });
      set({ messages, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  fetchConversations: async () => {
    try {
      const conversations = await invoke<string[]>('get_conversations');
      set({ conversations });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  sendMessage: async (input) => {
    set({ isLoading: true, error: null });
    try {
      const message = await invoke<ACPMessage>('send_message', { input });
      set((state) => ({
        messages: [...state.messages, message],
        isLoading: false,
      }));
      return message;
    } catch (e) {
      set({ error: String(e), isLoading: false });
      throw e;
    }
  },

  selectConversation: (id) => set({ selectedConversationId: id }),

  clearMessages: async () => {
    set({ isLoading: true, error: null });
    try {
      await invoke('clear_messages');
      set({ messages: [], conversations: [], selectedConversationId: null, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
      throw e;
    }
  },

  clearConversationMessages: async (conversationId) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('clear_conversation_messages', { conversationId });
      set((state) => ({
        messages: state.messages.filter((message) => message.conversationId !== conversationId),
        conversations: state.conversations.filter((id) => id !== conversationId),
        selectedConversationId:
          state.selectedConversationId === conversationId ? null : state.selectedConversationId,
        isLoading: false,
      }));
    } catch (e) {
      set({ error: String(e), isLoading: false });
      throw e;
    }
  },

  getConversationMessages: (conversationId) => {
    const { messages } = get();
    return messages.filter((m) => m.conversationId === conversationId);
  },
}));
