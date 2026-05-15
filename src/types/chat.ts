import type { ACPMessage } from './message';

export interface ChatMessage {
  id: string;
  role: 'user' | 'agent' | 'system';
  sender: string;
  text: string;
  timestamp: string;
  round?: number;
  isLoading?: boolean;
  acpMessage?: ACPMessage;
}

export interface ConversationSnapshot {
  selectedAgentIds: string[];
  inputText: string;
  rounds: number;
  autoContinue: boolean;
}
