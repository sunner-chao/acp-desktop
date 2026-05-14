import { useAgentStore, useUIStore } from '../../stores';
import AgentForm from './AgentForm';
import LLMSettings from './LLMSettings';
import type { Agent } from '../../types';

export default function AgentEditor() {
  const { agents, selectedAgentId } = useAgentStore();
  const { modalOpen, modalData, closeModal } = useUIStore();

  const selectedAgent = agents.find((a) => a.id === selectedAgentId);
  const editingAgent = modalData as Agent | undefined;

  if (modalOpen === 'create-agent') {
    return (
      <div className="bg-gray-800 rounded-lg border border-gray-700 h-full">
        <div className="p-4 border-b border-gray-700 flex items-center justify-between">
          <h3 className="font-semibold">创建新智能体</h3>
          <button
            onClick={closeModal}
            className="text-gray-400 hover:text-gray-200"
          >
            ✕
          </button>
        </div>
        <div className="p-4">
          <AgentForm onClose={closeModal} />
        </div>
      </div>
    );
  }

  if (modalOpen === 'edit-agent' && editingAgent) {
    return (
      <div className="bg-gray-800 rounded-lg border border-gray-700 h-full">
        <div className="p-4 border-b border-gray-700 flex items-center justify-between">
          <h3 className="font-semibold">编辑智能体</h3>
          <button
            onClick={closeModal}
            className="text-gray-400 hover:text-gray-200"
          >
            ✕
          </button>
        </div>
        <div className="p-4">
          <AgentForm
            onClose={closeModal}
            editId={editingAgent.id}
            initialValues={{
              name: editingAgent.name,
              description: editingAgent.description,
              driverType: editingAgent.driverType,
              config: editingAgent.config,
            }}
            mode="edit"
          />
        </div>
      </div>
    );
  }

  if (modalOpen === 'copy-agent' && editingAgent) {
    return (
      <div className="bg-gray-800 rounded-lg border border-gray-700 h-full">
        <div className="p-4 border-b border-gray-700 flex items-center justify-between">
          <h3 className="font-semibold">复制智能体</h3>
          <button
            onClick={closeModal}
            className="text-gray-400 hover:text-gray-200"
          >
            ✕
          </button>
        </div>
        <div className="p-4">
          <AgentForm
            onClose={closeModal}
            initialValues={{
              name: `${editingAgent.name} (副本)`,
              description: editingAgent.description,
              driverType: editingAgent.driverType,
              config: editingAgent.config,
            }}
            mode="copy"
          />
        </div>
      </div>
    );
  }

  if (!selectedAgent) {
    return (
      <div className="bg-gray-800 rounded-lg border border-gray-700 h-full flex items-center justify-center">
        <div className="text-center text-gray-500">
          <div className="text-4xl mb-4">📝</div>
          <div>选择一个智能体进行编辑</div>
          <div className="text-sm mt-2">或创建新的智能体</div>
        </div>
      </div>
    );
  }

  return (
    <div className="bg-gray-800 rounded-lg border border-gray-700 h-full flex flex-col">
      <div className="p-4 border-b border-gray-700">
        <h3 className="font-semibold">编辑智能体: {selectedAgent.name}</h3>
      </div>

      <div className="flex-1 overflow-auto p-4 space-y-6">
        <div>
          <label className="block text-sm font-medium text-gray-400 mb-2">名称</label>
          <div className="text-lg">{selectedAgent.name}</div>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-400 mb-2">地址</label>
          <div className="text-lg font-mono bg-gray-900 p-2 rounded">{selectedAgent.address}</div>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-400 mb-2">描述</label>
          <div className="text-gray-300">{selectedAgent.description || '无描述'}</div>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-400 mb-2">驱动类型</label>
          <span className={`px-2 py-1 rounded text-sm ${
            selectedAgent.driverType === 'llm' ? 'bg-primary-900 text-primary-300' : 'bg-gray-700 text-gray-300'
          }`}>
            {selectedAgent.driverType === 'llm' ? 'LLM 驱动' : 'Script 驱动'}
          </span>
        </div>

        {selectedAgent.driverType === 'llm' && (
          <div>
            <label className="block text-sm font-medium text-gray-400 mb-2">LLM 配置</label>
            <LLMSettings agent={selectedAgent} />
          </div>
        )}

        <div>
          <label className="block text-sm font-medium text-gray-400 mb-2">状态</label>
          <div className="flex items-center gap-2">
            <div className={`w-3 h-3 rounded-full ${selectedAgent.isOnline ? 'bg-green-400' : 'bg-gray-500'}`} />
            <span>{selectedAgent.isOnline ? '在线' : '离线'}</span>
          </div>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-400 mb-2">创建时间</label>
          <div className="text-gray-300">{new Date(selectedAgent.createdAt).toLocaleString()}</div>
        </div>
      </div>
    </div>
  );
}