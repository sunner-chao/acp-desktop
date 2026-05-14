use crate::models::{AgentConfig, ClaudeSettings};
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClaudeCliError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Process error: {0}")]
    Process(String),
    #[error("Timeout")]
    Timeout,
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Cancelled")]
    Cancelled,
}

/// Claude CLI 驱动 - 通过 spawn 本地 claude 命令驱动智能体
pub struct ClaudeCli {
    settings: ClaudeSettings,
}

#[derive(Debug, Clone)]
pub struct TerminalSessionInfo {
    pub pid: u32,
    pub started_at: String,
}

impl ClaudeCli {
    pub fn new(settings: ClaudeSettings) -> Self {
        Self { settings }
    }

    /// 调用 Claude CLI 发送消息并获取响应
    /// 使用本地可配置命令调用 Claude CLI 发送消息并获取响应
    pub fn invoke(
        &self,
        config: &AgentConfig,
        message: &str,
        system_prompt: &str,
        session_id: Option<&str>,
        resume_existing: bool,
    ) -> Result<String, ClaudeCliError> {
        let launcher = resolve_launcher_command(config, &self.settings.env_file);

        // 构建 prompt：组合系统提示词和消息
        let full_prompt = format!(
            "[System]\n{}\n\n[Incoming ACP Message]\n{}\n\nRespond concisely in character. Keep under 300 words.",
            system_prompt, message
        );

        log::info!("[ClaudeCli] Invoking agent with launcher={}", launcher);
        log::info!("[ClaudeCli] Project dir: {}", self.settings.project_dir);
        log::info!("[ClaudeCli] Env file: {}", self.settings.env_file);
        log::debug!("[ClaudeCli] Prompt length: {}", full_prompt.len());

        // 检测操作系统，使用正确的命令格式
        #[cfg(target_os = "windows")]
        {
            let mut args = vec![
                "/C".to_string(),
                launcher.clone(),
                "-p".to_string(),
                "--bare".to_string(),
                "--max-turns".to_string(),
                "1".to_string(),
                "--dangerously-skip-permissions".to_string(),
            ];
            if let Some(session_id) = session_id {
                if resume_existing {
                    args.push("-r".to_string());
                    args.push(session_id.to_string());
                } else {
                    args.push("--session-id".to_string());
                    args.push(session_id.to_string());
                }
            }

            let mut child = Command::new("cmd")
                .args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(full_prompt.as_bytes())?;
            }

            let output = child.wait_with_output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !output.stdout.is_empty() {
                    let stdout = String::from_utf8(output.stdout)?;
                    log::warn!("[ClaudeCli] Non-zero exit but has stdout, using it");
                    return Ok(clean_response(&stdout));
                }
                return Err(ClaudeCliError::Process(format!(
                    "Exit code {}: {}",
                    output.status.code().unwrap_or(-1),
                    stderr
                )));
            }

