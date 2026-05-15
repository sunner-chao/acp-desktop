import { useState } from 'react';
import { useUIStore, useAgentStore, useMessageStore, useChatStore, useConversationStore } from '../../stores';
import { invoke } from '@tauri-apps/api/core';

export default function Header() {
  const { toggleSidebar } = useUIStore();
  const [showResetModal, setShowResetModal] = useState(false);
  const [resetConfirmText, setResetConfirmText] = useState('');

  const handleOpenReset = () => {
    setShowResetModal(true);
    setResetConfirmText('');
  };

  const handleCloseReset = () => {
    setShowResetModal(false);
    setResetConfirmText('');
  };

  const handleResetDatabase = async () => {
    if (resetConfirmText !== 'RESET') return;
    try {
      await invoke('reset_database');
      useAgentStore.getState().resetAgents();
      useMessageStore.getState().resetMessages();
      useChatStore.getState().resetTransientState();
      useConversationStore.getState().resetConversations();
      handleCloseReset();
    } catch (e) {
      console.error('Failed to reset database:', e);
    }
  };

  return (
    <>
      <header className="h-14 bg-gray-800 border-b border-gray-700 flex items-center justify-between px-4">
        <div className="flex items-center gap-4">
          <button
            onClick={toggleSidebar}
            className="p-2 rounded-lg hover:bg-gray-700 text-gray-400 hover:text-gray-200"
          >
            ☰
          </button>
          <span className="text-sm text-gray-400">ACP 多智能体通信桌面端应用</span>
        </div>

        <div className="flex items-center gap-4">
          <button
            onClick={handleOpenReset}
            className="px-3 py-1 text-xs rounded bg-red-900 hover:bg-red-800 text-red-300 border border-red-700"
          >
            重置数据
          </button>
          <span className="text-xs text-gray-500">多智能体通信协议 (ACP)</span>
        </div>
      </header>

      {/* Reset Confirmation Modal */}
      {showResetModal && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-gray-800 border border-red-700 rounded-xl p-6 w-96 shadow-2xl">
            <div className="flex items-center gap-3 mb-4">
              <div className="text-2xl">⚠️</div>
              <h2 className="text-lg font-semibold text-red-300">危险操作</h2>
            </div>
            <p className="text-sm text-gray-300 mb-4 leading-relaxed">
              此操作将永久删除所有智能体、消息和会话记录，且<span className="text-red-400 font-semibold">无法撤销</span>。
            </p>
            <p className="text-xs text-gray-500 mb-4">
              请在下方输入 <span className="font-mono text-red-400">RESET</span> 确认：
            </p>
            <input
              type="text"
              value={resetConfirmText}
              onChange={(e) => setResetConfirmText(e.target.value.toUpperCase())}
              placeholder="RESET"
              className="w-full mb-4 px-3 py-2 bg-gray-900 border border-gray-700 rounded-lg text-gray-100 font-mono text-center tracking-widest focus:border-red-500 focus:outline-none"
              autoFocus
            />
            <div className="flex gap-3">
              <button
                onClick={handleResetDatabase}
                disabled={resetConfirmText !== 'RESET'}
                className={`flex-1 px-4 py-2 rounded-lg font-medium text-sm transition-colors ${
                  resetConfirmText === 'RESET'
                    ? 'bg-red-700 hover:bg-red-600 text-white'
                    : 'bg-gray-700 text-gray-500 cursor-not-allowed'
                }`}
              >
                确认删除
              </button>
              <button
                onClick={handleCloseReset}
                className="flex-1 px-4 py-2 bg-gray-700 hover:bg-gray-600 text-gray-200 rounded-lg text-sm transition-colors"
              >
                取消
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}