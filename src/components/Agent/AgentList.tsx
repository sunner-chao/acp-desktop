import { useAgentStore, useUIStore } from '../../stores';
import type { Agent } from '../../types';

export default function AgentList() {
  const { agents, selectedAgentId, selectAgent, deleteAgent, startAgentSession, stopAgentSession } = useAgentStore();
  const { openModal } = useUIStore();

  const handleDelete = async (e: React.MouseEvent, agent: Agent) => {
    e.stopPropagation();
    if (confirm(`确定要删除智能体 "${agent.name}" 吗？`)) {
      await deleteAgent(agent.id);
    }
  };

  return (
    <div className="bg-gray-800 rounded-lg border border-gray-700 h-full flex flex-col">
      <div className="p-4 border-b border-gray-700 flex items-center justify-between">
        <h3 className="font-semibold">智能体列表</h3>
        <button
          onClick={() => openModal('create-agent')}
          className="px-3 py-1 bg-primary-600 hover:bg-primary-700 rounded text-sm transition-colors"
        >
          + 新建
        </button>
      </div>

      <div className="flex-1 overflow-auto scrollbar-thin">
        {agents.length === 0 ? (
          <div className="p-8 text-center text-gray-500">
            <div className="text-4xl mb-4">🤖</div>
            <div className="text-sm">暂无智能体</div>
            <button
              onClick={() => openModal('create-agent')}
              className="mt-4 text-primary-400 hover:text-primary-300 text-sm"
            >
              创建第一个智能体
            </button>
          </div>
        ) : (
          <div className="p-2 space-y-1">
            {agents.map((agent) => (
              <div
                key={agent.id}
                onClick={() => selectAgent(agent.id)}
                className={`p-3 rounded-lg cursor-pointer transition-colors ${
                  selectedAgentId === agent.id
                    ? 'bg-primary-900/50 border border-primary-600'
                    : 'hover:bg-gray-700 border border-transparent'
                }`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <div className={`w-2 h-2 rounded-full ${agent.isOnline ? 'bg-green-400' : 'bg-gray-500'}`} />
                    <span className="font-medium">{agent.name}</span>
                  </div>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        if (agent.isOnline) {
                          void stopAgentSession(agent.id);
                        } else {
                          void startAgentSession(agent.id);
                        }
                      }}
                      className={`text-xs px-2 py-1 rounded ${
                        agent.isOnline
                          ? 'bg-gray-700 text-gray-200 hover:bg-gray-600'
                          : 'bg-green-700 text-white hover:bg-green-600'
                      }`}
                    >
                      {agent.isOnline ? '停止' : '启动'}
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        openModal('edit-agent', agent);
                      }}
                      className="text-gray-500 hover:text-blue-400 text-sm"
                      title="编辑"
                    >
                      ✎
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        openModal('copy-agent', agent);
                      }}
                      className="text-gray-500 hover:text-green-400 text-sm"
                      title="复制"
                    >
                      ⧉
                    </button>
                    <button
                      onClick={(e) => handleDelete(e, agent)}
                      className="text-gray-500 hover:text-red-400 text-sm"
                    >
                      ✕
                    </button>
                  </div>
                </div>
                <div className="text-xs text-gray-500 mt-1">{agent.address}</div>
                <div className="text-xs text-gray-600 mt-1">
                  {agent.driverType === 'llm' ? `LLM: ${agent.config.model || '未配置'}` : 'Script'}
                  <span className="ml-2">{agent.isOnline ? '在线' : '离线'}</span>
                  {agent.sessionId ? (
                    <span className="ml-2">Session {agent.sessionId.slice(0, 8)}</span>
                  ) : null}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
