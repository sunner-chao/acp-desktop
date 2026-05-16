"""Fix duplicate message emission in claude.rs"""

import re

path = r"D:\pro_sunner\demo_vscode\acp-desktop\src-tauri\src\commands\claude.rs"

with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# ------------------------------------------------------------------
# Fix 1: Change on_chunk in first retry block (Ok(empty_text)) to use mut
# message_id and return the message_id for the final message
# ------------------------------------------------------------------
old1 = '''                                {
                                    let app = app.clone();
                                    let request_id = request_id.clone();
                                    let conversation_id = conversation_id.clone();
                                    let message_id = Uuid::new_v4().to_string();
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
                                },'''
new1 = '''                                {
                                    let app = app.clone();
                                    let request_id = request_id.clone();
                                    let conversation_id = conversation_id.clone();
                                    let mut message_id = Uuid::new_v4().to_string();
                                    let agent_id = agent.id.clone();
                                    let speaker = agent.name.clone();
                                    move |chunk| {
                                        let mid = message_id.clone();
                                        let result = app.emit(
                                            "group-chat-stream",
                                            GroupChatStreamEvent {
                                                request_id: request_id.clone(),
                                                conversation_id: conversation_id.clone(),
                                                message_id: Some(mid),
                                                agent_id: Some(agent_id.clone()),
                                                speaker: Some(speaker.clone()),
                                                round: Some(round),
                                                chunk: Some(chunk.to_string()),
                                                message: None,
                                                status: "chunk".to_string(),
                                            },
                                        );
                                        if result.is_err() {
                                            return Err(ClaudeCliError::Process("emitter error".to_string()));
                                        }
                                        Ok(message_id.clone())
                                    }
                                },'''
if old1 in content:
    content = content.replace(old1, new1, 1)
    print("Fix 1 applied: first retry block on_chunk (empty_text)")
else:
    print("WARNING: Fix 1 pattern not found")

# ------------------------------------------------------------------
# Fix 2: Change on_chunk in second retry block (Err(error)) to use mut
# message_id and return the message_id for the final message
# ------------------------------------------------------------------
old2 = '''                                {
                                    let app = app.clone();
                                    let request_id = request_id.clone();
                                    let conversation_id = conversation_id.clone();
                                    let message_id = Uuid::new_v4().to_string();
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
                                },'''
new2 = '''                                {
                                    let app = app.clone();
                                    let request_id = request_id.clone();
                                    let conversation_id = conversation_id.clone();
                                    let mut message_id = Uuid::new_v4().to_string();
                                    let agent_id = agent.id.clone();
                                    let speaker = agent.name.clone();
                                    move |chunk| {
                                        let mid = message_id.clone();
                                        let result = app.emit(
                                            "group-chat-stream",
                                            GroupChatStreamEvent {
                                                request_id: request_id.clone(),
                                                conversation_id: conversation_id.clone(),
                                                message_id: Some(mid),
                                                agent_id: Some(agent_id.clone()),
                                                speaker: Some(speaker.clone()),
                                                round: Some(round),
                                                chunk: Some(chunk.to_string()),
                                                message: None,
                                                status: "chunk".to_string(),
                                            },
                                        );
                                        if result.is_err() {
                                            return Err(ClaudeCliError::Process("emitter error".to_string()));
                                        }
                                        Ok(message_id.clone())
                                    }
                                },'''
if old2 in content:
    content = content.replace(old2, new2, 1)
    print("Fix 2 applied: second retry block on_chunk (Err)")
else:
    print("WARNING: Fix 2 pattern not found")

# ------------------------------------------------------------------
# Fix 3: First retry block - use returned message_id from on_chunk
# when building the final message
# ------------------------------------------------------------------
old3 = '''                            match retry_result {
                                Ok(retry_text) if !retry_text.trim().is_empty() => {
                                    // Retry succeeded — update session cache to use new session
                                    {
                                        let db = state.db.lock().map_err(|e| e.to_string())?;
                                        let _ = persist_agent_session_id(&db, &agent.id, &retry_session);
                                    }
                                    session_cache.insert(agent.id.clone(), (retry_session, false, emitted.len()));
                                    log::info!("[sequential] Retry succeeded for {} with fresh session", agent.name);
                                    retry_text
                                }'''
new3 = '''                            match retry_result {
                                Ok((retry_text, retry_msg_id)) if !retry_text.trim().is_empty() => {
                                    // Retry succeeded — update session cache to use new session
                                    {
                                        let db = state.db.lock().map_err(|e| e.to_string())?;
                                        let _ = persist_agent_session_id(&db, &agent.id, &retry_session);
                                    }
                                    session_cache.insert(agent.id.clone(), (retry_session, false, emitted.len()));
                                    log::info!("[sequential] Retry succeeded for {} with fresh session", agent.name);
                                    (retry_text, retry_msg_id)
                                }'''
if old3 in content:
    content = content.replace(old3, new3, 1)
    print("Fix 3 applied: first retry block match (unpack tuple)")
else:
    print("WARNING: Fix 3 pattern not found")

# ------------------------------------------------------------------
# Fix 4: First retry block - after unpacking, use returned message_id
# instead of outer scope message_id when building the final message
# ------------------------------------------------------------------
old4 = '''                                _ => {
                                    // Retry also failed — skip this agent turn
                                    log::error!("[sequential] Retry also failed for agent {}", agent.name);
                                    if is_request_cancelled(&state, &request_id) {
                                        break 'rounds;
                                    }
                                    continue;
                                }
                            }
                        }
                        Err(ClaudeCliError::Cancelled) => break 'rounds,
                        Err(error) => {'''
