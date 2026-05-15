import { useState, useRef, useEffect } from 'react';
import type { Conversation } from '../../types/conversation';

interface ConversationSidebarProps {
  conversations: Conversation[];
  activeConversationId: string | null;
  onSelect: (id: string) => void;
  onNew: (title: string) => void;
  onDelete: (id: string) => void;
  onRename: (id: string, newTitle: string) => void;
  collapsed: boolean;
  onToggleCollapse: () => void;
}

export default function ConversationSidebar({
  conversations,
  activeConversationId,
  onSelect,
  onNew,
  onDelete,
  onRename,
  collapsed,
  onToggleCollapse,
}: ConversationSidebarProps) {
  const [creating, setCreating] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameTitle, setRenameTitle] = useState('');
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const newInputRef = useRef<HTMLInputElement>(null);
  const renameInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (creating && newInputRef.current) newInputRef.current.focus();
  }, [creating]);

  useEffect(() => {
    if (renamingId && renameInputRef.current) renameInputRef.current.focus();
  }, [renamingId]);

  const handleCreateSubmit = () => {
    const trimmed = newTitle.trim();
    if (trimmed) {
      onNew(trimmed);
      setNewTitle('');
      setCreating(false);
    }
  };

  const handleCreateKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleCreateSubmit();
    if (e.key === 'Escape') { setCreating(false); setNewTitle(''); }
  };

  const handleRenameSubmit = () => {
    const trimmed = renameTitle.trim();
    if (trimmed && renamingId) {
      onRename(renamingId, trimmed);
    }
    setRenamingId(null);
    setRenameTitle('');
  };

  const handleRenameKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleRenameSubmit();
    if (e.key === 'Escape') { setRenamingId(null); setRenameTitle(''); }
  };

  const startRename = (conv: Conversation) => {
    setRenamingId(conv.id);
    setRenameTitle(conv.title);
  };

  const confirmDelete = (id: string) => {
    onDelete(id);
    setDeletingId(null);
  };

  const formatTime = (isoStr: string) => {
    try {
      const d = new Date(isoStr);
      const now = new Date();
      const isToday = d.toDateString() === now.toDateString();
      if (isToday) return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
      return d.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' });
    } catch {
      return '';
    }
  };

  if (collapsed) {
    return (
      <div className="w-12 bg-gray-800 border-r border-gray-700 flex flex-col items-center py-3 gap-1">
        <button
          onClick={onToggleCollapse}
          className="w-8 h-8 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 flex items-center justify-center text-xs"
          title="展开会话列表"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <polyline points="9 18 15 12 9 6" />
          </svg>
        </button>
        <div className="w-px h-4 bg-gray-700 my-1" />
        {conversations.slice(0, 8).map((conv) => (
          <button
            key={conv.id}
            onClick={() => onSelect(conv.id)}
            className={`w-8 h-8 rounded-full text-xs font-medium flex items-center justify-center transition-colors ${
              conv.id === activeConversationId
                ? 'bg-primary-600 text-white'
                : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
            }`}
            title={conv.title}
          >
            {conv.title.charAt(0).toUpperCase()}
          </button>
        ))}
        {conversations.length > 8 && (
          <span className="text-xs text-gray-500 mt-1">+{conversations.length - 8}</span>
        )}
      </div>
    );
  }

  return (
    <div className="w-64 bg-gray-800 border-r border-gray-700 flex flex-col min-h-0">
      {/* Header */}
      <div className="p-3 border-b border-gray-700 flex items-center justify-between">
        <h3 className="text-sm font-semibold text-gray-200">会话列表</h3>
        <div className="flex items-center gap-1">
          <button
            onClick={() => setCreating(true)}
            className="w-6 h-6 rounded hover:bg-gray-700 text-primary-400 hover:text-primary-300 flex items-center justify-center"
            title="新建会话"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
              <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
            </svg>
          </button>
          <button
            onClick={onToggleCollapse}
            className="w-6 h-6 rounded hover:bg-gray-700 text-gray-400 hover:text-gray-200 flex items-center justify-center"
            title="收起"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <polyline points="15 18 9 12 15 6" />
            </svg>
          </button>
        </div>
      </div>

      {/* Inline create input */}
      {creating && (
        <div className="px-2 py-2 border-b border-gray-700">
          <input
            ref={newInputRef}
            value={newTitle}
            onChange={(e) => setNewTitle(e.target.value)}
            onKeyDown={handleCreateKeyDown}
            onBlur={() => { if (!newTitle.trim()) { setCreating(false); setNewTitle(''); } }}
            placeholder="输入会话名称..."
            className="w-full bg-gray-900 border border-primary-500 rounded px-2 py-1.5 text-sm text-gray-100 placeholder-gray-500 outline-none"
          />
        </div>
      )}

      {/* Conversation list */}
      <div className="flex-1 overflow-auto scrollbar-thin">
        {conversations.length === 0 && !creating && (
          <div className="px-4 py-8 text-center text-xs text-gray-500">
            暂无会话<br />
            <span className="text-gray-600">点击 + 创建新会话，或直接开始对话</span>
          </div>
        )}

        {conversations.map((conv) => (
          <div key={conv.id} className="group relative">
            {/* Active conversation */}
            {conv.id === activeConversationId && (
              <div className="absolute left-0 top-0 bottom-0 w-0.5 bg-primary-500" />
            )}

            <button
              onClick={() => {
                if (renamingId !== conv.id) onSelect(conv.id);
              }}
              className={`w-full text-left px-3 py-2.5 transition-colors ${
                conv.id === activeConversationId
                  ? 'bg-primary-900/20 hover:bg-primary-900/30'
                  : 'hover:bg-gray-700/60'
              }`}
            >
              {/* Rename inline */}
              {renamingId === conv.id ? (
                <input
                  ref={renameInputRef}
                  value={renameTitle}
                  onChange={(e) => setRenameTitle(e.target.value)}
                  onKeyDown={handleRenameKeyDown}
                  onBlur={handleRenameSubmit}
                  onClick={(e) => e.stopPropagation()}
                  className="w-full bg-gray-900 border border-primary-500 rounded px-1.5 py-0.5 text-sm text-gray-100 outline-none"
                />
              ) : (
                <div className="flex items-center gap-2 min-w-0">
                  <div className={`w-2 h-2 rounded-full flex-shrink-0 ${
                    conv.id === activeConversationId ? 'bg-primary-400' : 'bg-gray-600'
                  }`} />
                  <div className="min-w-0 flex-1">
                    <div className="text-sm text-gray-100 truncate">{conv.title}</div>
                    <div className="text-xs text-gray-500 mt-0.5">
                      {conv.selectedAgentIds.length > 0 && `${conv.selectedAgentIds.length} 智能体 · `}
                      {formatTime(conv.updatedAt)}
                    </div>
                  </div>
                </div>
              )}
            </button>

            {/* Action buttons on hover */}
            {renamingId !== conv.id && (
              <div className="absolute right-2 top-1/2 -translate-y-1/2 flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
                <button
                  onClick={(e) => { e.stopPropagation(); startRename(conv); }}
                  className="w-6 h-6 rounded hover:bg-gray-600 text-gray-400 hover:text-gray-200 flex items-center justify-center"
                  title="重命名"
                >
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z" />
                  </svg>
                </button>
                <button
                  onClick={(e) => { e.stopPropagation(); setDeletingId(conv.id); }}
                  className="w-6 h-6 rounded hover:bg-gray-600 text-gray-400 hover:text-red-400 flex items-center justify-center"
                  title="删除"
                >
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M3 6h18" /><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6" />
                    <path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2" />
                  </svg>
                </button>
              </div>
            )}

            {/* Delete confirmation */}
            {deletingId === conv.id && (
              <div className="absolute inset-0 bg-gray-800/95 flex items-center justify-center gap-2 px-3 z-10">
                <span className="text-xs text-gray-300">确认删除?</span>
                <button
                  onClick={(e) => { e.stopPropagation(); confirmDelete(conv.id); }}
                  className="px-2 py-0.5 bg-red-600 hover:bg-red-500 text-white text-xs rounded"
                >
                  是
                </button>
                <button
                  onClick={(e) => { e.stopPropagation(); setDeletingId(null); }}
                  className="px-2 py-0.5 bg-gray-700 hover:bg-gray-600 text-gray-300 text-xs rounded"
                >
                  否
                </button>
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Bottom: New conversation button */}
      <div className="border-t border-gray-700 p-2">
        <button
          onClick={() => setCreating(true)}
          className="w-full text-left px-3 py-2 rounded hover:bg-gray-700/60 text-primary-400 hover:text-primary-300 text-sm flex items-center gap-2 transition-colors"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          新建会话
        </button>
      </div>
    </div>
  );
}
