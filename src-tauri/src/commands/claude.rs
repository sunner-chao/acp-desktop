use crate::commands::agents::set_agent_online_status;
use crate::models::{ACPContent, ACPMessage, ACPPerformative, AgentConfig, ClaudeSettings};
use crate::services::claude_cli::{terminate_session, ClaudeCli, ClaudeCliError};
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokeClaudeInput {
    pub agent_id: String,
    pub agent_name: String,
    pub agent_config: AgentConfig,
    pub agent_description: Option<String>,
    pub sender_address: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeResponse {
    pub agent_name: String,
    pub response_text: String,
    pub conversation_id: String,
    pub messages: Vec<ACPMessage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupChatAgentInput {
    pub id: String,
    pub name: String,
    pub config: AgentConfig,
    pub description: Option<String>,
    pub address: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokeAgentGroupChatInput {
    pub agents: Vec<GroupChatAgentInput>,
    pub message: String,
    pub rounds: Option<u32>,
    pub conversation_id: Option<String>,
    pub request_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupChatResponse {
    pub conversation_id: String,
    pub messages: Vec<ACPMessage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupChatStreamEvent {
    pub request_id: String,
    pub conversation_id: String,
    pub message_id: Option<String>,
    pub agent_id: Option<String>,
    pub speaker: Option<String>,
    pub round: Option<u32>,
    pub chunk: Option<String>,
    pub message: Option<ACPMessage>,
    pub status: String,
}

#[tauri::command]
pub async fn invoke_claude_agent(
    state: State<'_, AppState>,
    input: InvokeClaudeInput,
) -> Result<ClaudeResponse, String> {
    let settings = ClaudeSettings::with_agent_config(Some(&input.agent_config));
    let cli = ClaudeCli::new(settings);

    let system_prompt = input
        .agent_description
        .unwrap_or_else(|| "An AI agent in the ACP network.".to_string());

    // 提前 clone 避免 move
    let agent_name = input.agent_name.clone();
    let sender_addr = input.sender_address.clone();

    log::info!(
        "[invoke_claude_agent] Agent: {}, message from: {}",
        agent_name,
        sender_addr
    );

    if let Ok(db) = state.db.lock() {
        let _ = set_agent_online_status(&db, &input.agent_id, true);
    }

    let (resolved_session_id, resume_existing) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        resolve_agent_session_id(&db, &input.agent_id)?
    };

    // 调用本地 Claude CLI
    let response_text = cli
        .invoke(
            &input.agent_config,
            &input.message,
            &system_prompt,
            Some(&resolved_session_id),
            resume_existing,
        )
        .map_err(|e| e.to_string())?;

    // 生成 conversation_id
    let mut parts = vec![sender_addr.as_str(), agent_name.as_str()];
    parts.sort();
    let conversation_id =
        Uuid::new_v5(&Uuid::NAMESPACE_URL, parts.join(":").as_bytes()).to_string();
    let now = chrono::Utc::now().to_rfc3339();

    // 构建双向 ACP 消息
    let incoming = ACPMessage {
        id: Uuid::new_v4().to_string(),
        performative: ACPPerformative::Request,
        sender: sender_addr.clone(),
        receiver: format!("agent://local/{}", agent_name),
        content: ACPContent {
            action: Some("chat".to_string()),
            parameters: Some(serde_json::json!({"text": input.message})),
            result: None,
            reason: None,
        },
        conversation_id: conversation_id.clone(),
        timestamp: now.clone(),
        metadata: None,
    };

    let response = ACPMessage {
        id: Uuid::new_v4().to_string(),
        performative: ACPPerformative::Inform,
        sender: format!("agent://local/{}", agent_name),
        receiver: sender_addr,
        content: ACPContent {
            action: Some("chat_response".to_string()),
            result: Some(serde_json::json!({"text": response_text})),
            parameters: None,
            reason: None,
        },
        conversation_id: conversation_id.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        metadata: None,
    };

    // 持久化消息
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        persist_agent_session_id(&db, &input.agent_id, &resolved_session_id)?;

        for msg in &[&incoming, &response] {
            let _ = db.get_connection().execute(
                "INSERT OR REPLACE INTO messages (id, performative, sender, receiver, content, conversation_id, timestamp, metadata, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    msg.id,
                    serde_json::to_string(&msg.performative).unwrap(),
                    msg.sender,
                    msg.receiver,
                    serde_json::to_string(&msg.content).unwrap(),
                    msg.conversation_id,
                    msg.timestamp,
                    msg.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap()),
                    chrono::Utc::now().to_rfc3339(),
                ],
            );
        }
        let _ = set_agent_online_status(&db, &input.agent_id, true);
    }

    Ok(ClaudeResponse {
        agent_name: agent_name.clone(),
        response_text,
        conversation_id,
        messages: vec![incoming, response],
    })
}

#[tauri::command]
pub async fn invoke_agent_group_chat(
    state: State<'_, AppState>,
    input: InvokeAgentGroupChatInput,
) -> Result<GroupChatResponse, String> {
    if input.agents.is_empty() {
        return Err("至少选择一个智能体".to_string());
    }

    let rounds = input.rounds.unwrap_or(1).clamp(1, 6);
    let first_agent_config = input.agents.first().map(|a| &a.config);
    let settings = ClaudeSettings::with_agent_config(first_agent_config);
    let cli = ClaudeCli::new(settings);
    let conversation_id = input
        .conversation_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let user_address = "agent://local/user".to_string();
    let group_address = "agent://local/group-chat".to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let history = load_conversation_history(&state, &conversation_id)?;
    let mut emitted = history.clone();
    let persisted_len = history.len();

    let mut session_cache = std::collections::HashMap::<String, (String, bool, usize)>::new();
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        for agent in &input.agents {
            let (session_id, is_new) = resolve_agent_session_id(&db, &agent.id)?;
            session_cache.insert(agent.id.clone(), (session_id, is_new, 0));
            let _ = set_agent_online_status(&db, &agent.id, true);
        }
    }

    let kickoff = ACPMessage {
        id: Uuid::new_v4().to_string(),
        performative: ACPPerformative::Request,
        sender: user_address,
        receiver: group_address.clone(),
        content: ACPContent {
            action: Some("start_group_chat".to_string()),
            parameters: Some(serde_json::json!({"text": input.message})),
            result: None,
            reason: None,
        },
        conversation_id: conversation_id.clone(),
        timestamp: now,
        metadata: Some(serde_json::json!({"round": 0, "speaker": "user"})),
    };
    emitted.push(kickoff);

    for round in 1..=rounds {
        for agent in &input.agents {
            let (session_id, is_new, last_seen) = session_cache
                .get(&agent.id)
                .cloned()
                .ok_or_else(|| format!("{} 缺少会话上下文", agent.name))?;
            let transcript = build_group_transcript(&emitted[last_seen..]);
            let prompt = format!(
                "You are participating in a multi-agent ACP group conversation.\n\nNew ACP messages since your last turn:\n{}\n\nNow respond as {}. Address the other agents when useful, keep the reply focused, and do not invent messages for anyone else.",
                transcript, agent.name
            );
            let system_prompt = agent
                .description
                .clone()
                .unwrap_or_else(|| "An AI agent in the ACP network.".to_string());

            let response_text = cli
                .invoke(
                    &agent.config,
                    &prompt,
                    &system_prompt,
                    Some(session_id.as_str()),
                    !is_new,
                )
                .map_err(|e| format!("{} 调用失败: {}", agent.name, e))?;

            let msg = ACPMessage {
                id: Uuid::new_v4().to_string(),
                performative: ACPPerformative::Inform,
                sender: agent.address.clone(),
                receiver: group_address.clone(),
                content: ACPContent {
                    action: Some("group_chat_turn".to_string()),
                    parameters: None,
                    result: Some(serde_json::json!({"text": response_text})),
                    reason: None,
                },
                conversation_id: conversation_id.clone(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                metadata: Some(serde_json::json!({
                    "round": round,
                    "agentId": agent.id,
                    "speaker": agent.name
                })),
            };
            emitted.push(msg);

            {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                let _ = persist_agent_session_id(&db, &agent.id, &session_id);
            }

            session_cache.insert(agent.id.clone(), (session_id, false, emitted.len()));
        }
    }

    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        for msg in &emitted[persisted_len..] {
            let _ = db.get_connection().execute(
                "INSERT OR REPLACE INTO messages (id, performative, sender, receiver, content, conversation_id, timestamp, metadata, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    msg.id,
                    serde_json::to_string(&msg.performative).unwrap(),
                    msg.sender,
                    msg.receiver,
                    serde_json::to_string(&msg.content).unwrap(),
                    msg.conversation_id,
                    msg.timestamp,
                    msg.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap()),
                    chrono::Utc::now().to_rfc3339(),
                ],
            );
        }
    }

    Ok(GroupChatResponse {
        conversation_id,
        messages: emitted,
    })
}

