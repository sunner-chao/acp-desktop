import type { Agent } from '../../types';

interface LLMSettingsProps {
  agent: Agent;
}

export default function LLMSettings({ agent }: LLMSettingsProps) {
  const { config } = agent;

  return (
    <div className="bg-gray-900 rounded-lg p-4 space-y-3">
      <div className="grid grid-cols-2 gap-4 text-sm">
        <div>
          <span className="text-gray-500">API 格式:</span>
          <span className="ml-2 text-gray-300">{config.apiFormat || '未配置'}</span>
        </div>
        <div>
          <span className="text-gray-500">模型:</span>
          <span className="ml-2 text-gray-300">{config.model || '未配置'}</span>
        </div>
        <div className="col-span-2">
          <span className="text-gray-500">端点:</span>
          <span className="ml-2 text-gray-300 font-mono text-xs">{config.endpoint || '未配置'}</span>
        </div>
        <div>
          <span className="text-gray-500">温度:</span>
          <span className="ml-2 text-gray-300">{config.temperature ?? 0.7}</span>
        </div>
        <div>
          <span className="text-gray-500">最大 Token:</span>
          <span className="ml-2 text-gray-300">{config.maxTokens ?? 4096}</span>
        </div>
      </div>

      <div className="pt-2 border-t border-gray-700">
        <span className="text-gray-500 text-sm">API Key:</span>
        <span className="ml-2 text-gray-300 text-sm">
          {config.apiKey ? '••••••••' + config.apiKey.slice(-4) : '未配置'}
        </span>
      </div>
    </div>
  );
}