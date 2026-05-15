use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ACPPerformative {
    Request,
    Inform,
    Query,
    Agree,
    Refuse,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ACPContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ACPMessage {
    pub id: String,
    pub performative: ACPPerformative,
    pub sender: String,
    pub receiver: String,
    pub content: ACPContent,
    pub conversation_id: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageInput {
    pub sender: String,
    pub receiver: String,
    pub performative: ACPPerformative,
    pub content: ACPContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MessageFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performative: Option<ACPPerformative>,
}

impl ACPMessage {
    pub fn new(input: SendMessageInput) -> Self {
        let id = Uuid::new_v4().to_string();
        let conversation_id = input.conversation_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = chrono::Utc::now().to_rfc3339();

        Self {
            id,
            performative: input.performative,
            sender: input.sender,
            receiver: input.receiver,
            content: input.content,
            conversation_id,
            timestamp,
            metadata: None,
        }
    }
}