#[tauri::command]
pub async fn invoke_agent_group_chat_stream(
    app: AppHandle,
    state: State<'_, AppState>,
    input: InvokeAgentGroupChatInput,
) -> Result<GroupChatResponse, String> {
    if input.agents.is_empty() {
        return Err("至少选择一个智能体".to_string());
    }

    let request_id = input
        .request_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    clear_cancelled_request(&state, &request_id);

    let rounds = input.rounds.unwrap_or(1).clamp(1, 6);
    let first_agent_config = input.agents.first().map(|a| &a.config);
    let settings = ClaudeSettings::with_agent_config(first_agent_config);

    log::info!(
        "[invoke_agent_group_chat_stream] request={}, rounds={}, agents={:?}, project_dir={}, env_file={}",
        request_id,
        rounds,
        input.agents.iter().map(|a| format!("{}(id={})", a.name, a.id)).collect::<Vec<_>>(),
        settings.project_dir,
        settings.env_file
    );

    let cli = ClaudeCli::new(settings);
    let conversation_id = input
        .conversation_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let user_address = "agent://local/user".to_string();
    let group_address = "agent://local/group-chat".to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let history = load_conversation_history(&state, &conversation_id)?;
    let mut emitted = history.clone();
    let persisted_len = history.len();

    let mut session_cache = std::collections::HashMap::<String, (String, bool, usize)>::new();
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        for agent in &input.agents {
            let (session_id, is_new) = resolve_agent_session_id(&db, &agent.id)?;
            session_cache.insert(agent.id.clone(), (session_id, is_new, 0));
            let _ = set_agent_online_status(&db, &agent.id, true);
        }
    }

    let kickoff = ACPMessage {
        id: Uuid::new_v4().to_string(),
        performative: ACPPerformative::Request,
        sender: user_address,
        receiver: group_address.clone(),
        content: ACPContent {
            action: Some("start_group_chat".to_string()),
            parameters: Some(serde_json::json!({"text": input.message})),
            result: None,
            reason: None,
        },
        conversation_id: conversation_id.clone(),
        timestamp: now,
        metadata: Some(serde_json::json!({"round": 0, "speaker": "user"})),
    };
    emitted.push(kickoff.clone());
    let _ = app.emit(
        "group-chat-stream",
        GroupChatStreamEvent {
            request_id: request_id.clone(),
            conversation_id: conversation_id.clone(),
            message_id: Some(kickoff.id.clone()),
            agent_id: None,
            speaker: Some("user".to_string()),
            round: Some(0),
            chunk: None,
            message: Some(kickoff),
            status: "message".to_string(),
        },
    );

    'rounds: for round in 1..=rounds {
        for agent in &input.agents {
            if is_request_cancelled(&state, &request_id) {
                break 'rounds;
            }

            let (session_id, is_new, last_seen) = session_cache
                .get(&agent.id)
                .cloned()
                .ok_or_else(|| format!("{} 缺少会话上下文", agent.name))?;
            let transcript = build_group_transcript(&emitted[last_seen..]);
            let prompt = format!(
                "You are participating in a multi-agent ACP group conversation.\n\nNew ACP messages since your last turn:\n{}\n\nNow respond as {}. Address the other agents when useful, keep the reply focused, and do not invent messages for anyone else.",
                transcript, agent.name
            );
            let system_prompt = agent
                .description
                .clone()
                .unwrap_or_else(|| "An AI agent in the ACP network.".to_string());

            let message_id = Uuid::new_v4().to_string();
            let _ = app.emit(
                "group-chat-stream",
                GroupChatStreamEvent {
                    request_id: request_id.clone(),
                    conversation_id: conversation_id.clone(),
                    message_id: Some(message_id.clone()),
                    agent_id: Some(agent.id.clone()),
                    speaker: Some(agent.name.clone()),
                    round: Some(round),
                    chunk: None,
                    message: None,
                    status: "start".to_string(),
                },
            );

            let response_text = cli.invoke_streaming(
                &agent.config,
                &prompt,
                &system_prompt,
                Some(session_id.as_str()),
                !is_new,
                {
                    let state = &state;
                    let request_id = request_id.clone();
                    move |pid| {
                        state
                            .active_chat_processes
                            .lock()
                            .map_err(|e| ClaudeCliError::Process(e.to_string()))?
                            .insert(request_id.clone(), pid);
                        Ok(())
                    }
                },
                {
                    let state = &state;
                    let request_id = request_id.clone();
                    move || {
                        state
                            .active_chat_processes
                            .lock()
                            .map_err(|e| ClaudeCliError::Process(e.to_string()))?
                            .remove(&request_id);
                        Ok(())
                    }
                },
                {
                    let state = &state;
                    let request_id = request_id.clone();
                    move || is_request_cancelled(state, &request_id)
                },
                {
                    let app = app.clone();
                    let request_id = request_id.clone();
                    let conversation_id = conversation_id.clone();
                    let message_id = message_id.clone();
                    let agent_id = agent.id.clone();
                    let speaker = agent.name.clone();
                    move |chunk| {
                        app.emit(
                            "group-chat-stream",
                            GroupChatStreamEvent {
                                request_id: request_id.clone(),
                                conversation_id: conversation_id.clone(),
                                message_id: Some(message_id.clone()),
                                agent_id: Some(agent_id.clone()),
                                speaker: Some(speaker.clone()),
                                round: Some(round),
                                chunk: Some(chunk.to_string()),
                                message: None,
                                status: "chunk".to_string(),
                            },
                        )
                        .map_err(|e| ClaudeCliError::Process(e.to_string()))
                    }
                },
            );

            let response_text = match response_text {
                Ok(text) => text,
                Err(ClaudeCliError::Cancelled) => break 'rounds,
                Err(error) => return Err(format!("{} 调用失败: {}", agent.name, error)),
            };

            if response_text.trim().is_empty() {
                if is_request_cancelled(&state, &request_id) {
                    break 'rounds;
                }
                continue;
            }

            let msg = ACPMessage {
                id: message_id.clone(),
                performative: ACPPerformative::Inform,
                sender: agent.address.clone(),
                receiver: group_address.clone(),
                content: ACPContent {
                    action: Some("group_chat_turn".to_string()),
                    parameters: None,
                    result: Some(serde_json::json!({"text": response_text})),
                    reason: None,
                },
                conversation_id: conversation_id.clone(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                metadata: Some(serde_json::json!({
                    "round": round,
                    "agentId": agent.id,
                    "speaker": agent.name
                })),
            };
            emitted.push(msg.clone());
            let _ = app.emit(
                "group-chat-stream",
                GroupChatStreamEvent {
                    request_id: request_id.clone(),
                    conversation_id: conversation_id.clone(),
                    message_id: Some(message_id),
                    agent_id: Some(agent.id.clone()),
                    speaker: Some(agent.name.clone()),
                    round: Some(round),
                    chunk: None,
                    message: Some(msg),
                    status: "message".to_string(),
                },
            );

            {
                let db = state.db.lock().map_err(|e| e.to_string())?;
                let _ = persist_agent_session_id(&db, &agent.id, &session_id);
            }

            session_cache.insert(agent.id.clone(), (session_id, false, emitted.len()));
        }
    }

    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        for msg in &emitted[persisted_len..] {
            let _ = db.get_connection().execute(
                "INSERT OR REPLACE INTO messages (id, performative, sender, receiver, content, conversation_id, timestamp, metadata, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    msg.id,
                    serde_json::to_string(&msg.performative).unwrap(),
                    msg.sender,
                    msg.receiver,
                    serde_json::to_string(&msg.content).unwrap(),
                    msg.conversation_id,
                    msg.timestamp,
                    msg.metadata.as_ref().map(|m| serde_json::to_string(m).unwrap()),
                    chrono::Utc::now().to_rfc3339(),
                ],
            );
        }
    }

    clear_cancelled_request(&state, &request_id);
    let _ = app.emit(
        "group-chat-stream",
        GroupChatStreamEvent {
            request_id,
            conversation_id: conversation_id.clone(),
            message_id: None,
            agent_id: None,
            speaker: None,
            round: None,
            chunk: None,
            message: None,
            status: "done".to_string(),
        },
    );

    Ok(GroupChatResponse {
        conversation_id,
        messages: emitted,
    })
}

