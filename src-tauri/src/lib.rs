mod commands;
mod models;
mod services;

use commands::{agents, claude, conversations, messages, system};
use services::db::Database;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use tauri::Manager;

#[derive(Debug, Clone)]
pub struct AgentSessionInfo {
    pub pid: u32,
    pub started_at: String,
}

pub struct AppState {
    pub db: Mutex<Database>,
    pub sessions: Mutex<HashMap<String, AgentSessionInfo>>,
    pub active_chat_processes: Mutex<HashMap<String, u32>>,
    pub cancelled_chat_requests: Mutex<HashSet<String>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 先占位，实际 log 在 setup 中初始化（因为需要 app_data_dir 路径）
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_dir).expect("Failed to create app data dir");

            // 多实例支持：通过 ACP_INSTANCE 环境变量区分数据库
            let instance = std::env::var("ACP_INSTANCE").unwrap_or_default();
            let suffix = if instance.is_empty() {
                String::new()
            } else {
                format!("_{}", instance)
            };

            // 初始化 file-based logging
            let log_path = app_dir.join(format!("acp_desktop{}.log", suffix));
            let log_file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .expect("Failed to open log file");
            env_logger::Builder::from_default_env()
                .target(env_logger::Target::Pipe(Box::new(log_file)))
                .filter_level(log::LevelFilter::Info)
                .init();
            log::info!("Starting ACP Desktop Agent Hub, log at {:?}", log_path);

            let db_path = app_dir.join(format!("acp_desktop{}.db", suffix));
            let db = Database::new(&db_path).expect("Failed to initialize database");

            app.manage(AppState {
                db: Mutex::new(db),
                sessions: Mutex::new(HashMap::new()),
                active_chat_processes: Mutex::new(HashMap::new()),
                cancelled_chat_requests: Mutex::new(HashSet::new()),
            });
            log::info!("Database initialized at {:?}", db_path);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            agents::get_agents,
            agents::create_agent,
            agents::update_agent,
            agents::delete_agent,
            agents::start_agent_session,
            agents::stop_agent_session,
            agents::import_agents,
            agents::export_agents,
            messages::send_message,
            messages::get_messages,
            messages::get_conversations,
            messages::clear_messages,
            messages::clear_conversation_messages,
            claude::invoke_claude_agent,
            claude::invoke_agent_group_chat,
            claude::invoke_agent_group_chat_stream,
            claude::stop_agent_group_chat,
            claude::get_claude_settings,
            claude::check_claude_cli,
            conversations::create_conversation,
            conversations::list_conversations,
            conversations::delete_conversation,
            conversations::update_conversation,
            system::reset_database,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
