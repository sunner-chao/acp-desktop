import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { Conversation, CreateConversationInput, UpdateConversationInput } from '../types/conversation';

interface ConversationState {
  conversations: Conversation[];
  activeConversationId: string | null;
  selectedConversationId: string | null;
  isLoading: boolean;
  error: string | null;

  fetchConversations: () => Promise<void>;
  createConversation: (input: CreateConversationInput) => Promise<Conversation>;
  deleteConversation: (id: string) => Promise<void>;
  updateConversation: (input: UpdateConversationInput) => Promise<Conversation>;
  setActiveConversation: (id: string | null) => void;
  selectConversation: (id: string | null) => void;
  resetConversations: () => void;
}

export const useConversationStore = create<ConversationState>((set) => ({
  conversations: [],
  activeConversationId: null,
  selectedConversationId: null,
  isLoading: false,
  error: null,

  fetchConversations: async () => {
    set({ isLoading: true, error: null });
    try {
      const conversations = await invoke<Conversation[]>('list_conversations');
      set({ conversations, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  createConversation: async (input) => {
    set({ isLoading: true, error: null });
    try {
      const conversation = await invoke<Conversation>('create_conversation', { input });
      set((state) => ({
        conversations: [conversation, ...state.conversations],
        activeConversationId: conversation.id,
        isLoading: false,
      }));
      return conversation;
    } catch (e) {
      set({ error: String(e), isLoading: false });
      throw e;
    }
  },

  deleteConversation: async (id) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('delete_conversation', { id });
      set((state) => ({
        conversations: state.conversations.filter((c) => c.id !== id),
        activeConversationId: state.activeConversationId === id ? null : state.activeConversationId,
        isLoading: false,
      }));
    } catch (e) {
      set({ error: String(e), isLoading: false });
      throw e;
    }
  },

  updateConversation: async (input) => {
    set({ isLoading: true, error: null });
    try {
      const conversation = await invoke<Conversation>('update_conversation', { input });
      set((state) => ({
        conversations: state.conversations.map((c) => (c.id === conversation.id ? conversation : c)),
        isLoading: false,
      }));
      return conversation;
    } catch (e) {
      set({ error: String(e), isLoading: false });
      throw e;
    }
  },

  setActiveConversation: (id) => set({ activeConversationId: id }),
  selectConversation: (id) => set({ selectedConversationId: id }),

  resetConversations: () => set({ conversations: [], activeConversationId: null, selectedConversationId: null, error: null }),
}));