#[tauri::command]
pub fn stop_agent_group_chat(state: State<'_, AppState>, request_id: String) -> Result<(), String> {
    state
        .cancelled_chat_requests
        .lock()
        .map_err(|e| e.to_string())?
        .insert(request_id.clone());

    let pid = state
        .active_chat_processes
        .lock()
        .map_err(|e| e.to_string())?
        .get(&request_id)
        .copied();

    if let Some(pid) = pid {
        terminate_session(pid).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn build_group_transcript(messages: &[ACPMessage]) -> String {
    messages
        .iter()
        .filter_map(|message| {
            let speaker = message
                .metadata
                .as_ref()
                .and_then(|m| m.get("speaker"))
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| message.sender.as_str());
            let text = extract_message_text(message)?;
            Some(format!("{}: {}", speaker, text))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_message_text(message: &ACPMessage) -> Option<String> {
    message
        .content
        .parameters
        .as_ref()
        .and_then(|v| v.get("text"))
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
        .or_else(|| {
            message
                .content
                .result
                .as_ref()
                .and_then(|v| v.get("text"))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string())
        })
}

fn load_conversation_history(
    state: &State<'_, AppState>,
    conversation_id: &str,
) -> Result<Vec<ACPMessage>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .get_connection()
        .prepare(
            "SELECT id, performative, sender, receiver, content, conversation_id, timestamp, metadata, created_at
             FROM messages WHERE conversation_id = ?1 ORDER BY timestamp ASC, created_at ASC",
        )
        .map_err(|e| e.to_string())?;

    let messages = stmt
        .query_map([conversation_id], |row| {
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

fn resolve_agent_session_id(
    db: &crate::services::Database,
    agent_id: &str,
) -> Result<(String, bool), String> {
    let mut stmt = db
        .get_connection()
        .prepare("SELECT session_id FROM agents WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let existing: Option<String> = stmt
        .query_row([agent_id], |row| row.get(0))
        .map_err(|e| e.to_string())?;

    if let Some(session_id) = existing.filter(|v| !v.trim().is_empty()) {
        Ok((session_id, false))
    } else {
        Ok((Uuid::new_v4().to_string(), true))
    }
}

fn persist_agent_session_id(
    db: &crate::services::Database,
    agent_id: &str,
    session_id: &str,
) -> Result<(), String> {
    db.get_connection()
        .execute(
            "UPDATE agents SET session_id = ?1 WHERE id = ?2",
            rusqlite::params![session_id, agent_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn is_request_cancelled(state: &State<'_, AppState>, request_id: &str) -> bool {
    state
        .cancelled_chat_requests
        .lock()
        .map(|requests| requests.contains(request_id))
        .unwrap_or(true)
}

fn clear_cancelled_request(state: &State<'_, AppState>, request_id: &str) {
    if let Ok(mut requests) = state.cancelled_chat_requests.lock() {
        requests.remove(request_id);
    }
}

/// 获取 Claude CLI 设置
#[tauri::command]
pub fn get_claude_settings() -> Result<ClaudeSettings, String> {
    Ok(ClaudeSettings::default())
}

/// 检测 Claude CLI 是否可用
#[tauri::command]
pub fn check_claude_cli(project_dir: String) -> Result<bool, String> {
    Ok(crate::services::claude_cli::detect_claude_cli(&project_dir))
}
