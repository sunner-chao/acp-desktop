// 导入智能体模版到 ACP Desktop 数据库
import Database from 'better-sqlite3';
import { readFileSync, mkdirSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { homedir } from 'os';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const templatesPath = join(__dirname, '..', 'templates', 'agents.json');
const dbDir = join(homedir(), 'AppData', 'Roaming', 'com.acp.desktop');
const dbPath = join(dbDir, 'acp_desktop.db');

// 确保目录存在
if (!existsSync(dbDir)) {
  mkdirSync(dbDir, { recursive: true });
}

const agents = JSON.parse(readFileSync(templatesPath, 'utf-8'));
const db = new Database(dbPath);

// 初始化表结构
db.exec(`
  CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    description TEXT,
    driver_type TEXT NOT NULL,
    address TEXT UNIQUE NOT NULL,
    config TEXT NOT NULL,
    is_online INTEGER DEFAULT 0,
    last_active TEXT,
    created_at TEXT NOT NULL
  );

  CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    performative TEXT NOT NULL,
    sender TEXT NOT NULL,
    receiver TEXT NOT NULL,
    content TEXT NOT NULL,
    conversation_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    metadata TEXT,
    created_at TEXT NOT NULL
  );

  CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);
  CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender);
  CREATE INDEX IF NOT EXISTS idx_messages_receiver ON messages(receiver);
`);

// 导入智能体
const stmt = db.prepare(`
  INSERT OR REPLACE INTO agents (id, name, description, driver_type, address, config, is_online, last_active, created_at)
  VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
`);

const insertAll = db.transaction((agents) => {
  for (const agent of agents) {
    stmt.run(
      agent.id,
      agent.name,
      agent.description || null,
      agent.driverType,
      agent.address,
      JSON.stringify(agent.config),
      agent.isOnline ? 1 : 0,
      agent.lastActive || null,
      agent.createdAt || new Date().toISOString()
    );
    console.log(`  ✓ ${agent.name} (${agent.driverType}/${agent.config.apiFormat || 'script'})`);
  }
});

console.log('导入智能体模版:');
insertAll(agents);
console.log(`\n已导入 ${agents.length} 个智能体到: ${dbPath}`);
db.close();