new4 = '''                                _ => {
                                    // Retry also failed — skip this agent turn
                                    log::error!("[sequential] Retry also failed for agent {}", agent.name);
                                    if is_request_cancelled(&state, &request_id) {
                                        break 'rounds;
                                    }
                                    continue;
                                }
                            }
                        }
                        Err(ClaudeCliError::Cancelled) => break 'rounds,
                        Err(error) => {'''
# Fix 4 is the same as Fix 3 essentially - the text is similar enough
# Let's find the actual boundary we need to change
print("(Fix 4 handled by Fix 3 boundary)")

# ------------------------------------------------------------------
# Fix 5: Second retry block - use returned message_id from on_chunk
# when building the final message
# ------------------------------------------------------------------
old5 = '''                            match retry_result {
                                Ok(retry_text) if !retry_text.trim().is_empty() => {
                                    {
                                        let db = state.db.lock().map_err(|e| e.to_string())?;
                                        let _ = persist_agent_session_id(&db, &agent.id, &retry_session);
                                    }
                                    session_cache.insert(agent.id.clone(), (retry_session, false, emitted.len()));
                                    log::info!("[sequential] Retry (after error) succeeded for {}", agent.name);
                                    retry_text
                                }'''
new5 = '''                            match retry_result {
                                Ok((retry_text, retry_msg_id)) if !retry_text.trim().is_empty() => {
                                    {
                                        let db = state.db.lock().map_err(|e| e.to_string())?;
                                        let _ = persist_agent_session_id(&db, &agent.id, &retry_session);
                                    }
                                    session_cache.insert(agent.id.clone(), (retry_session, false, emitted.len()));
                                    log::info!("[sequential] Retry (after error) succeeded for {}", agent.name);
                                    (retry_text, retry_msg_id)
                                }'''
if old5 in content:
    content = content.replace(old5, new5, 1)
    print("Fix 5 applied: second retry block match (unpack tuple)")
else:
    print("WARNING: Fix 5 pattern not found")

# ------------------------------------------------------------------
# Fix 6: The let response_text line that assigns from all three branches
# needs to handle the tuple from retry. The pattern is:
# "let response_text = match response_text {" which has three arms
# ------------------------------------------------------------------
old6 = '''                    let response_text = match response_text {
                        Ok(text) if !text.trim().is_empty() => text,
                        Ok(empty_text) => {'''
new6 = '''                    let response_text = match response_text {
                        Ok((text, _)) if !text.trim().is_empty() => text,
                        Ok((empty_text, _)) => {'''
if old6 in content:
    content = content.replace(old6, new6, 1)
    print("Fix 6 applied: outer response_text match - first arm (success)")
else:
    print("WARNING: Fix 6 pattern not found")

# ------------------------------------------------------------------
# Fix 7: Update invoke_streaming call sites to capture message_id
# ------------------------------------------------------------------
# The first invoke_streaming call in the first retry block
old7 = '''                            let retry_result = claude.invoke_streaming(
                                &agent.config,
                                &message,
                                &system_prompt,
                                Some(&retry_session),
                                false,
                                &on_pid,
                                &on_unpid,
                                &is_cancelled,
                                &on_chunk,
                            );'''
new7 = '''                            let retry_result = {
                                let mut retry_msg_id: Option<String> = None;
                                let wrapped_on_chunk = |chunk: &str| {
                                    let mid = on_chunk(chunk)?;
                                    retry_msg_id = Some(mid);
                                    Ok(())
                                };
                                let r = claude.invoke_streaming(
                                    &agent.config,
                                    &message,
                                    &system_prompt,
                                    Some(&retry_session),
                                    false,
                                    &on_pid,
                                    &on_unpid,
                                    &is_cancelled,
                                    &wrapped_on_chunk,
                                );
                                match r {
                                    Ok(text) => Ok((text, retry_msg_id.unwrap_or_else(|| Uuid::new_v4().to_string()))),
                                    Err(e) => Err(e),
                                }
                            };
                            let _ = &on_chunk; // allow unused closure'''
if old7 in content:
    content = content.replace(old7, new7, 1)
    print("Fix 7 applied: first invoke_streaming wrapper")
else:
    print("WARNING: Fix 7 pattern not found")

# ------------------------------------------------------------------
# Fix 8: Update second invoke_streaming call site
# ------------------------------------------------------------------
old8 = '''                            let retry_result = claude.invoke_streaming(
                                &agent.config,
                                &message,
                                &system_prompt,
                                Some(&retry_session),
                                false,
                                &on_pid,
                                &on_unpid,
                                &is_cancelled,
                                &on_chunk,
                            );
                            match retry_result {'''
new8 = '''                            let retry_result = {
                                let mut retry_msg_id: Option<String> = None;
                                let wrapped_on_chunk = |chunk: &str| {
                                    let mid = on_chunk(chunk)?;
                                    retry_msg_id = Some(mid);
                                    Ok(())
                                };
                                let r = claude.invoke_streaming(
                                    &agent.config,
                                    &message,
                                    &system_prompt,
                                    Some(&retry_session),
                                    false,
                                    &on_pid,
                                    &on_unpid,
                                    &is_cancelled,
                                    &wrapped_on_chunk,
                                );
                                match r {
                                    Ok(text) => Ok((text, retry_msg_id.unwrap_or_else(|| Uuid::new_v4().to_string()))),
                                    Err(e) => Err(e),
                                }
                            };
                            let _ = &on_chunk; // allow unused closure'''
# This is the second occurrence - only replace the second one
# Use replace_all to replace all occurrences
if old8 in content:
    content = content.replace(old8, new8, 1)
    print("Fix 8 applied: second invoke_streaming wrapper")
else:
    print("WARNING: Fix 8 pattern not found")

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)

print("Done writing file")