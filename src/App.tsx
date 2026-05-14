import { useEffect } from 'react';
import { useAgentStore, useMessageStore, useUIStore } from './stores';
import Layout from './components/Layout/Layout';
import Dashboard from './components/Dashboard/Dashboard';
import AgentList from './components/Agent/AgentList';
import AgentEditor from './components/Agent/AgentEditor';
import MessagePanel from './components/Message/MessagePanel';
import MessageHistory from './components/Message/MessageHistory';
import ChatWindow from './components/Message/ChatWindow';

function App() {
  const { fetchAgents } = useAgentStore();
  const { fetchConversations } = useMessageStore();
  const { currentView } = useUIStore();

  useEffect(() => {
    fetchAgents();
    fetchConversations();
  }, [fetchAgents, fetchConversations]);

  const renderView = () => {
    switch (currentView) {
      case 'agents':
        return (
          <div className="flex gap-4 h-full">
            <div className="w-80 flex-shrink-0">
              <AgentList />
            </div>
            <div className="flex-1">
              <AgentEditor />
            </div>
          </div>
        );
      case 'messages':
        return <MessagePanel />;
      case 'history':
        return <MessageHistory />;
      case 'chat':
        return <ChatWindow />;
      default:
        return <Dashboard />;
    }
  };

  return (
    <Layout>
      {renderView()}
    </Layout>
  );
}

export default App;
