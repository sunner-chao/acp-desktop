import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';

type ViewMode = 'dashboard' | 'agents' | 'messages' | 'history' | 'chat';

interface UIState {
  currentView: ViewMode;
  sidebarCollapsed: boolean;
  modalOpen: string | null;
  agentDrawerCollapsed: boolean;
  protocolDrawerCollapsed: boolean;
  historyDrawerCollapsed: boolean;

  setCurrentView: (view: ViewMode) => void;
  toggleSidebar: () => void;
  toggleAgentDrawer: () => void;
  toggleProtocolDrawer: () => void;
  toggleHistoryDrawer: () => void;
  openModal: (id: string) => void;
  closeModal: () => void;
}

export const useUIStore = create<UIState>()(
  persist(
    (set) => ({
      currentView: 'dashboard',
      sidebarCollapsed: false,
      modalOpen: null,
      agentDrawerCollapsed: false,
      protocolDrawerCollapsed: false,
      historyDrawerCollapsed: false,

      setCurrentView: (view) => set({ currentView: view }),
      toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
      toggleAgentDrawer: () =>
        set((state) => ({ agentDrawerCollapsed: !state.agentDrawerCollapsed })),
      toggleProtocolDrawer: () =>
        set((state) => ({ protocolDrawerCollapsed: !state.protocolDrawerCollapsed })),
      toggleHistoryDrawer: () =>
        set((state) => ({ historyDrawerCollapsed: !state.historyDrawerCollapsed })),
      openModal: (id) => set({ modalOpen: id }),
      closeModal: () => set({ modalOpen: null }),
    }),
    {
      name: 'acp-desktop-ui-state',
      storage: createJSONStorage(() => localStorage),
      partialize: (state) => ({
        currentView: state.currentView,
        sidebarCollapsed: state.sidebarCollapsed,
        agentDrawerCollapsed: state.agentDrawerCollapsed,
        protocolDrawerCollapsed: state.protocolDrawerCollapsed,
        historyDrawerCollapsed: state.historyDrawerCollapsed,
      }),
    }
  )
);
