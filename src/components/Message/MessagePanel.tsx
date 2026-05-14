import { useState } from 'react';
import { useAgentStore, useMessageStore, useUIStore } from '../../stores';
import type { ACPPerformative, ACPContent } from '../../types';

export default function MessagePanel() {
  const { agents } = useAgentStore();
  const { sendMessage, isLoading } = useMessageStore();
  const { protocolDrawerCollapsed, toggleProtocolDrawer } = useUIStore();

  const [sender, setSender] = useState('');
  const [receiver, setReceiver] = useState('');
  const [performative, setPerformative] = useState<ACPPerformative>('inform');
  const [contentText, setContentText] = useState('');
  const [action, setAction] = useState('');

  const handleSend = async () => {
    if (!sender || !receiver) {
      alert('请选择发送者和接收者');
      return;
    }

    const content: ACPContent = {};

    if (contentText) {
      content.parameters = { text: contentText };
    }
    if (action) {
      content.action = action;
    }

    try {
      await sendMessage({
        sender: `agent://local/${sender}`,
        receiver: `agent://local/${receiver}`,
        performative,
        content,
      });
      setContentText('');
      setAction('');
      alert('消息发送成功！');
    } catch (error) {
      alert(`发送失败: ${error}`);
    }
  };

  const handleReset = () => {
    setSender('');
    setReceiver('');
    setPerformative('inform');
    setContentText('');
    setAction('');
  };

  return (
    <div className="flex gap-4 h-full">
      <div className="flex-1 bg-gray-800 rounded-lg border border-gray-700 flex flex-col">
        <div className="p-4 border-b border-gray-700 flex items-center justify-between">
          <h3 className="font-semibold">发送 ACP 消息</h3>
          <button
            onClick={handleReset}
            className="text-xs px-3 py-1 rounded bg-gray-700 hover:bg-gray-600"
          >
            重置表单
          </button>
        </div>

        <div className="flex-1 overflow-auto p-4 space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-400 mb-2">发送者</label>
              <select
                value={sender}
                onChange={(e) => setSender(e.target.value)}
                className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none"
              >
                <option value="">选择发送者</option>
                {agents.map((agent) => (
                  <option key={agent.id} value={agent.name}>
                    {agent.name} ({agent.isOnline ? '在线' : '离线'})
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-400 mb-2">接收者</label>
              <select
                value={receiver}
                onChange={(e) => setReceiver(e.target.value)}
                className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none"
              >
                <option value="">选择接收者</option>
                {agents.map((agent) => (
                  <option key={agent.id} value={agent.name}>
                    {agent.name} ({agent.isOnline ? '在线' : '离线'})
                  </option>
                ))}
              </select>
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-400 mb-2">消息类型 (Performative)</label>
            <div className="grid grid-cols-5 gap-2">
              {(['request', 'inform', 'query', 'agree', 'refuse'] as ACPPerformative[]).map((p) => (
                <button
                  key={p}
                  onClick={() => setPerformative(p)}
                  className={`px-3 py-2 rounded-lg text-sm transition-colors ${
                    performative === p
                      ? p === 'request' ? 'bg-blue-600 text-white' :
                        p === 'inform' ? 'bg-green-600 text-white' :
                        p === 'query' ? 'bg-yellow-600 text-white' :
                        p === 'agree' ? 'bg-purple-600 text-white' :
                        'bg-red-600 text-white'
                      : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
                  }`}
                >
                  {p}
                </button>
              ))}
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-400 mb-2">动作 (可选)</label>
            <input
              type="text"
              value={action}
              onChange={(e) => setAction(e.target.value)}
              className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none"
              placeholder="例如: get_status, query_info"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-400 mb-2">消息内容</label>
            <textarea
              value={contentText}
              onChange={(e) => setContentText(e.target.value)}
              className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-gray-100 focus:border-primary-500 focus:outline-none h-32"
              placeholder="输入消息内容..."
            />
          </div>

          <button
            onClick={handleSend}
            disabled={isLoading || !sender || !receiver}
            className="w-full px-4 py-3 bg-primary-600 hover:bg-primary-700 disabled:bg-gray-600 rounded-lg transition-colors"
          >
            {isLoading ? '发送中...' : '发送消息'}
          </button>
        </div>
      </div>

      <div
        className={`bg-gray-800 rounded-lg border border-gray-700 flex flex-col transition-all ${
          protocolDrawerCollapsed ? 'w-14' : 'w-80'
        }`}
      >
        <div className="p-4 border-b border-gray-700 flex items-center justify-between">
          {!protocolDrawerCollapsed ? <h3 className="font-semibold">ACP 协议说明</h3> : <div className="text-xs text-gray-500">P</div>}
          <button
            onClick={toggleProtocolDrawer}
            className="h-7 w-7 rounded-md hover:bg-gray-700 text-gray-400 hover:text-gray-200"
            title={protocolDrawerCollapsed ? '展开' : '折叠'}
          >
            {protocolDrawerCollapsed ? '‹' : '›'}
          </button>
        </div>

        <div className="flex-1 overflow-auto p-4 text-sm text-gray-400 space-y-3">
          {protocolDrawerCollapsed ? (
            <div className="text-xs text-gray-500 text-center py-4">ACP</div>
          ) : (
            <>
              <div>
                <div className="font-medium text-gray-300 mb-1">request</div>
                <div>请求某个智能体执行操作</div>
              </div>
              <div>
                <div className="font-medium text-gray-300 mb-1">inform</div>
                <div>向其他智能体通知信息</div>
              </div>
              <div>
                <div className="font-medium text-gray-300 mb-1">query</div>
                <div>查询某个智能体的状态</div>
              </div>
              <div>
                <div className="font-medium text-gray-300 mb-1">agree</div>
                <div>同意执行某个请求</div>
              </div>
              <div>
                <div className="font-medium text-gray-300 mb-1">refuse</div>
                <div>拒绝执行某个请求</div>
              </div>

              <div className="pt-4 border-t border-gray-700">
                <div className="font-medium text-gray-300 mb-1">地址格式</div>
                <div className="font-mono text-xs">agent://local/{'{name}'}</div>
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
