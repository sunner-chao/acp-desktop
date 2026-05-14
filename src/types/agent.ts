export type DriverType = 'script' | 'llm';

export type ApiFormat = 'openai' | 'anthropic';

export interface Agent {
  id: string;
  name: string;
  description?: string;
  driverType: DriverType;
  address: string;
  config: AgentConfig;
  isOnline: boolean;
  sessionPid?: number;
  sessionId?: string;
  lastActive?: string;
  createdAt: string;
}

export interface AgentConfig {
  apiFormat?: ApiFormat;
  endpoint?: string;
  apiKey?: string;
  model?: string;
  temperature?: number;
  maxTokens?: number;
  scriptPath?: string;
  scriptType?: 'node' | 'python' | 'bash';
  // Claude CLI 设置（仅 cc 格式）
  claudeProjectDir?: string;
  claudeEnvFile?: string;
  claudeModel?: string;
}

export interface ClaudeSettings {
  command: string;
  projectDir: string;
  envFile: string;
  entrypoint: string;
  defaultModel: string;
  timeoutMs: number;
}

export interface CreateAgentInput {
  name: string;
  description?: string;
  driverType: DriverType;
  config: AgentConfig;
}

export interface UpdateAgentInput extends Partial<CreateAgentInput> {
  id: string;
}
