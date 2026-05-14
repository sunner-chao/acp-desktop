import { useUIStore } from '../../stores';

export default function Header() {
  const { toggleSidebar } = useUIStore();

  return (
    <header className="h-14 bg-gray-800 border-b border-gray-700 flex items-center justify-between px-4">
      <div className="flex items-center gap-4">
        <button
          onClick={toggleSidebar}
          className="p-2 rounded-lg hover:bg-gray-700 text-gray-400 hover:text-gray-200"
        >
          ☰
        </button>
        <span className="text-sm text-gray-400">ACP 多智能体通信桌面端应用</span>
      </div>

      <div className="flex items-center gap-4">
        <span className="text-xs text-gray-500">多智能体通信协议 (ACP)</span>
      </div>
    </header>
  );
}