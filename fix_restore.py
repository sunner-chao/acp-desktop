"""Restore invoke_streaming to correct signature (returns String, not tuple)"""
import re

path = r"D:\pro_sunner\demo_vscode\acp-desktop\src-tauri\src\commands\claude.rs"

with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

changes = 0

# ------------------------------------------------------------------
# Fix 1: Outer response_text match — restore to plain Ok(text)
# ------------------------------------------------------------------
old1 = '''                    let response_text = match response_text {
                        Ok((text, _)) if !text.trim().is_empty() => text,
                        Ok((empty_text, _)) => {'''
new1 = '''                    let response_text = match response_text {
                        Ok(text) if !text.trim().is_empty() => text,
                        Ok(empty_text) => {'''
if old1 in content:
    content = content.replace(old1, new1, 1)
    changes += 1
    print("Fix 1 applied: outer response_text match (plain Ok)")
else:
    print("WARNING: Fix 1 pattern not found")

# ------------------------------------------------------------------
# Fix 2: First retry block on_chunk — return Ok(()) instead of Ok(message_id.clone())
# ------------------------------------------------------------------
old2 = '''                                        if result.is_err() {
                                            return Err(ClaudeCliError::Process("emitter error".to_string()));
                                        }
                                        Ok(message_id.clone())'''
new2 = '''                                        if result.is_err() {
                                            return Err(ClaudeCliError::Process("emitter error".to_string()));
                                        }
                                        Ok(())'''
if old2 in content:
    content = content.replace(old2, new2, 1)
    changes += 1
    print("Fix 2 applied: first retry on_chunk (return Ok(()))")
else:
    print("WARNING: Fix 2 pattern not found")

# ------------------------------------------------------------------
# Fix 3: First retry block match arm — restore to plain Ok(retry_text)
# ------------------------------------------------------------------
old3 = '''                            match retry_result {
                                Ok((retry_text, _)) if !retry_text.trim().is_empty() => {'''
new3 = '''                            match retry_result {
                                Ok(retry_text) if !retry_text.trim().is_empty() => {'''
if old3 in content:
    content = content.replace(old3, new3, 1)
    changes += 1
    print("Fix 3 applied: first retry match arm (plain Ok)")
else:
    print("WARNING: Fix 3 pattern not found")

# ------------------------------------------------------------------
# Fix 4: Second retry block on_chunk — return Ok(()) instead of Ok(message_id.clone())
# ------------------------------------------------------------------
old4 = '''                                        if result.is_err() {
                                            return Err(ClaudeCliError::Process("emitter error".to_string()));
                                        }
                                        retry_msg_id = Some(message_id.clone());
                                        Ok(())'''
new4 = '''                                        if result.is_err() {
                                            return Err(ClaudeCliError::Process("emitter error".to_string()));
                                        }
                                        Ok(())'''
if old4 in content:
    content = content.replace(old4, new4, 1)
    changes += 1
    print("Fix 4 applied: second retry on_chunk (return Ok(()))")
else:
    print("WARNING: Fix 4 pattern not found")

# ------------------------------------------------------------------
# Fix 5: Second retry block match arm — restore to plain Ok(retry_text)
# ------------------------------------------------------------------
old5 = '''                            match retry_result {
                                Ok((retry_text, _)) if !retry_text.trim().is_empty() => {'''
new5 = '''                            match retry_result {
                                Ok(retry_text) if !retry_text.trim().is_empty() => {'''
# Use replace_all since this pattern may appear twice (both retry blocks)
# First occurrence is already fixed in Fix 3, this is the second
count = content.count(old5)
if count > 0:
    # Replace only the second occurrence
    content = content.replace(old5, new5, 1)
    changes += 1
    print(f"Fix 5 applied: second retry match arm (plain Ok) - {count} occurrences found")
else:
    print("WARNING: Fix 5 pattern not found")

# ------------------------------------------------------------------
# Fix 6: Second retry block — remove the wrapper that returns tuple,
# just use the plain invoke_streaming result
# ------------------------------------------------------------------
old6 = '''                            let retry_result = {
                                let mut retry_msg_id: Option<String> = None;
                                let wrapped_on_chunk = {
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
                                        retry_msg_id = Some(message_id.clone());
                                        Ok(())
                                    }
                                };
                                let r = cli.invoke_streaming(
                                    &agent.config,
                                    &prompt,
                                    &system_prompt,
                                    Some(retry_session.as_str()),
                                    false,
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
                                    &wrapped_on_chunk,
                                );
                                match r {
                                    Ok(text) => Ok((text, retry_msg_id.unwrap_or_else(|| Uuid::new_v4().to_string()))),
                                    Err(e) => Err(e),
                                }
                            };'''
new6 = '''                            let retry_result = cli.invoke_streaming(
                                &agent.config,
                                &prompt,
                                &system_prompt,
                                Some(retry_session.as_str()),
                                false,
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
                                    let agent_id = agent.id.clone();
                                    let speaker = agent.name.clone();
                                    move |chunk| {
                                        let result = app.emit(
                                            "group-chat-stream",
                                            GroupChatStreamEvent {
                                                request_id: request_id.clone(),
                                                conversation_id: conversation_id.clone(),
                                                message_id: None,
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
                                        Ok(())
                                    }
                                },
                            );'''
if old6 in content:
    content = content.replace(old6, new6, 1)
    changes += 1
    print("Fix 6 applied: second retry block (remove wrapper, plain invoke_streaming)")
else:
    print("WARNING: Fix 6 pattern not found")

print(f"\nTotal changes applied: {changes}/6")

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)

print("Done")