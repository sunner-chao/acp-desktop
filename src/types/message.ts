export type ACPPerformative = 'request' | 'inform' | 'query' | 'agree' | 'refuse';

export interface ACPContent {
  action?: string;
  parameters?: Record<string, unknown>;
  result?: unknown;
  reason?: string;
}

export interface ACPMessage {
  id: string;
  performative: ACPPerformative;
  sender: string;
  receiver: string;
  content: ACPContent;
  conversationId: string;
  timestamp: string;
  metadata?: Record<string, unknown>;
}

export interface SendMessageInput {
  sender: string;
  receiver: string;
  performative: ACPPerformative;
  content: ACPContent;
}

export interface MessageFilter {
  conversationId?: string;
  sender?: string;
  receiver?: string;
  performative?: ACPPerformative;
}
