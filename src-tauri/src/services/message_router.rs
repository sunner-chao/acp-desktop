use crate::models::{ACPMessage, MessageFilter, SendMessageInput};
use crate::services::db::Database;
use rusqlite::{params, Result};
use std::sync::Mutex;

pub struct MessageRouter {
    db: Mutex<Database>,
}

impl MessageRouter {
    pub fn new(db: Database) -> Self {
        Self { db: Mutex::new(db) }
    }

    pub fn send_message(&self, input: SendMessageInput) -> Result<ACPMessage> {
        let db = self.db.lock().unwrap();
        let message = ACPMessage::new(input);

        db.get_connection().execute(
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
        )?;

        log::info!(
            "Message sent: {} -> {} ({:?})",
            message.sender,
            message.receiver,
            message.performative
        );
        Ok(message)
    }

    pub fn get_messages(&self, filter: Option<MessageFilter>) -> Result<Vec<ACPMessage>> {
        let db = self.db.lock().unwrap();

        let sql = "SELECT id, performative, sender, receiver, content, conversation_id, timestamp, metadata, created_at FROM messages";
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

        let mut stmt = db.get_connection().prepare(&query)?;

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let messages = stmt.query_map(params_refs.as_slice(), |row| {
            let performative_str: String = row.get(1)?;
            let content_str: String = row.get(4)?;
            let metadata_str: Option<String> = row.get(7)?;

            let performative = serde_json::from_str(&performative_str).unwrap();
            let content = serde_json::from_str(&content_str).unwrap();
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
        })?;

        messages.collect()
    }

    pub fn get_conversations(&self) -> Result<Vec<String>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db
            .get_connection()
            .prepare("SELECT DISTINCT conversation_id FROM messages ORDER BY timestamp DESC")?;

        let conversations = stmt.query_map([], |row| row.get(0))?;
        conversations.collect()
    }
}
