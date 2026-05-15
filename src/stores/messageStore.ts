import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { ACPMessage, SendMessageInput, MessageFilter } from '../types/message';

interface MessageState {
  messages: ACPMessage[];
  isLoading: boolean;
  error: string | null;

  fetchMessages: (filter?: MessageFilter) => Promise<void>;
  sendMessage: (input: SendMessageInput) => Promise<ACPMessage>;
  clearMessages: () => Promise<void>;
  clearConversationMessages: (conversationId: string) => Promise<void>;
  resetMessages: () => void;
  getConversationMessages: (conversationId: string) => ACPMessage[];
}

export const useMessageStore = create<MessageState>((set, get) => ({
  messages: [],
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

  clearMessages: async () => {
    set({ isLoading: true, error: null });
    try {
      await invoke('clear_messages');
      set({ messages: [], isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
      throw e;
    }
  },

  resetMessages: () => set({ messages: [], error: null }),

  clearConversationMessages: async (conversationId) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('clear_conversation_messages', { conversationId });
      set((state) => ({
        messages: state.messages.filter((m) => m.conversationId !== conversationId),
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
