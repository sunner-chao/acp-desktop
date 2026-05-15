import { useEffect, useState } from 'react';
import { useMessageStore, useConversationStore, useUIStore } from '../../stores';
import type { ACPPerformative } from '../../types';

export default function MessageHistory() {
  const {
    messages,
    fetchMessages,
    clearMessages,
    clearConversationMessages,
  } = useMessageStore();
  const { conversations: convList, fetchConversations, selectedConversationId, selectConversation } = useConversationStore();
  const { historyDrawerCollapsed, toggleHistoryDrawer } = useUIStore();
  const [filter, setFilter] = useState<{ sender?: string; receiver?: string; performative?: ACPPerformative }>({});

  useEffect(() => {
    fetchMessages();
    if (convList.length === 0) fetchConversations();
  }, [fetchMessages, convList.length]);

  const filteredMessages = messages.filter((msg) => {
    if (selectedConversationId && msg.conversationId !== selectedConversationId) return false;
    if (filter.sender && !msg.sender.includes(filter.sender)) return false;
    if (filter.receiver && !msg.receiver.includes(filter.receiver)) return false;
    if (filter.performative && msg.performative !== filter.performative) return false;
    return true;
  });

  const getPerformativeColor = (p: ACPPerformative) => {
    switch (p) {
      case 'request': return 'bg-blue-900 text-blue-300';
      case 'inform': return 'bg-green-900 text-green-300';
      case 'query': return 'bg-yellow-900 text-yellow-300';
      case 'agree': return 'bg-purple-900 text-purple-300';
      case 'refuse': return 'bg-red-900 text-red-300';
    }
  };

  return (
    <div className="h-full flex flex-col">
      <div className="mb-4 flex items-center justify-between gap-3">
        <div>
          <h2 className="text-2xl font-bold mb-2">消息历史</h2>
          <p className="text-gray-400">查看所有智能体间的消息记录</p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={async () => {
              if (confirm('确定要清空全部历史消息吗？')) {
                await clearMessages();
                await useConversationStore.getState().fetchConversations();
              }
            }}
            className="px-3 py-2 text-sm rounded bg-red-700 hover:bg-red-600 text-white"
          >
            清空全部
          </button>
          <button
            onClick={toggleHistoryDrawer}
            className="h-9 w-9 rounded-md bg-gray-800 border border-gray-700 hover:bg-gray-700 text-gray-300"
            title={historyDrawerCollapsed ? '展开会话栏' : '折叠会话栏'}
          >
            {historyDrawerCollapsed ? '‹' : '›'}
          </button>
        </div>
      </div>

      <div className="bg-gray-800 rounded-lg border border-gray-700 mb-4">
        <div className="p-4 border-b border-gray-700 flex items-center gap-4">
          <span className="text-sm text-gray-400">过滤器:</span>
          <input
            type="text"
            placeholder="发送者"
            value={filter.sender || ''}
            onChange={(e) => setFilter((prev) => ({ ...prev, sender: e.target.value || undefined }))}
            className="bg-gray-900 border border-gray-700 rounded px-3 py-1 text-sm text-gray-100 focus:border-primary-500 focus:outline-none"
          />
          <input
            type="text"
            placeholder="接收者"
            value={filter.receiver || ''}
            onChange={(e) => setFilter((prev) => ({ ...prev, receiver: e.target.value || undefined }))}
            className="bg-gray-900 border border-gray-700 rounded px-3 py-1 text-sm text-gray-100 focus:border-primary-500 focus:outline-none"
          />
          <select
            value={filter.performative || ''}
            onChange={(e) => setFilter((prev) => ({ ...prev, performative: (e.target.value || undefined) as ACPPerformative | undefined }))}
            className="bg-gray-900 border border-gray-700 rounded px-3 py-1 text-sm text-gray-100 focus:border-primary-500 focus:outline-none"
          >
            <option value="">所有类型</option>
            <option value="request">request</option>
            <option value="inform">inform</option>
            <option value="query">query</option>
            <option value="agree">agree</option>
            <option value="refuse">refuse</option>
          </select>
        </div>
      </div>

      <div className="flex-1 bg-gray-800 rounded-lg border border-gray-700 overflow-hidden flex">
        <div className={`border-r border-gray-700 overflow-auto transition-all ${historyDrawerCollapsed ? 'w-14' : 'w-48'}`}>
          <div className="p-2 space-y-1">
            <div
              onClick={() => selectConversation(null)}
              className={`p-2 rounded cursor-pointer ${
                !selectedConversationId ? 'bg-primary-900/50 text-primary-300' : 'hover:bg-gray-700'
              }`}
            >
              {historyDrawerCollapsed ? 'A' : '所有会话'}
            </div>
            {convList.map((conv) => (
              <div
                key={conv.id}
                onClick={() => selectConversation(conv.id)}
                className={`p-2 rounded cursor-pointer text-xs truncate ${
                  selectedConversationId === conv.id ? 'bg-primary-900/50 text-primary-300' : 'hover:bg-gray-700'
                }`}
              >
                {historyDrawerCollapsed ? conv.id.slice(0, 2) : `${conv.title.slice(0, 12)}`}
              </div>
            ))}
          </div>
        </div>

        <div className="flex-1 overflow-auto">
          {selectedConversationId && (
            <div className="px-4 py-2 flex items-center justify-between border-b border-gray-700 bg-gray-900/60">
              <div className="text-sm text-gray-300 font-mono">会话 {selectedConversationId}</div>
              <div className="flex items-center gap-2">
                <button
                  onClick={async () => {
                    if (confirm('确定要删除当前会话的全部消息吗？')) {
                      await clearConversationMessages(selectedConversationId);
                      await fetchConversations();
                      selectConversation(null);
                    }
                  }}
                  className="px-3 py-1.5 text-xs rounded bg-red-700 hover:bg-red-600 text-white"
                >
                  删除当前
                </button>
                <button
                  onClick={() => selectConversation(null)}
                  className="px-3 py-1.5 text-xs rounded bg-gray-700 hover:bg-gray-600 text-gray-100"
                >
                  取消筛选
                </button>
              </div>
            </div>
          )}
          {filteredMessages.length === 0 ? (
            <div className="flex items-center justify-center h-full text-gray-500">
              <div className="text-center">
                <div className="text-4xl mb-4">📭</div>
                <div>暂无消息记录</div>
              </div>
            </div>
          ) : (
            <table className="w-full">
              <thead className="bg-gray-900 sticky top-0">
                <tr className="text-left text-sm text-gray-400">
                  <th className="p-3">时间</th>
                  <th className="p-3">类型</th>
                  <th className="p-3">发送者</th>
                  <th className="p-3">接收者</th>
                  <th className="p-3">内容</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-700">
                {filteredMessages.map((msg) => (
                  <tr key={msg.id} className="hover:bg-gray-700/50">
                    <td className="p-3 text-xs text-gray-500 whitespace-nowrap">
                      {new Date(msg.timestamp).toLocaleString()}
                    </td>
                    <td className="p-3">
                      <span className={`px-2 py-0.5 rounded text-xs ${getPerformativeColor(msg.performative)}`}>
                        {msg.performative}
                      </span>
                    </td>
                    <td className="p-3 text-sm">{msg.sender.split('/').pop()}</td>
                    <td className="p-3 text-sm">{msg.receiver.split('/').pop()}</td>
                    <td className="p-3 text-sm text-gray-300 max-w-xs truncate">
                      {JSON.stringify(msg.content)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </div>
    </div>
  );
}
