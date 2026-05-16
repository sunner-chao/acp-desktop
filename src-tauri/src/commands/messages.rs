use crate::models::{ACPContent, ACPMessage, ACPPerformative, MessageFilter, SendMessageInput};
use crate::AppState;
use rusqlite::params;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn send_message(state: State<AppState>, input: SendMessageInput) -> Result<ACPMessage, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let id = Uuid::new_v4().to_string();
    let conversation_id = input
        .conversation_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let timestamp = chrono::Utc::now().to_rfc3339();

    let message = ACPMessage {
        id: id.clone(),
        performative: input.performative.clone(),
        sender: input.sender.clone(),
        receiver: input.receiver.clone(),
        content: input.content.clone(),
        conversation_id: conversation_id.clone(),
        timestamp: timestamp.clone(),
        metadata: None,
    };

    db.get_connection()
        .execute(
            "INSERT INTO messages (id, performative, sender, receiver, content, conversation_id, timestamp, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                message.id,
                serde_json::to_string(&message.performative).unwrap(),
                message.sender,
                message.receiver,
                serde_json::to_string(&message.content).unwrap(),
                message.conversation_id,
                message.timestamp,
                message.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap()),
                chrono::Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| e.to_string())?;

    log::info!(
        "Message sent: {} -> {} ({:?})",
        message.sender,
        message.receiver,
        message.performative
    );
    Ok(message)
}

#[tauri::command]
pub fn get_messages(
    state: State<AppState>,
    filter: Option<MessageFilter>,
) -> Result<Vec<ACPMessage>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let sql = "SELECT id, performative, sender, receiver, content, conversation_id, timestamp, metadata, created_at FROM messages WHERE 1=1";
    let mut conditions = Vec::new();
    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref f) = filter {
        if let Some(ref conv_id) = f.conversation_id {
            conditions.push("conversation_id = ?".to_string());
            params_vec.push(Box::new(conv_id.clone()));
        }
        if let Some(ref sender) = f.sender {
            conditions.push("sender = ?".to_string());
            params_vec.push(Box::new(sender.clone()));
        }
        if let Some(ref receiver) = f.receiver {
            conditions.push("receiver = ?".to_string());
            params_vec.push(Box::new(receiver.clone()));
        }
        if let Some(ref perf) = f.performative {
            conditions.push("performative = ?".to_string());
            params_vec.push(Box::new(serde_json::to_string(perf).unwrap()));
        }
    }

    let query = if conditions.is_empty() {
        sql.to_string()
    } else {
        format!("{} WHERE {}", sql, conditions.join(" AND "))
    };

    let mut stmt = db
        .get_connection()
        .prepare(&query)
        .map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let messages = stmt
        .query_map(params_refs.as_slice(), |row| {
            let performative_str: String = row.get(1)?;
            let content_str: String = row.get(4)?;
            let metadata_str: Option<String> = row.get(7)?;

            let performative: ACPPerformative =
                serde_json::from_str(&performative_str).unwrap_or(ACPPerformative::Inform);
            let content: ACPContent = serde_json::from_str(&content_str).unwrap_or_default();
            let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());

            Ok(ACPMessage {
                id: row.get(0)?,
                performative,
                sender: row.get(2)?,
                receiver: row.get(3)?,
                content,
                conversation_id: row.get(5)?,
                timestamp: row.get(6)?,
                metadata,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(messages)
}

#[tauri::command]
pub fn get_conversations(state: State<AppState>) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .get_connection()
        .prepare("SELECT DISTINCT conversation_id FROM messages ORDER BY timestamp DESC")
        .map_err(|e| e.to_string())?;

    let conversations = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(conversations)
}

#[tauri::command]
pub fn clear_messages(state: State<AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_connection()
        .execute("DELETE FROM messages", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn clear_conversation_messages(
    state: State<AppState>,
    conversation_id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_connection()
        .execute(
            "DELETE FROM messages WHERE conversation_id = ?1",
            [conversation_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}
