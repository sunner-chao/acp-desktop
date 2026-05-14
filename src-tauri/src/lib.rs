mod commands;
mod models;
mod services;

use commands::{agents, claude, messages};
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
    env_logger::init();
    log::info!("Starting ACP Desktop Agent Hub");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");
            std::fs::create_dir_all(&app_dir).expect("Failed to create app data dir");

            let db_path = app_dir.join("acp_desktop.db");
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
