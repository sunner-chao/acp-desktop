import type { ACPMessage } from './message';

export interface ToolCallInfo {
  name: string;
  input: string;
}

export interface ChatMessage {
  id: string;
  role: 'user' | 'agent' | 'system';
  sender: string;
  text: string;
  timestamp: string;
  round?: number;
  isLoading?: boolean;
  phase?: string | null;
  toolCalls?: ToolCallInfo[] | null;
  acpMessage?: ACPMessage;
}

export interface ConversationSnapshot {
  selectedAgentIds: string[];
  inputText: string;
  rounds: number;
  autoContinue: boolean;
}
