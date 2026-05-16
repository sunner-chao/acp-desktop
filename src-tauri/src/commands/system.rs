use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn reset_database(state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_connection()
        .execute("DELETE FROM messages", [])
        .map_err(|e| e.to_string())?;
    db.get_connection()
        .execute("DELETE FROM agents", [])
        .map_err(|e| e.to_string())?;
    let _ = db.get_connection().execute("DELETE FROM conversations", []);
    log::info!("Database reset: all agents and messages cleared");
    Ok(())
}