            let stdout = String::from_utf8(output.stdout)?;
            Ok(clean_response(&stdout))
        }

        #[cfg(not(target_os = "windows"))]
        {
            let bash_cmd = format!(
                "echo '{}' | {} -p --bare --max-turns 1 --dangerously-skip-permissions{}",
                full_prompt.replace("'", "'\\''"),
                launcher,
                session_id.map(|sid| {
                    if resume_existing {
                        format!(" -r {}", sid)
                    } else {
                        format!(" --session-id {}", sid)
                    }
                }).unwrap_or_default()
            );
            log::info!("[ClaudeCli] Full bash command: {}", bash_cmd);

            let mut child = Command::new("bash")
                .args(["-c", &bash_cmd])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let output = child.wait_with_output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                log::warn!("[ClaudeCli] Non-zero exit: stderr={}, stdout={}", stderr, stdout);
                if !stdout.is_empty() {
                    return Ok(clean_response(&stdout));
                }
                return Err(ClaudeCliError::Process(format!(
                    "Exit code {}: {}",
                    output.status.code().unwrap_or(-1),
                    stderr
                )));
            }

            let stdout = String::from_utf8(output.stdout)?;
            Ok(clean_response(&stdout))
        }
    }

    pub fn invoke_streaming<F, R, U, C>(
        &self,
        config: &AgentConfig,
        message: &str,
        system_prompt: &str,
        session_id: Option<&str>,
        resume_existing: bool,
        on_pid: R,
        on_unpid: U,
        is_cancelled: C,
        mut on_chunk: F,
    ) -> Result<String, ClaudeCliError>
    where
        F: FnMut(&str) -> Result<(), ClaudeCliError>,
        R: FnOnce(u32) -> Result<(), ClaudeCliError>,
        U: FnOnce() -> Result<(), ClaudeCliError>,
        C: Fn() -> bool,
    {
        let launcher = resolve_launcher_command(config, &self.settings.env_file);
        let full_prompt = format!(
            "[System]\n{}\n\n[Incoming ACP Message]\n{}\n\nRespond concisely in character. Keep under 300 words.",
            system_prompt, message
        );

        #[cfg(target_os = "windows")]
        {
            let mut args = vec![
                "/C".to_string(),
                launcher,
                "-p".to_string(),
                "--bare".to_string(),
                "--max-turns".to_string(),
                "1".to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--include-partial-messages".to_string(),
                "--dangerously-skip-permissions".to_string(),
            ];
            if let Some(session_id) = session_id {
                if resume_existing {
                    args.push("-r".to_string());
                    args.push(session_id.to_string());
                } else {
                    args.push("--session-id".to_string());
                    args.push(session_id.to_string());
                }
            }

            let mut child = Command::new("cmd")
                .args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let pid = child.id();
            on_pid(pid)?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(full_prompt.as_bytes())?;
            }

            let stderr = child.stderr.take();
            let stderr_handle = stderr.map(|mut stderr| {
                std::thread::spawn(move || {
                    let mut text = String::new();
                    let _ = stderr.read_to_string(&mut text);
                    text
                })
            });

            let mut accumulated = String::new();
            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if is_cancelled() {
                        let _ = terminate_session(pid);
                        break;
                    }

                    let line = line?;
                    if line.trim().is_empty() {
                        continue;
                    }

                    if let Some(text) = extract_stream_text(&line) {
                        let (chunk, next_accumulated) = if text.starts_with(&accumulated) {
                            (text[accumulated.len()..].to_string(), text)
                        } else {
                            let mut next = accumulated.clone();
                            next.push_str(&text);
                            (text, next)
                        };
                        if !chunk.is_empty() {
                            on_chunk(&chunk)?;
                        }
                        accumulated = next_accumulated;
                    } else if serde_json::from_str::<serde_json::Value>(&line).is_err() {
                        let cleaned = clean_response(&line);
                        if !cleaned.is_empty() {
                            let chunk = if accumulated.is_empty() {
                                cleaned.clone()
                            } else {
                                format!("\n{}", cleaned)
                            };
                            on_chunk(&chunk)?;
                            accumulated.push_str(&chunk);
                        }
                    }
                }
            }

            let output_status = child.wait()?;
            on_unpid()?;
            let stderr_text = stderr_handle
                .and_then(|handle| handle.join().ok())
                .unwrap_or_default();

            if is_cancelled() && accumulated.trim().is_empty() {
                return Err(ClaudeCliError::Cancelled);
            }

            if !output_status.success() && accumulated.trim().is_empty() {
                return Err(ClaudeCliError::Process(format!(
                    "Exit code {}: {}",
                    output_status.code().unwrap_or(-1),
                    stderr_text
                )));
            }

            Ok(clean_response(&accumulated))
        }

        #[cfg(not(target_os = "windows"))]
        {
            let session_arg = session_id.map(|sid| {
                if resume_existing {
                    format!(" -r {}", sid)
                } else {
                    format!(" --session-id {}", sid)
                }
            }).unwrap_or_default();

            let bash_cmd = format!(
                "echo '{}' | {} -p --verbose --bare --max-turns 1 --output-format stream-json --include-partial-messages --dangerously-skip-permissions{}",
                full_prompt.replace("'", "'\\''"),
                launcher,
                session_arg
            );
            log::info!("[ClaudeCli] Full bash command: {}", bash_cmd);

            let mut child = Command::new("bash")
                .args(["-c", &bash_cmd])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let pid = child.id();
            let _ = on_pid(pid);

            let stderr = child.stderr.take();
            let stderr_handle = stderr.map(|mut stderr| {
                std::thread::spawn(move || {
                    let mut text = String::new();
                    let _ = stderr.read_to_string(&mut text);
                    text
                })
            });

            let mut accumulated = String::new();
            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if is_cancelled() {
                        let _ = terminate_session(pid);
                        break;
                    }

                    let line = line?;
                    if line.trim().is_empty() {
                        continue;
                    }

                    if let Some(text) = extract_stream_text(&line) {
                        let (chunk, next_accumulated) = if text.starts_with(&accumulated) {
                            (text[accumulated.len()..].to_string(), text)
                        } else {
                            let mut next = accumulated.clone();
                            next.push_str(&text);
                            (text, next)
                        };
                        if !chunk.is_empty() {
                            on_chunk(&chunk)?;
                        }
                        accumulated = next_accumulated;
                    } else if serde_json::from_str::<serde_json::Value>(&line).is_err() {
                        let cleaned = clean_response(&line);
                        if !cleaned.is_empty() {
                            let chunk = if accumulated.is_empty() {
                                cleaned.clone()
                            } else {
                                format!("\n{}", cleaned)
                            };
                            on_chunk(&chunk)?;
                            accumulated.push_str(&chunk);
                        }
                    }
                }
            }

            let output_status = child.wait()?;
            let _ = on_unpid();
            let stderr_text = stderr_handle
                .and_then(|handle| handle.join().ok())
                .unwrap_or_default();

            if is_cancelled() && accumulated.trim().is_empty() {
                return Err(ClaudeCliError::Cancelled);
            }

            if !output_status.success() && accumulated.trim().is_empty() {
                return Err(ClaudeCliError::Process(format!(
                    "Exit code {}: {}",
                    output_status.code().unwrap_or(-1),
                    stderr_text
                )));
            }

            Ok(clean_response(&accumulated))
        }
    }

    /// 启动交互式会话（兼容旧实现；当前桌面端主要通过 session-id 续会话）
    pub fn spawn_session(
        &self,
        config: &AgentConfig,
        system_prompt: &str,
    ) -> Result<TerminalSessionInfo, ClaudeCliError> {
        let launcher = resolve_launcher_command(config, &self.settings.env_file);
        let _initial_prompt = format!(
            "You are now online in the ACP network. Your role:\n{}\n\nWait for ACP messages. When you receive one, respond naturally in character. Keep responses under 300 words.",
            system_prompt
        );

        let child = Command::new("cmd")
            .args(["/C", &launcher])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        log::info!(
            "[ClaudeCli] Spawned session for agent with launcher={} (PID: {})",
            launcher,
            child.id()
        );

        Ok(TerminalSessionInfo {
            pid: child.id(),
            started_at: now_iso(),
        })
    }
}

