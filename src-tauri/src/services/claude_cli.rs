use crate::models::{AgentConfig, ClaudeSettings};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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
    #[error("Context overflow: {0}")]
    ContextOverflow(String),
}

/// Maximum time (seconds) to wait for a single CLI invocation before killing it
const CLI_TIMEOUT_SECS: u64 = 600;
const DEFAULT_CLI_MAX_TURNS: &str = "6";
const DEFAULT_ADAPTIVE_MIN_TURNS: u32 = 10;
const DEFAULT_ADAPTIVE_MAX_TURNS: u32 = 50;
const ABSOLUTE_MAX_TURNS: u32 = 50;

#[derive(Debug, Clone)]
pub struct StreamToolCall {
    pub name: String,
    pub input: String,
}

fn configured_turn_bound(key: &str, fallback: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .map(|value| value.clamp(1, ABSOLUTE_MAX_TURNS))
        .unwrap_or(fallback)
}

fn fixed_max_turns_arg() -> Option<String> {
    std::env::var("ACP_CLAUDE_MAX_TURNS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .map(|value| value.clamp(1, ABSOLUTE_MAX_TURNS).to_string())
}

fn adaptive_max_turns_arg(
    config: &AgentConfig,
    message: &str,
    system_prompt: &str,
    resume_existing: bool,
    streaming: bool,
) -> String {
    if let Some(fixed) = fixed_max_turns_arg() {
        return fixed;
    }

    let min_turns = configured_turn_bound("ACP_CLAUDE_MIN_TURNS", DEFAULT_ADAPTIVE_MIN_TURNS);
    let max_turns =
        configured_turn_bound("ACP_CLAUDE_ADAPTIVE_MAX_TURNS", DEFAULT_ADAPTIVE_MAX_TURNS)
            .max(min_turns);
    let combined_len = message.len() + system_prompt.len();
    let task_text = format!("{} {}", message, system_prompt).to_lowercase();

    let mut budget = DEFAULT_CLI_MAX_TURNS
        .parse::<u32>()
        .unwrap_or(DEFAULT_ADAPTIVE_MIN_TURNS);

    if streaming {
        budget += 1;
    }
    if resume_existing {
        budget += 1;
    }
    if config.thinking_enabled.unwrap_or(false) {
        budget += 1;
    }
    if combined_len > 8_000 {
        budget += 1;
    }
    if combined_len > 20_000 {
        budget += 2;
    }
    if combined_len > 45_000 {
        budget += 2;
    }
    if task_text.contains("工具")
        || task_text.contains("tool")
        || task_text.contains("文件")
        || task_text.contains("日志")
        || task_text.contains("测试")
        || task_text.contains("构建")
        || task_text.contains("运行")
        || task_text.contains("修复")
        || task_text.contains("实现")
        || task_text.contains("检查")
        || task_text.contains("查看")
        || task_text.contains("代码")
        || task_text.contains("repo")
        || task_text.contains("build")
        || task_text.contains("test")
        || task_text.contains("fix")
        || task_text.contains("implement")
        || task_text.contains("debug")
        || task_text.contains("log")
    {
        budget += 2;
    }

    let clamped = budget.clamp(min_turns, max_turns);
    log::info!(
        "[ClaudeCli] Adaptive max-turns={}, prompt_chars={}, resume={}, streaming={}, thinking={}, bounds={}..{}",
        clamped,
        combined_len,
        resume_existing,
        streaming,
        config.thinking_enabled.unwrap_or(false),
        min_turns,
        max_turns
    );
    clamped.to_string()
}

fn resolve_agent_model(config: &AgentConfig) -> Option<&str> {
    config
        .claude_model
        .as_deref()
        .or(config.model.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter(|value| !value.starts_with("http://") && !value.starts_with("https://"))
}

fn setting_sources_arg() -> Option<String> {
    let sources =
        std::env::var("ACP_CLAUDE_SETTING_SOURCES").unwrap_or_else(|_| "project,local".to_string());
    let trimmed = sources.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(format!("--setting-sources={trimmed}"))
    }
}

fn append_common_cli_args(args: &mut Vec<String>) {
    if let Some(setting_sources) = setting_sources_arg() {
        args.push(setting_sources);
    }
}

fn isolated_config_dir(project_dir: &str, env_file: &str, config: &AgentConfig) -> PathBuf {
    let model = resolve_agent_model(config).unwrap_or("default-model");
    let endpoint = config.endpoint.as_deref().unwrap_or("default-endpoint");
    let raw = format!("{env_file}-{model}-{endpoint}");
    let namespace = sanitize_namespace(&raw);
    Path::new(project_dir)
        .join(".acp-claude-config")
        .join(namespace)
}

fn sanitize_namespace(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "default".to_string()
    } else {
        trimmed.chars().take(96).collect()
    }
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

/// 解析 agent 使用的 env file（优先 agent 配置，否则用全局默认）
fn resolve_env_file(config: &AgentConfig, default_env_file: &str) -> String {
    if let Some(env_file) = config.claude_env_file.as_deref() {
        let trimmed = env_file.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    default_env_file.to_string()
}

/// 读取 .env 文件并显式设置到 Command，确保覆盖父进程继承的环境变量
fn load_env_file_to_command(
    cmd: &mut Command,
    project_dir: &str,
    env_file: &str,
    config: &AgentConfig,
) {
    clear_provider_env(cmd);
    configure_process_isolation(cmd, project_dir, env_file, config);

    let env_path = std::path::Path::new(project_dir).join(env_file);
    let mut env_provides_auth = false;
    let mut env_auth_token: Option<String> = None;
    let mut env_has_api_key = false;
    if let Ok(content) = std::fs::read_to_string(&env_path) {
        for line in content.lines() {
            let trimmed = line.trim();
            // 跳过空行和注释
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            // 解析 KEY=VALUE
            if let Some(eq_pos) = trimmed.find('=') {
                let key = trimmed[..eq_pos].trim();
                let value = trimmed[eq_pos + 1..].trim();
                if !key.is_empty() {
                    if matches!(key, "ANTHROPIC_AUTH_TOKEN" | "ANTHROPIC_API_KEY") && !value.is_empty() {
                        env_provides_auth = true;
                    }
                    if key == "ANTHROPIC_AUTH_TOKEN" && !value.is_empty() {
                        env_auth_token = Some(value.to_string());
                    }
                    if key == "ANTHROPIC_API_KEY" {
                        env_has_api_key = !value.is_empty();
                    }
                    cmd.env(key, value);
                }
            }
        }
        log::info!(
            "[ClaudeCli] Loaded env vars from {} (overriding inherited env)",
            env_path.display()
        );
    } else {
        log::warn!(
            "[ClaudeCli] Failed to read env file: {}",
            env_path.display()
        );
    }

    apply_agent_config_to_command(cmd, config, env_provides_auth, env_auth_token.as_deref(), env_has_api_key);
}

fn clear_provider_env(cmd: &mut Command) {
    for key in [
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "OPENAI_API_KEY",
        "OPENAI_BASE_URL",
        "OPENAI_MODEL",
        "API_TIMEOUT_MS",
    ] {
        cmd.env_remove(key);
    }
}

fn configure_process_isolation(
    cmd: &mut Command,
    project_dir: &str,
    env_file: &str,
    config: &AgentConfig,
) {
    cmd.env("CLAUDE_CODE_PROVIDER_MANAGED_BY_HOST", "1");

    if std::env::var("ACP_CLAUDE_SHARED_CONFIG_DIR")
        .map(|value| {
            matches!(
                value.trim().to_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
    {
        return;
    }

    let config_dir = isolated_config_dir(project_dir, env_file, config);
    if let Err(error) = std::fs::create_dir_all(&config_dir) {
        log::warn!(
            "[ClaudeCli] Failed to create isolated CLAUDE_CONFIG_DIR {}: {}",
            config_dir.display(),
            error
        );
        return;
    }

    cmd.env("CLAUDE_CONFIG_DIR", &config_dir);
    log::info!(
        "[ClaudeCli] Using isolated CLAUDE_CONFIG_DIR: {}",
        config_dir.display()
    );
}

fn apply_agent_config_to_command(
    cmd: &mut Command,
    config: &AgentConfig,
    env_provides_auth: bool,
    env_auth_token: Option<&str>,
    env_has_api_key: bool,
) {
    let endpoint = config
        .endpoint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(endpoint) = endpoint {
        cmd.env("ANTHROPIC_BASE_URL", endpoint);
    }

    let api_key = config
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    // Case 1: env file provides auth AND agent config has apiKey
    //   - Use env file's ANTHROPIC_AUTH_TOKEN for bearer auth
    //   - BUT ensure ANTHROPIC_API_KEY is also set (required in --bare mode)
    //   - If env file only has ANTHROPIC_AUTH_TOKEN without ANTHROPIC_API_KEY,
    //     sync ANTHROPIC_API_KEY = ANTHROPIC_AUTH_TOKEN so the SDK's x-api-key header works
    // Case 2: env file provides auth, no agent apiKey
    //   - Same sync logic: ensure ANTHROPIC_API_KEY mirrors ANTHROPIC_AUTH_TOKEN
    // Case 3: no env auth, agent config has apiKey
    //   - Set both ANTHROPIC_AUTH_TOKEN and ANTHROPIC_API_KEY from config
    if env_provides_auth {
        // Env file provides auth token. Ensure ANTHROPIC_API_KEY is also set
        // because the Claude CLI in --bare mode only reads ANTHROPIC_API_KEY for
        // the x-api-key header (ANTHROPIC_AUTH_TOKEN is only used for Authorization: Bearer).
        // Third-party API providers may only check x-api-key.
        if !env_has_api_key {
            if let Some(config_key) = api_key {
                cmd.env("ANTHROPIC_API_KEY", config_key);
                log::info!(
                    "[ClaudeCli] Synced ANTHROPIC_API_KEY from agent config (env file only had ANTHROPIC_AUTH_TOKEN)"
                );
            } else if let Some(token) = env_auth_token {
                cmd.env("ANTHROPIC_API_KEY", token);
                log::info!(
                    "[ClaudeCli] Synced ANTHROPIC_API_KEY from ANTHROPIC_AUTH_TOKEN (env file had no ANTHROPIC_API_KEY)"
                );
            }
        }
    } else if let Some(api_key) = api_key {
        // No env auth; use agent config's apiKey for both
        cmd.env("ANTHROPIC_AUTH_TOKEN", api_key);
        cmd.env("ANTHROPIC_API_KEY", api_key);
    }

    let model = resolve_agent_model(config);
    if let Some(model) = model {
        cmd.env("ANTHROPIC_MODEL", model);
        cmd.env("ANTHROPIC_DEFAULT_HAIKU_MODEL", model);
        cmd.env("ANTHROPIC_DEFAULT_OPUS_MODEL", model);
        cmd.env("ANTHROPIC_DEFAULT_SONNET_MODEL", model);
    }

    let effective_api_key = if env_provides_auth {
        api_key.or(env_auth_token)
    } else {
        api_key
    };
    log::info!(
        "[ClaudeCli] Applied agent-local provider overrides: endpoint={}, api_key={}, model={}, env_auth={}, synced_api_key={}",
        endpoint.is_some(),
        effective_api_key.is_some(),
        model.unwrap_or("<env-file>"),
        env_provides_auth,
        !env_has_api_key && (api_key.is_some() || env_auth_token.is_some()),
    );
}

/// 在 Windows 上启动 Claude CLI 子进程，直接调用 bun（绕过 cmd /C 避免 stdin 转发问题）
/// 返回 (child, pid)
#[cfg(target_os = "windows")]
fn spawn_cli_process(
    settings: &ClaudeSettings,
    config: &AgentConfig,
    env_file: &str,
    shared_args: &[String],
    session_id: Option<&str>,
    resume_existing: bool,
    thinking_enabled: bool,
) -> Result<(std::process::Child, u32), ClaudeCliError> {
    let mut args: Vec<String> = vec![
        "--env-file".to_string(),
        env_file.to_string(),
        settings.entrypoint.clone(),
    ];
    append_common_cli_args(&mut args);
    args.extend(shared_args.iter().cloned());
    if let Some(model) = resolve_agent_model(config) {
        args.push("--model".to_string());
        args.push(model.to_string());
    }
    if let Some(session_id) = session_id {
        if resume_existing {
            args.push("-r".to_string());
            args.push(session_id.to_string());
        } else {
            args.push("--session-id".to_string());
            args.push(session_id.to_string());
        }
    }
    if thinking_enabled {
        args.push("--thinking".to_string());
        args.push("enabled".to_string());
    }

    log::info!(
        "[ClaudeCli] Spawning: {} {} (cwd: {}, env_file: {})",
        settings.command,
        args.join(" "),
        settings.project_dir,
        env_file
    );

    #[cfg(target_os = "windows")]
    let mut cmd = {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new(&settings.command);
        c.creation_flags(0x08000000); // CREATE_NO_WINDOW
        c
    };
    #[cfg(not(target_os = "windows"))]
    let mut cmd = Command::new(&settings.command);

    // 显式加载 env 文件，覆盖父进程继承的污染变量
    load_env_file_to_command(&mut cmd, &settings.project_dir, env_file, config);

    let mut child = cmd
        .args(&args)
        .current_dir(&settings.project_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let pid = child.id();
    log::info!("[ClaudeCli] Process spawned with PID: {}", pid);
    Ok((child, pid))
}

/// 在非 Windows 平台上不需要此函数
#[cfg(not(target_os = "windows"))]
fn spawn_cli_process(
    _settings: &ClaudeSettings,
    _config: &AgentConfig,
    _env_file: &str,
    _shared_args: &[String],
    _session_id: Option<&str>,
    _resume_existing: bool,
    _thinking_enabled: bool,
) -> Result<(std::process::Child, u32), ClaudeCliError> {
    unreachable!("spawn_cli_process is only used on Windows")
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
        let env_file = resolve_env_file(config, &self.settings.env_file);

        // 构建 prompt：组合系统提示词和消息
        let full_prompt = format!(
            "[System]\n{}\n\n[Incoming ACP Message]\n{}\n\nRespond concisely in character. Keep under 300 words.",
            system_prompt, message
        );

        log::info!(
            "[ClaudeCli] Invoking agent, project dir: {}",
            self.settings.project_dir
        );
        log::info!("[ClaudeCli] Env file: {}", env_file);
        log::debug!("[ClaudeCli] Prompt length: {}", full_prompt.len());

        // 检测操作系统，使用正确的命令格式
        #[cfg(target_os = "windows")]
        {
            let max_turns =
                adaptive_max_turns_arg(config, message, system_prompt, resume_existing, false);
            let shared_args = vec![
                "-p".to_string(),
                "--bare".to_string(),
                "--max-turns".to_string(),
                max_turns,
                "--dangerously-skip-permissions".to_string(),
            ];
            let (mut child, pid) = spawn_cli_process(
                &self.settings,
                config,
                &env_file,
                &shared_args,
                session_id,
                resume_existing,
                config.thinking_enabled.unwrap_or(false),
            )?;

            log::info!(
                "[ClaudeCli::invoke] Writing prompt to stdin ({} bytes)...",
                full_prompt.len()
            );
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(full_prompt.as_bytes())?;
                // stdin is dropped here, closing the pipe so the child reads EOF
            }
            log::info!(
                "[ClaudeCli::invoke] Stdin closed, waiting for process PID={}...",
                pid
            );

            let output = child.wait_with_output()?;
            let _ = pid;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout_lossy = String::from_utf8_lossy(&output.stdout);
                log::error!(
                    "[ClaudeCli::invoke] Exit code: {}, stderr: {}, stdout_len: {}",
                    output.status.code().unwrap_or(-1),
                    stderr,
                    stdout_lossy.len()
                );
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
            log::info!(
                "[ClaudeCli::invoke] Success, stdout length: {}",
                stdout.len()
            );
            let cleaned = clean_response(&stdout);
            if is_context_overflow_text(&cleaned) {
                log::warn!("[ClaudeCli::invoke] Context overflow detected in response");
                return Err(ClaudeCliError::ContextOverflow(cleaned));
            }
            Ok(cleaned)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let max_turns =
                adaptive_max_turns_arg(config, message, system_prompt, resume_existing, false);
            let mut args: Vec<String> = vec![
                "--env-file".to_string(),
                env_file.clone(),
                self.settings.entrypoint.clone(),
            ];
            append_common_cli_args(&mut args);
            args.extend([
                "-p".to_string(),
                "--bare".to_string(),
                "--max-turns".to_string(),
                max_turns,
                "--dangerously-skip-permissions".to_string(),
            ]);
            if let Some(model) = resolve_agent_model(config) {
                args.push("--model".to_string());
                args.push(model.to_string());
            }

            if let Some(sid) = session_id {
                if resume_existing {
                    args.push("-r".to_string());
                    args.push(sid.to_string());
                } else {
                    args.push("--session-id".to_string());
                    args.push(sid.to_string());
                }
            }

            log::info!(
                "[ClaudeCli::invoke] Running: {} {:?}",
                self.settings.command,
                args
            );

            let mut cmd = Command::new(&self.settings.command);
            load_env_file_to_command(&mut cmd, &self.settings.project_dir, &env_file, config);
            let mut child = cmd
                .args(&args)
                .current_dir(&self.settings.project_dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let mut stdin = child.stdin.take().unwrap();
            stdin.write_all(full_prompt.as_bytes())?;
            drop(stdin);

            // Wait with timeout to prevent indefinite hangs
            let pid = child.id();
            let output_result = child.wait_with_output();
            let output = match output_result {
                Ok(o) => o,
                Err(e) => {
                    let _ = terminate_session(pid);
                    return Err(ClaudeCliError::Io(e));
                }
            };

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                log::warn!(
                    "[ClaudeCli] Non-zero exit: stderr={}, stdout={}",
                    stderr,
                    stdout
                );
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
            let cleaned = clean_response(&stdout);
            if is_context_overflow_text(&cleaned) {
                log::warn!("[ClaudeCli::invoke] Context overflow detected in response");
                return Err(ClaudeCliError::ContextOverflow(cleaned));
            }
            Ok(cleaned)
        }
    }

    pub fn invoke_streaming<F, R, U, C, T>(
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
        mut on_tool_call: T,
    ) -> Result<String, ClaudeCliError>
    where
        F: FnMut(&str) -> Result<(), ClaudeCliError>,
        R: FnOnce(u32) -> Result<(), ClaudeCliError>,
        U: FnOnce() -> Result<(), ClaudeCliError>,
        C: Fn() -> bool,
        T: FnMut(StreamToolCall) -> Result<(), ClaudeCliError>,
    {
        let full_prompt = format!(
            "[System]\n{}\n\n[Incoming ACP Message]\n{}\n\nRespond concisely in character. Keep under 300 words.",
            system_prompt, message
        );

        #[cfg(target_os = "windows")]
        {
            let env_file = resolve_env_file(config, &self.settings.env_file);
            let max_turns =
                adaptive_max_turns_arg(config, message, system_prompt, resume_existing, true);
            let shared_args = vec![
                "-p".to_string(),
                "--verbose".to_string(),
                "--bare".to_string(),
                "--max-turns".to_string(),
                max_turns,
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--include-partial-messages".to_string(),
                "--dangerously-skip-permissions".to_string(),
            ];
            let (mut child, pid) = spawn_cli_process(
                &self.settings,
                config,
                &env_file,
                &shared_args,
                session_id,
                resume_existing,
                config.thinking_enabled.unwrap_or(false),
            )?;

            on_pid(pid)?;

            log::info!(
                "[ClaudeCli::invoke_streaming] Writing prompt to stdin ({} bytes)...",
                full_prompt.len()
            );
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(full_prompt.as_bytes())?;
                // stdin dropped here, closing pipe
            }
            log::info!(
                "[ClaudeCli::invoke_streaming] Stdin closed, reading stdout from PID={}...",
                pid
            );

            let stderr = child.stderr.take();
            let stderr_handle = stderr.map(|mut stderr| {
                std::thread::spawn(move || {
                    let mut text = String::new();
                    let _ = stderr.read_to_string(&mut text);
                    text
                })
            });

            let mut accumulated = String::new();
            let mut seen_tool_calls = std::collections::HashSet::new();
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

                    handle_stream_line(
                        &line,
                        &mut accumulated,
                        &mut seen_tool_calls,
                        &mut on_chunk,
                        &mut on_tool_call,
                    )?;
                }
            }

            let output_status = child.wait()?;
            on_unpid()?;
            let stderr_text = stderr_handle
                .and_then(|handle| handle.join().ok())
                .unwrap_or_default();

            log::info!(
                "[ClaudeCli::invoke_streaming] Process exited with code {:?}, accumulated {} chars, stderr: {}",
                output_status.code(),
                accumulated.len(),
                &stderr_text[..stderr_text.len().min(500)]
            );

            if is_cancelled() && accumulated.trim().is_empty() {
                log::info!("[ClaudeCli::invoke_streaming] Request cancelled, no output");
                return Err(ClaudeCliError::Cancelled);
            }

            if !output_status.success() && accumulated.trim().is_empty() {
                log::error!("[ClaudeCli::invoke_streaming] Process failed with empty output");
                return Err(ClaudeCliError::Process(format!(
                    "Exit code {}: {}",
                    output_status.code().unwrap_or(-1),
                    stderr_text
                )));
            }

            let cleaned = clean_response(&accumulated);
            if is_context_overflow_text(&cleaned) {
                log::warn!(
                    "[ClaudeCli::invoke_streaming] Context overflow detected in response: {}",
                    &cleaned[..cleaned.len().min(200)]
                );
                return Err(ClaudeCliError::ContextOverflow(cleaned));
            }
            Ok(cleaned)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let env_file = resolve_env_file(config, &self.settings.env_file);
            let max_turns =
                adaptive_max_turns_arg(config, message, system_prompt, resume_existing, true);
            let mut args: Vec<String> = vec![
                "--env-file".to_string(),
                env_file.clone(),
                self.settings.entrypoint.clone(),
            ];
            append_common_cli_args(&mut args);
            args.extend([
                "-p".to_string(),
                "--verbose".to_string(),
                "--bare".to_string(),
                "--max-turns".to_string(),
                max_turns,
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--include-partial-messages".to_string(),
                "--dangerously-skip-permissions".to_string(),
            ]);
            if let Some(model) = resolve_agent_model(config) {
                args.push("--model".to_string());
                args.push(model.to_string());
            }

            if let Some(sid) = session_id {
                if resume_existing {
                    args.push("-r".to_string());
                    args.push(sid.to_string());
                } else {
                    args.push("--session-id".to_string());
                    args.push(sid.to_string());
                }
            }

            if config.thinking_enabled.unwrap_or(false) {
                args.push("--thinking".to_string());
                args.push("enabled".to_string());
            }

            log::info!(
                "[ClaudeCli::invoke_streaming] Running: {} {:?}",
                self.settings.command,
                args
            );

            let mut cmd = Command::new(&self.settings.command);
            load_env_file_to_command(&mut cmd, &self.settings.project_dir, &env_file, config);
            let mut child = cmd
                .args(&args)
                .current_dir(&self.settings.project_dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let pid = child.id();
            let _ = on_pid(pid);

            log::info!(
                "[ClaudeCli::invoke_streaming] Writing prompt to stdin ({} bytes)...",
                full_prompt.len()
            );
            let mut stdin = child.stdin.take().unwrap();
            stdin.write_all(full_prompt.as_bytes())?;
            drop(stdin);
            log::info!(
                "[ClaudeCli::invoke_streaming] Stdin closed, reading stdout from PID={}...",
                pid
            );

            let stderr = child.stderr.take();
            let stderr_handle = stderr.map(|mut stderr| {
                std::thread::spawn(move || {
                    let mut text = String::new();
                    let _ = stderr.read_to_string(&mut text);
                    text
                })
            });

            // Read stdout lines in a separate thread, send via channel to allow timeout
            let (line_tx, line_rx) = std::sync::mpsc::channel::<String>();
            let stdout_reader = child.stdout.take();
            let reader_cancelled = Arc::new(Mutex::new(false));
            let reader_cancelled_clone = Arc::clone(&reader_cancelled);

            let reader_handle = stdout_reader.map(move |stdout| {
                std::thread::spawn(move || {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines() {
                        if *reader_cancelled.lock().unwrap() {
                            break;
                        }
                        match line {
                            Ok(l) => {
                                if line_tx.send(l).is_err() {
                                    break; // receiver dropped
                                }
                            }
                            Err(_) => break,
                        }
                    }
                })
            });

            let mut accumulated = String::new();
            let mut seen_tool_calls = std::collections::HashSet::new();
            let timeout = Duration::from_secs(CLI_TIMEOUT_SECS);
            let mut last_activity = std::time::Instant::now();

            loop {
                if is_cancelled() {
                    let _ = terminate_session(pid);
                    *reader_cancelled_clone.lock().unwrap() = true;
                    break;
                }

                match line_rx.recv_timeout(Duration::from_secs(5)) {
                    Ok(line) => {
                        last_activity = std::time::Instant::now();
                        if line.trim().is_empty() {
                            continue;
                        }

                        handle_stream_line(
                            &line,
                            &mut accumulated,
                            &mut seen_tool_calls,
                            &mut on_chunk,
                            &mut on_tool_call,
                        )?;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // Check if we've exceeded the overall timeout
                        if last_activity.elapsed() > timeout {
                            log::warn!(
                                "[ClaudeCli::invoke_streaming] Timeout ({})s reached for PID={}, killing process",
                                CLI_TIMEOUT_SECS, pid
                            );
                            let _ = terminate_session(pid);
                            *reader_cancelled_clone.lock().unwrap() = true;
                            break;
                        }
                        // Otherwise just continue waiting
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        // Reader thread finished (stdout closed)
                        break;
                    }
                }
            }

            // Wait for reader thread to finish
            if let Some(handle) = reader_handle {
                let _ = handle.join();
            }

            let output_status = child.wait()?;
            let _ = on_unpid();
            let stderr_text = stderr_handle
                .and_then(|handle| handle.join().ok())
                .unwrap_or_default();

            if is_cancelled() && accumulated.trim().is_empty() {
                return Err(ClaudeCliError::Cancelled);
            }

            // Check if we timed out
            if last_activity.elapsed() > timeout && accumulated.trim().is_empty() {
                return Err(ClaudeCliError::Timeout);
            }

            if !output_status.success() && accumulated.trim().is_empty() {
                return Err(ClaudeCliError::Process(format!(
                    "Exit code {}: {}",
                    output_status.code().unwrap_or(-1),
                    stderr_text
                )));
            }

            let cleaned = clean_response(&accumulated);
            if is_context_overflow_text(&cleaned) {
                log::warn!(
                    "[ClaudeCli::invoke_streaming] Context overflow detected in response: {}",
                    &cleaned[..cleaned.len().min(200)]
                );
                return Err(ClaudeCliError::ContextOverflow(cleaned));
            }
            Ok(cleaned)
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

        #[cfg(target_os = "windows")]
        let child = {
            use std::os::windows::process::CommandExt;
            let mut c = Command::new("cmd");
            c.creation_flags(0x08000000); // CREATE_NO_WINDOW
            c.args(["/C", &launcher])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?
        };
        #[cfg(not(target_os = "windows"))]
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

fn handle_stream_line<F, T>(
    raw_line: &str,
    accumulated: &mut String,
    seen_tool_calls: &mut std::collections::HashSet<String>,
    on_chunk: &mut F,
    on_tool_call: &mut T,
) -> Result<(), ClaudeCliError>
where
    F: FnMut(&str) -> Result<(), ClaudeCliError>,
    T: FnMut(StreamToolCall) -> Result<(), ClaudeCliError>,
{
    for tool_call in extract_tool_calls(raw_line) {
        let key = format!("{}:{}", tool_call.name, tool_call.input);
        if seen_tool_calls.insert(key) {
            on_tool_call(tool_call)?;
        }
    }

    if let Some(text) = extract_stream_text(raw_line) {
        let (chunk, next_accumulated) = if text.starts_with(accumulated.as_str()) {
            (text[accumulated.len()..].to_string(), text)
        } else {
            let mut next = accumulated.clone();
            next.push_str(&text);
            (text, next)
        };
        if !chunk.is_empty() {
            on_chunk(&chunk)?;
        }
        *accumulated = next_accumulated;
    } else if serde_json::from_str::<serde_json::Value>(raw_line).is_err() {
        let cleaned = clean_response(raw_line);
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

    Ok(())
}

fn extract_tool_calls(raw_line: &str) -> Vec<StreamToolCall> {
    let value: serde_json::Value = match serde_json::from_str(raw_line) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let mut calls = Vec::new();
    collect_tool_calls(&value, &mut calls);
    calls
}

fn collect_tool_calls(value: &serde_json::Value, calls: &mut Vec<StreamToolCall>) {
    match value {
        serde_json::Value::Object(map) => {
            let is_tool_use = map
                .get("type")
                .and_then(|v| v.as_str())
                .is_some_and(|kind| kind == "tool_use" || kind == "server_tool_use");
            if is_tool_use {
                if let Some(name) = map.get("name").and_then(|v| v.as_str()) {
                    let input = map
                        .get("input")
                        .map(|v| {
                            if let Some(s) = v.as_str() {
                                s.to_string()
                            } else {
                                v.to_string()
                            }
                        })
                        .unwrap_or_default();
                    calls.push(StreamToolCall {
                        name: name.to_string(),
                        input,
                    });
                }
            }

            if let Some(content_block) = map.get("content_block") {
                collect_tool_calls(content_block, calls);
            }
            if let Some(message) = map.get("message") {
                collect_tool_calls(message, calls);
            }
            if let Some(content) = map.get("content") {
                collect_tool_calls(content, calls);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_tool_calls(item, calls);
            }
        }
        _ => {}
    }
}

fn extract_stream_text(raw_line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(raw_line).ok()?;

    // Skip non-content event types — these describe tool calls, not visible text output.
    // Without this, tool call JSON blobs get sent as text chunks and pollute the agent's reply.
    if value
        .get("type")
        .and_then(|v| v.as_str())
        .is_some_and(|kind| {
            matches!(
                kind,
                "result"
                    | "system"
                    | "tool_use"
                    | "input_required"
                    | "hook"
                    | "usage"
                    | "ping"
                    | "error"
            )
        })
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

/// Detect context overflow responses from Claude CLI
/// These come back as normal response text, not as errors
fn is_context_overflow_text(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("prompt is too long")
        || lower.contains("prompt_too_long")
        || lower.contains("context length exceeded")
        || lower.contains("context window exceeded")
        || lower.contains("too many tokens")
        || lower.contains("token limit exceeded")
        || lower.contains("request too large")
        // Also detect very short error-like responses that indicate overflow
        || (text.len() < 100 && lower.contains("too long"))
}

fn resolve_launcher_command(config: &AgentConfig, default_env_file: &str) -> String {
    // 1. 环境变量 ACP_CLAUDE_LAUNCHER 最高优先级
    if let Some(explicit) = std::env::var("ACP_CLAUDE_LAUNCHER")
        .ok()
        .filter(|v| !v.trim().is_empty())
    {
        return explicit;
    }

    // 2. AgentConfig 中的 claude_launcher 字段（显式指定）
    if let Some(launcher) = config.claude_launcher.as_deref() {
        let trimmed = launcher.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    // 3. 从 claude_env_file 推断 launcher
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
        // First try killing the process group (negative PID = kill all child processes)
        // This is critical because the Claude launcher runs in a bash -c subshell,
        // and the actual claude-haha process may not be a direct child of our bash
        let pgid_result = Command::new("kill")
            .args(["--", &format!("-{}", pid)])
            .status();

        if pgid_result.is_ok() && pgid_result.as_ref().map_or(false, |s| s.success()) {
            log::info!("[terminate_session] Killed process group for PID {}", pid);
            return Ok(());
        }

        // Fallback: try direct kill
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
