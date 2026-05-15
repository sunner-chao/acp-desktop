"""Minimal fix for duplicate message emission in claude.rs"""
import re

path = r"D:\pro_sunner\demo_vscode\acp-desktop\src-tauri\src\commands\claude.rs"

with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

changes = 0

# ------------------------------------------------------------------
# Fix 1: First invoke_streaming in Ok(empty_text) retry block
# Wrap it to capture message_id from on_chunk callback
# ------------------------------------------------------------------
# Find the first invoke_streaming call inside the "Ok(empty_text)" block
# It appears after: "let retry_session = Uuid::new_v4().to_string();"
old1 = '''                            let retry_session = Uuid::new_v4().to_string();
                            let retry_result = claude.invoke_streaming(
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
new1 = '''                            let retry_session = Uuid::new_v4().to_string();
                            let retry_result = {
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
                            let _ = &on_chunk;
                            match retry_result {'''
if old1 in content:
    content = content.replace(old1, new1, 1)
    changes += 1
    print("Fix 1 applied: first invoke_streaming wrapper (empty_text retry)")
else:
    print("WARNING: Fix 1 pattern not found")

# ------------------------------------------------------------------
# Fix 2: on_chunk in first retry block — make message_id mutable
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
    changes += 1
    print("Fix 2 applied: first retry on_chunk (mut message_id + return)")
else:
    print("WARNING: Fix 2 pattern not found")

# ------------------------------------------------------------------
# Fix 3: First retry block match arm — unpack tuple
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
                                Ok((retry_text, _)) if !retry_text.trim().is_empty() => {
                                    // Retry succeeded — update session cache to use new session
                                    {
                                        let db = state.db.lock().map_err(|e| e.to_string())?;
                                        let _ = persist_agent_session_id(&db, &agent.id, &retry_session);
                                    }
                                    session_cache.insert(agent.id.clone(), (retry_session, false, emitted.len()));
                                    log::info!("[sequential] Retry succeeded for {} with fresh session", agent.name);
                                    retry_text
                                }'''
if old3 in content:
    content = content.replace(old3, new3, 1)
    changes += 1
    print("Fix 3 applied: first retry match arm (unpack tuple, ignore msg_id)")
else:
    print("WARNING: Fix 3 pattern not found")

# ------------------------------------------------------------------
# Fix 4: Outer response_text match — handle (text, _) from retry
# ------------------------------------------------------------------
old4 = '''                    let response_text = match response_text {
                        Ok(text) if !text.trim().is_empty() => text,
                        Ok(empty_text) => {'''
new4 = '''                    let response_text = match response_text {
                        Ok((text, _)) if !text.trim().is_empty() => text,
                        Ok((empty_text, _)) => {'''
if old4 in content:
    content = content.replace(old4, new4, 1)
    changes += 1
    print("Fix 4 applied: outer response_text match (unpack tuple)")
else:
    print("WARNING: Fix 4 pattern not found")

# ------------------------------------------------------------------
# Fix 5: Second retry block on_chunk — make message_id mutable + return
# ------------------------------------------------------------------
old5 = '''                                {
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
                                },
                            );
                            match retry_result {'''
new5 = '''                                {
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
                                },
                            );
                            match retry_result {'''
if old5 in content:
    content = content.replace(old5, new5, 1)
    changes += 1
    print("Fix 5 applied: second retry on_chunk (mut message_id + return)")
else:
    print("WARNING: Fix 5 pattern not found")

# ------------------------------------------------------------------
# Fix 6: Second invoke_streaming in Err(error) retry block
# Wrap it to capture message_id from on_chunk callback
# ------------------------------------------------------------------
old6 = '''                            let retry_result = claude.invoke_streaming(
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
                            match retry_result {
                                Ok(retry_text) if !retry_text.trim().is_empty() => {
                                    {
                                        let db = state.db.lock().map_err(|e| e.to_string())?;
                                        let _ = persist_agent_session_id(&db, &agent.id, &retry_session);
                                    }
                                    session_cache.insert(agent.id.clone(), (retry_session, false, emitted.len()));
                                    log::info!("[sequential] Retry (after error) succeeded for {}", agent.name);
                                    retry_text
                                }'''
new6 = '''                            let retry_result = {
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
                            let _ = &on_chunk;
                            match retry_result {
                                Ok((retry_text, _)) if !retry_text.trim().is_empty() => {
                                    {
                                        let db = state.db.lock().map_err(|e| e.to_string())?;
                                        let _ = persist_agent_session_id(&db, &agent.id, &retry_session);
                                    }
                                    session_cache.insert(agent.id.clone(), (retry_session, false, emitted.len()));
                                    log::info!("[sequential] Retry (after error) succeeded for {}", agent.name);
                                    retry_text
                                }'''
if old6 in content:
    content = content.replace(old6, new6, 1)
    changes += 1
    print("Fix 6 applied: second invoke_streaming wrapper + match arm")
else:
    print("WARNING: Fix 6 pattern not found")

print(f"\nTotal changes applied: {changes}/6")

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)

print("Done")