import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';
import type { Agent, CreateAgentInput, UpdateAgentInput } from '../types/agent';

interface AgentState {
  agents: Agent[];
  selectedAgentId: string | null;
  isLoading: boolean;
  error: string | null;

  fetchAgents: () => Promise<void>;
  createAgent: (input: CreateAgentInput) => Promise<Agent>;
  updateAgent: (input: UpdateAgentInput) => Promise<Agent>;
  deleteAgent: (id: string) => Promise<void>;
  startAgentSession: (id: string) => Promise<Agent>;
  stopAgentSession: (id: string) => Promise<Agent>;
  selectAgent: (id: string | null) => void;
  resetAgents: () => void;
  importAgents: (json: string) => Promise<Agent[]>;
  exportAgents: () => string;
}

export const useAgentStore = create<AgentState>((set, get) => ({
  agents: [],
  selectedAgentId: null,
  isLoading: false,
  error: null,

  fetchAgents: async () => {
    set({ isLoading: true, error: null });
    try {
      const agents = await invoke<Agent[]>('get_agents');
      set({ agents, isLoading: false });
    } catch (e) {
      set({ error: String(e), isLoading: false });
    }
  },

  createAgent: async (input) => {
    set({ isLoading: true, error: null });
    try {
      const agent = await invoke<Agent>('create_agent', { input });
      set((state) => ({
        agents: [...state.agents, agent],
        isLoading: false,
      }));
      return agent;
    } catch (e) {
      set({ error: String(e), isLoading: false });
      throw e;
    }
  },

  updateAgent: async (input) => {
    set({ isLoading: true, error: null });
    try {
      const agent = await invoke<Agent>('update_agent', { input });
      set((state) => ({
        agents: state.agents.map((a) => (a.id === agent.id ? agent : a)),
        isLoading: false,
      }));
      return agent;
    } catch (e) {
      set({ error: String(e), isLoading: false });
      throw e;
    }
  },

  deleteAgent: async (id) => {
    set({ isLoading: true, error: null });
    try {
      await invoke('delete_agent', { id });
      set((state) => ({
        agents: state.agents.filter((a) => a.id !== id),
        selectedAgentId: state.selectedAgentId === id ? null : state.selectedAgentId,
        isLoading: false,
      }));
    } catch (e) {
      set({ error: String(e), isLoading: false });
      throw e;
    }
  },

  startAgentSession: async (id) => {
    set({ isLoading: true, error: null });
    try {
      const agent = await invoke<Agent>('start_agent_session', { id });
      set((state) => ({
        agents: state.agents.map((a) => (a.id === agent.id ? agent : a)),
        isLoading: false,
      }));
      return agent;
    } catch (e) {
      set({ error: String(e), isLoading: false });
      throw e;
    }
  },

  stopAgentSession: async (id) => {
    set({ isLoading: true, error: null });
    try {
      const agent = await invoke<Agent>('stop_agent_session', { id });
      set((state) => ({
        agents: state.agents.map((a) => (a.id === agent.id ? agent : a)),
        isLoading: false,
      }));
      return agent;
    } catch (e) {
      set({ error: String(e), isLoading: false });
      throw e;
    }
  },

  selectAgent: (id) => set({ selectedAgentId: id }),

  resetAgents: () => set({ agents: [], selectedAgentId: null, error: null }),

  importAgents: async (json) => {
    const agents = await invoke<Agent[]>('import_agents', { json });
    set((state) => ({ agents: [...state.agents, ...agents] }));
    return agents;
  },

  exportAgents: () => {
    const { agents } = get();
    return JSON.stringify(agents, null, 2);
  },
}));
