import { useState } from 'react';
import { useAgentStore } from '../../stores';
import type { CreateAgentInput, DriverType } from '../../types';

interface AgentFormProps {
  onClose: () => void;
  initialValues?: CreateAgentInput;
  mode?: 'create' | 'edit' | 'copy';
  editId?: string;
}

export default function AgentForm({ onClose, initialValues, mode = 'create', editId }: AgentFormProps) {
  const { createAgent, updateAgent } = useAgentStore();
  const [isSubmitting, setIsSubmitting] = useState(false);

  const [formData, setFormData] = useState<CreateAgentInput>(
    initialValues || {
      name: '',
      description: '',
      driverType: 'llm',
      config: {
        apiFormat: 'openai',
        endpoint: '',
        apiKey: '',
        model: '',
        temperature: 0.7,
        maxTokens: 4096,
      },
    }
  );

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!formData.name.trim()) {
      alert('请输入智能体名称');
      return;
    }

    setIsSubmitting(true);
    try {
      if (mode === 'edit' && editId) {
        await updateAgent({ id: editId, ...formData });
      } else {
        await createAgent(formData);
      }
      onClose();
    } catch (error) {
      console.error('AgentForm submit error:', error);
      console.error('Mode:', mode, 'FormData:', JSON.stringify(formData));
      alert(`${mode === 'edit' ? '更新' : '创建'}失败: ${error}`);
    } finally {
      setIsSubmitting(false);
    }
  };

  const updateConfig = (key: string, value: unknown) => {
    setFormData((prev) => ({
      ...prev,
      config: { ...prev.config, [key]: value },
    }));
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <div>
        <label className="block text-sm font-medium text-gray-400 mb-1">名称 *</label>
        <input
          type="text"
          value={formData.name}
          onChange={(e) => setFormData((prev) => ({ ...prev, name: e.target.value }))}
          className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none"
          placeholder="例如: assistant"
        />
      </div>

      <div>
        <label className="block text-sm font-medium text-gray-400 mb-1">描述</label>
        <textarea
          value={formData.description || ''}
          onChange={(e) => setFormData((prev) => ({ ...prev, description: e.target.value }))}
          className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none"
          rows={3}
          placeholder="智能体的功能和角色描述..."
        />
      </div>

      <div>
        <label className="block text-sm font-medium text-gray-400 mb-1">驱动类型</label>
        <div className="flex gap-4">
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="radio"
              name="driverType"
              value="llm"
              checked={formData.driverType === 'llm'}
              onChange={() => setFormData((prev) => ({ ...prev, driverType: 'llm' as DriverType }))}
              className="text-primary-600"
            />
            <span>LLM 驱动</span>
          </label>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="radio"
              name="driverType"
              value="script"
              checked={formData.driverType === 'script'}
              onChange={() => setFormData((prev) => ({ ...prev, driverType: 'script' as DriverType }))}
              className="text-primary-600"
            />
            <span>Script 驱动</span>
          </label>
        </div>
      </div>

      {formData.driverType === 'llm' && (
        <div className="space-y-4 p-4 bg-gray-900 rounded-lg border border-gray-700">
          <h4 className="font-medium text-primary-400">LLM 配置</h4>

          <div>
            <label className="block text-sm font-medium text-gray-400 mb-1">API 格式</label>
            <select
              value={formData.config.apiFormat || 'openai'}
              onChange={(e) => updateConfig('apiFormat', e.target.value)}
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none"
            >
              <option value="openai">OpenAI Chat Completions</option>
              <option value="anthropic">Anthropic Claude Messages</option>
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-400 mb-1">API 端点</label>
            <input
              type="text"
              value={formData.config.endpoint || ''}
              onChange={(e) => updateConfig('endpoint', e.target.value)}
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none"
              placeholder={
                formData.config.apiFormat === 'anthropic'
                  ? 'https://api.anthropic.com/v1/messages'
                  : 'https://api.openai.com/v1/chat/completions'
              }
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-400 mb-1">API Key</label>
            <input
              type="password"
              value={formData.config.apiKey || ''}
              onChange={(e) => updateConfig('apiKey', e.target.value)}
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none"
              placeholder="sk-..."
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-400 mb-1">模型名称</label>
            <input
              type="text"
              value={formData.config.model || ''}
              onChange={(e) => updateConfig('model', e.target.value)}
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none"
              placeholder={
                formData.config.apiFormat === 'anthropic'
                  ? 'claude-3-haiku-20240307'
                  : 'gpt-3.5-turbo'
              }
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-400 mb-1">温度</label>
              <input
                type="number"
                step="0.1"
                min="0"
                max="2"
                value={formData.config.temperature ?? 0.7}
                onChange={(e) => updateConfig('temperature', parseFloat(e.target.value))}
                className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-400 mb-1">最大 Token</label>
              <input
                type="number"
                min="1"
                max="100000"
                value={formData.config.maxTokens ?? 4096}
                onChange={(e) => updateConfig('maxTokens', parseInt(e.target.value))}
                className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none"
              />
            </div>
          </div>

          {/* Claude CLI 设置 (仅 cc 格式显示) */}
          {formData.config.apiFormat === 'anthropic' && (
            <div className="pt-4 border-t border-gray-700">
              <h4 className="font-medium text-green-400 mb-3">本地 Claude CLI 设置</h4>
              <div className="space-y-3">
                <div>
                  <label className="block text-sm font-medium text-gray-400 mb-1">Launcher</label>
                  <select
                    value={formData.config.claudeLauncher || ''}
                    onChange={(e) => updateConfig('claudeLauncher', e.target.value)}
                    className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 text-sm focus:border-green-500 focus:outline-none"
                  >
                    <option value="">自动检测</option>
                    <option value="claude-haha">claude-haha (默认)</option>
                    <option value="claude-haha-dsv4">claude-haha-dsv4</option>
                    <option value="claude-haha-minimax27">claude-haha-minimax27</option>
                    <option value="claude-haha-glm51">claude-haha-glm51</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-400 mb-1">项目目录</label>
                  <input
                    type="text"
                    value={formData.config.claudeProjectDir || ''}
                    onChange={(e) => updateConfig('claudeProjectDir', e.target.value)}
                    className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 text-sm focus:border-green-500 focus:outline-none"
                    placeholder="例如: ../claude-code-main"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-400 mb-1">环境文件</label>
                  <input
                    type="text"
                    value={formData.config.claudeEnvFile || '.env.dsv4'}
                    onChange={(e) => updateConfig('claudeEnvFile', e.target.value)}
                    className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 text-sm focus:border-green-500 focus:outline-none"
                    placeholder=".env.dsv4"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-400 mb-1">模型名</label>
                  <input
                    type="text"
                    value={formData.config.claudeModel || 'claude-sonnet-4-6'}
                    onChange={(e) => updateConfig('claudeModel', e.target.value)}
                    className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 text-sm focus:border-green-500 focus:outline-none"
                    placeholder="claude-sonnet-4-6"
                  />
                </div>
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    id="thinkingEnabled"
                    checked={formData.config.thinkingEnabled || false}
                    onChange={(e) => updateConfig('thinkingEnabled', e.target.checked)}
                    className="w-4 h-4 rounded border-gray-600 bg-gray-800 text-green-500 focus:ring-green-500"
                  />
                  <label htmlFor="thinkingEnabled" className="text-sm text-gray-400 cursor-pointer">
                    启用思考模型 (Thinking)
                  </label>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {formData.driverType === 'script' && (
        <div className="space-y-4 p-4 bg-gray-900 rounded-lg border border-gray-700">
          <h4 className="font-medium text-primary-400">Script 配置</h4>

          <div>
            <label className="block text-sm font-medium text-gray-400 mb-1">脚本路径</label>
            <input
              type="text"
              value={formData.config.scriptPath || ''}
              onChange={(e) => updateConfig('scriptPath', e.target.value)}
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none"
              placeholder="/path/to/script.js"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-400 mb-1">脚本类型</label>
            <select
              value={formData.config.scriptType || 'node'}
              onChange={(e) => updateConfig('scriptType', e.target.value)}
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none"
            >
              <option value="node">Node.js</option>
              <option value="python">Python</option>
              <option value="bash">Bash</option>
            </select>
          </div>
        </div>
      )}

      <div className="flex gap-3 pt-4">
        <button
          type="submit"
          disabled={isSubmitting}
          className="flex-1 px-4 py-2 bg-primary-600 hover:bg-primary-700 disabled:bg-gray-600 rounded-lg transition-colors"
        >
          {isSubmitting ? (mode === 'edit' ? '更新中...' : '创建中...') : (mode === 'edit' ? '保存修改' : '创建智能体')}
        </button>
        <button
          type="button"
          onClick={onClose}
          className="px-4 py-2 bg-gray-700 hover:bg-gray-600 rounded-lg transition-colors"
        >
          取消
        </button>
      </div>
    </form>
  );
}
