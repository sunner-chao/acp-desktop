path = r"D:\pro_sunner\demo_vscode\acp-desktop\src-tauri\src\commands\claude.rs"

with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# The exact current text of the second invoke_streaming call (lines 978-1042)
old = """                            let retry_result = cli.invoke_streaming(
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
                            match retry_result {
                                Ok(retry_text) if !retry_text.trim().is_empty() => {"""

new = """                            let retry_result = {
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
                            };
                            match retry_result {
                                Ok((retry_text, _)) if !retry_text.trim().is_empty() => {"""

if old in content:
    content = content.replace(old, new, 1)
    print("SUCCESS: second invoke_streaming wrapper applied")
else:
    print("FAIL: pattern not found")
    # Check for the key distinguishing string
    key = "let retry_result = cli.invoke_streaming("
    idx = content.find(key)
    if idx >= 0:
        print(f"  Found 'cli.invoke_streaming' at index {idx}")
        snippet = content[idx:idx+100]
        print(f"  First 100 chars: {repr(snippet)}")
    else:
        print("  No 'cli.invoke_streaming' found at all")

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)