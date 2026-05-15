export interface Conversation {
  id: string;
  title: string;
  selectedAgentIds: string[];
  createdAt: string;
  updatedAt: string;
}

export interface CreateConversationInput {
  title: string;
  selectedAgentIds: string[];
}

export interface UpdateConversationInput {
  id: string;
  title?: string;
  selectedAgentIds?: string[];
}
