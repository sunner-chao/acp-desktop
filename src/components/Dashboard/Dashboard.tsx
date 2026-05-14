import { useAgentStore, useMessageStore } from '../../stores';
import AgentCard from './AgentCard';

export default function Dashboard() {
  const { agents } = useAgentStore();
  const { messages, conversations } = useMessageStore();

  const onlineCount = agents.filter((a) => a.isOnline).length;
  const offlineCount = agents.length - onlineCount;

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-2xl font-bold mb-4">驾驶舱</h2>
        <p className="text-gray-400">实时监控和管理您的多智能体系统</p>
      </div>

      <div className="grid grid-cols-4 gap-4">
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="text-3xl font-bold text-primary-400">{agents.length}</div>
          <div className="text-sm text-gray-400">总智能体</div>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="text-3xl font-bold text-green-400">{onlineCount}</div>
          <div className="text-sm text-gray-400">在线</div>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="text-3xl font-bold text-gray-400">{offlineCount}</div>
          <div className="text-sm text-gray-400">离线</div>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="text-3xl font-bold text-yellow-400">{conversations.length}</div>
          <div className="text-sm text-gray-400">活跃会话</div>
        </div>
      </div>

      <div>
        <h3 className="text-lg font-semibold mb-3">智能体状态</h3>
        {agents.length === 0 ? (
          <div className="bg-gray-800 rounded-lg p-8 border border-gray-700 text-center">
            <div className="text-4xl mb-4">🤖</div>
            <div className="text-gray-400 mb-4">暂无智能体</div>
            <div className="text-sm text-gray-500">前往"智能体"页面创建您的第一个智能体</div>
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-4">
            {agents.slice(0, 6).map((agent) => (
              <AgentCard key={agent.id} agent={agent} />
            ))}
          </div>
        )}
      </div>

      <div>
        <h3 className="text-lg font-semibold mb-3">最近消息</h3>
        <div className="bg-gray-800 rounded-lg border border-gray-700">
          {messages.length === 0 ? (
            <div className="p-8 text-center text-gray-500">暂无消息记录</div>
          ) : (
            <div className="divide-y divide-gray-700">
              {messages.slice(-5).reverse().map((msg) => (
                <div key={msg.id} className="p-4">
                  <div className="flex items-center gap-2 mb-1">
                    <span className={`px-2 py-0.5 rounded text-xs ${
                      msg.performative === 'request' ? 'bg-blue-900 text-blue-300' :
                      msg.performative === 'inform' ? 'bg-green-900 text-green-300' :
                      msg.performative === 'query' ? 'bg-yellow-900 text-yellow-300' :
                      msg.performative === 'agree' ? 'bg-purple-900 text-purple-300' :
                      'bg-red-900 text-red-300'
                    }`}>
                      {msg.performative}
                    </span>
                    <span className="text-xs text-gray-500">{msg.timestamp}</span>
                  </div>
                  <div className="text-sm">
                    <span className="text-primary-400">{msg.sender.split('/').pop()}</span>
                    <span className="text-gray-500 mx-2">→</span>
                    <span className="text-secondary-400">{msg.receiver.split('/').pop()}</span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}