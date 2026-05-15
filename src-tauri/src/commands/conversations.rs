use crate::models::{Conversation, CreateConversationInput, UpdateConversationInput};
use crate::AppState;
use rusqlite::params;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn create_conversation(
    state: State<AppState>,
    input: CreateConversationInput,
) -> Result<Conversation, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let agent_ids_json = serde_json::to_string(&input.selected_agent_ids).unwrap();

    db.get_connection()
        .execute(
            "INSERT INTO conversations (id, title, selected_agent_ids, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, input.title, agent_ids_json, now, now],
        )
        .map_err(|e| e.to_string())?;

    log::info!("Created conversation: {} ({})", input.title, id);
    Ok(Conversation {
        id,
        title: input.title,
        selected_agent_ids: input.selected_agent_ids,
        created_at: now.clone(),
        updated_at: now,
    })
}

#[tauri::command]
pub fn list_conversations(state: State<AppState>) -> Result<Vec<Conversation>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .get_connection()
        .prepare(
            "SELECT id, title, selected_agent_ids, created_at, updated_at FROM conversations ORDER BY updated_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let conversations = stmt
        .query_map([], |row| {
            let agent_ids_str: String = row.get(2)?;
            let selected_agent_ids: Vec<String> =
                serde_json::from_str(&agent_ids_str).unwrap_or_default();
            Ok(Conversation {
                id: row.get(0)?,
                title: row.get(1)?,
                selected_agent_ids,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(conversations)
}

#[tauri::command]
pub fn delete_conversation(state: State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_connection()
        .execute("DELETE FROM messages WHERE conversation_id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    db.get_connection()
        .execute("DELETE FROM conversations WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    log::info!("Deleted conversation: {}", id);
    Ok(())
}

#[tauri::command]
pub fn update_conversation(
    state: State<AppState>,
    input: UpdateConversationInput,
) -> Result<Conversation, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();

    let mut stmt = db
        .get_connection()
        .prepare(
            "SELECT id, title, selected_agent_ids, created_at FROM conversations WHERE id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let (current_title, current_agent_ids, created_at): (String, Vec<String>, String) = stmt
        .query_row([&input.id], |row| {
            let agent_ids_str: String = row.get(2)?;
            let selected_agent_ids: Vec<String> =
                serde_json::from_str(&agent_ids_str).unwrap_or_default();
            Ok((row.get(1)?, selected_agent_ids, row.get(3)?))
        })
        .map_err(|e| format!("Conversation not found: {}", e))?;

    let new_title = input.title.unwrap_or(current_title);
    let new_agent_ids = input.selected_agent_ids.unwrap_or(current_agent_ids);
    let new_agent_ids_json = serde_json::to_string(&new_agent_ids).unwrap();

    db.get_connection()
        .execute(
            "UPDATE conversations SET title = ?1, selected_agent_ids = ?2, updated_at = ?3 WHERE id = ?4",
            params![new_title, new_agent_ids_json, now, input.id],
        )
        .map_err(|e| e.to_string())?;

    Ok(Conversation {
        id: input.id,
        title: new_title,
        selected_agent_ids: new_agent_ids,
        created_at,
        updated_at: now,
    })
}
