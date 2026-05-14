import { useUIStore } from '../../stores';

const navItems = [
  { id: 'dashboard', label: '驾驶舱', icon: '📊' },
  { id: 'chat', label: '聊天', icon: '💬' },
  { id: 'agents', label: '智能体', icon: '🤖' },
  { id: 'messages', label: 'ACP消息', icon: '📨' },
  { id: 'history', label: '历史', icon: '📜' },
] as const;

export default function Sidebar() {
  const { currentView, setCurrentView, sidebarCollapsed } = useUIStore();

  return (
    <aside
      className={`bg-gray-800 border-r border-gray-700 transition-all duration-300 ${
        sidebarCollapsed ? 'w-16' : 'w-56'
      }`}
    >
      <div className="flex flex-col h-full">
        <div className="p-4 border-b border-gray-700">
          <h1 className={`font-bold text-lg text-primary-400 ${sidebarCollapsed ? 'text-center' : ''}`}>
            {sidebarCollapsed ? 'ACP' : 'ACP Desktop'}
          </h1>
        </div>

        <nav className="flex-1 p-2">
          {navItems.map((item) => (
            <button
              key={item.id}
              onClick={() => setCurrentView(item.id)}
              className={`w-full flex items-center gap-3 px-4 py-3 rounded-lg mb-1 transition-colors ${
                currentView === item.id
                  ? 'bg-primary-600 text-white'
                  : 'text-gray-400 hover:bg-gray-700 hover:text-gray-200'
              }`}
            >
              <span className="text-xl">{item.icon}</span>
              {!sidebarCollapsed && <span>{item.label}</span>}
            </button>
          ))}
        </nav>

        <div className="p-4 border-t border-gray-700 text-xs text-gray-500">
          {sidebarCollapsed ? 'v0.1' : 'ACP Desktop v0.1.0'}
        </div>
      </div>
    </aside>
  );
}