fn extract_stream_text(raw_line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(raw_line).ok()?;

    if value
        .get("type")
        .and_then(|v| v.as_str())
        .is_some_and(|kind| kind == "result" || kind == "system")
    {
        return None;
    }

    let mut texts = Vec::new();
    collect_text_fields(&value, &mut texts);
    if texts.is_empty() {
        None
    } else {
        Some(texts.join(""))
    }
}

fn collect_text_fields(value: &serde_json::Value, texts: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if map
                .get("type")
                .and_then(|v| v.as_str())
                .is_some_and(|kind| kind == "text" || kind == "text_delta")
            {
                if let Some(text) = map.get("text").and_then(|v| v.as_str()) {
                    texts.push(text.to_string());
                }
            }
            if let Some(text) = map.get("partial").and_then(|v| v.as_str()) {
                texts.push(text.to_string());
            }
            if let Some(delta) = map.get("delta").and_then(|v| v.as_object()) {
                if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                    texts.push(text.to_string());
                }
            }
            for (key, child) in map {
                if matches!(key.as_str(), "text" | "partial" | "delta") {
                    continue;
                }
                collect_text_fields(child, texts);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_text_fields(item, texts);
            }
        }
        _ => {}
    }
}

/// 清理 Claude CLI 输出，去掉启动日志等噪音
fn clean_response(raw: &str) -> String {
    // Claude CLI 输出可能包含启动信息，尝试提取核心回复
    let lines: Vec<&str> = raw.lines().collect();

    // 如果输出超过 50 行，可能包含大量启动日志
    if lines.len() > 50 {
        // 找最后有意义的内容
        lines
            .iter()
            .rev()
            .take(50)
            .rev()
            .copied()
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        raw.trim().to_string()
    }
}

fn resolve_launcher_command(config: &AgentConfig, default_env_file: &str) -> String {
    if let Some(explicit) = std::env::var("ACP_CLAUDE_LAUNCHER")
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        return explicit;
    }

    if let Some(explicit_env) = config.claude_env_file.as_deref() {
        let normalized = explicit_env.trim();
        if !normalized.is_empty() && normalized != ".env" {
            if let Some(explicit) = env_to_launcher(normalized) {
                return explicit;
            }
        }
    }

    env_to_launcher(default_env_file).unwrap_or_else(|| "claude-haha".to_string())
}

fn env_to_launcher(env_file: &str) -> Option<String> {
    let env_name = std::path::Path::new(env_file)
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or(env_file)
        .trim();
    let suffix = env_name
        .strip_prefix(".env.")
        .or_else(|| env_name.strip_prefix("env."))
        .unwrap_or("");

    if suffix.is_empty() {
        Some("claude-haha".to_string())
    } else {
        Some(format!("claude-haha-{}", suffix))
    }
}

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs as i64, 0)
        .unwrap_or_else(chrono::Utc::now)
        .to_rfc3339()
}

pub fn terminate_session(pid: u32) -> Result<(), ClaudeCliError> {
    #[cfg(target_os = "windows")]
    {
        let status = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err(ClaudeCliError::Process(format!(
                "Failed to terminate session pid={}",
                pid
            )))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let status = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()?;

        if status.success() {
            Ok(())
        } else {
            Err(ClaudeCliError::Process(format!(
                "Failed to terminate session pid={}",
                pid
            )))
        }
    }
}

/// 获取 claude 命令路径（用于检测）
pub fn detect_claude_cli(project_dir: &str) -> bool {
    let bin_dir = std::path::Path::new(project_dir).join("bin");
    let candidates = [
        "claude-haha-dsv4.cmd",
        "claude-haha-dsv4",
        "claude-haha.cmd",
        "claude-haha",
    ];

    candidates.iter().any(|name| bin_dir.join(name).exists())
}
