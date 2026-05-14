import type { Agent } from '../../types';

interface AgentCardProps {
  agent: Agent;
}

export default function AgentCard({ agent }: AgentCardProps) {
  return (
    <div className="bg-gray-800 rounded-lg p-4 border border-gray-700 hover:border-gray-600 transition-colors">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <div className={`w-3 h-3 rounded-full ${agent.isOnline ? 'bg-green-400' : 'bg-gray-500'}`} />
          <div>
            <div className="font-semibold">{agent.name}</div>
            <div className="text-xs text-gray-500">{agent.address}</div>
          </div>
        </div>
        <span className={`px-2 py-1 rounded text-xs ${
          agent.driverType === 'llm' ? 'bg-primary-900 text-primary-300' : 'bg-gray-700 text-gray-300'
        }`}>
          {agent.driverType === 'llm' ? 'LLM' : 'Script'}
        </span>
      </div>

      {agent.description && (
        <div className="text-sm text-gray-400 mb-3 line-clamp-2">{agent.description}</div>
      )}

      <div className="flex items-center justify-between text-xs text-gray-500">
        <span>{agent.config.model || '未配置模型'}</span>
        <span>{agent.isOnline ? '在线' : '离线'}</span>
      </div>
    </div>
  );
}