use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DriverType {
    Script,
    Llm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub driver_type: DriverType,
    pub address: String,
    pub config: AgentConfig,
    pub is_online: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub last_active: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_project_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_env_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_launcher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_enabled: Option<bool>,
}

/// 全局 Claude CLI 设置（用户可自定义）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeSettings {
    pub command: String,
    pub project_dir: String,
    pub env_file: String,
    pub entrypoint: String,
    pub default_model: String,
    pub timeout_ms: u64,
}

impl Default for ClaudeSettings {
    fn default() -> Self {
        Self::with_agent_config(None)
    }
}

impl ClaudeSettings {
    /// 从 agent 配置构建设置（可选），使用 agent 的 claudeProjectDir 优先
    pub fn with_agent_config(agent_config: Option<&AgentConfig>) -> Self {
        let timeout_ms = std::env::var("ACP_CLAUDE_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(120000);
        let agent_project_dir = agent_config
            .and_then(|c| c.claude_project_dir.as_deref())
            .filter(|v| !v.trim().is_empty());
        let project_dir = resolve_claude_project_dir(agent_project_dir);
        let env_file = resolve_claude_env_file(&project_dir);
        let entrypoint = resolve_claude_entrypoint(&project_dir);

        Self {
            command: std::env::var("ACP_CLAUDE_COMMAND").unwrap_or_else(|_| "bun".to_string()),
            project_dir,
            env_file,
            entrypoint,
            default_model: std::env::var("ACP_CLAUDE_MODEL")
                .unwrap_or_else(|_| "default".to_string()),
            timeout_ms,
        }
    }
}

fn resolve_claude_project_dir(agent_project_dir: Option<&str>) -> String {
    if let Ok(dir) = std::env::var("ACP_CLAUDE_PROJECT_DIR") {
        if !dir.trim().is_empty() {
            return canonicalize_string(&dir).unwrap_or(dir);
        }
    }

    // agent 配置的 claudeProjectDir 优先于目录扫描
    if let Some(dir) = agent_project_dir {
        return canonicalize_string(dir).unwrap_or_else(|| dir.to_string());
    }

    let candidates = [
        "../claude-code-main",
        "../../claude-code-main",
        "./claude-code-main",
        ".",
    ];
    for candidate in candidates {
        let path = std::path::Path::new(candidate);
        if has_working_claude_launcher(path) {
            return canonicalize_string(candidate).unwrap_or_else(|| candidate.to_string());
        }
    }

    ".".to_string()
}

fn resolve_claude_env_file(project_dir: &str) -> String {
    if let Ok(file) = std::env::var("ACP_CLAUDE_ENV_FILE") {
        if !file.trim().is_empty() {
            return file;
        }
    }

    let bin_dir = std::path::Path::new(project_dir).join("bin");
    if bin_dir.join("claude-haha-dsv4.cmd").exists() || bin_dir.join("claude-haha-dsv4").exists() {
        return ".env.dsv4".to_string();
    }
    if bin_dir.join("claude-haha-minimax27.cmd").exists() || bin_dir.join("claude-haha-minimax27").exists() {
        return ".env.minimax27".to_string();
    }
    if bin_dir.join("claude-haha-glm51.cmd").exists() || bin_dir.join("claude-haha-glm51").exists() {
        return ".env.glm51".to_string();
    }

    let dsv4 = std::path::Path::new(project_dir).join(".env.dsv4");
    if dsv4.exists() {
        return ".env.dsv4".to_string();
    }
    let minimax27 = std::path::Path::new(project_dir).join(".env.minimax27");
    if minimax27.exists() {
        return ".env.minimax27".to_string();
    }
    let glm51 = std::path::Path::new(project_dir).join(".env.glm51");
    if glm51.exists() {
        return ".env.glm51".to_string();
    }

    let env = std::path::Path::new(project_dir).join(".env");
    if env.exists() {
        return ".env".to_string();
    }

    ".env".to_string()
}

fn resolve_claude_entrypoint(project_dir: &str) -> String {
    if let Ok(entrypoint) = std::env::var("ACP_CLAUDE_ENTRYPOINT") {
        if !entrypoint.trim().is_empty() {
            return entrypoint;
        }
    }

    let entry = std::path::Path::new(project_dir)
        .join("src")
        .join("entrypoints")
        .join("cli.tsx");
    if entry.exists() {
        return "./src/entrypoints/cli.tsx".to_string();
    }

    "./src/entrypoints/cli.tsx".to_string()
}

fn canonicalize_string(path: &str) -> Option<String> {
    std::fs::canonicalize(path)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

fn has_working_claude_launcher(project_dir: &std::path::Path) -> bool {
    let bin_dir = project_dir.join("bin");
    [
        "claude-haha-dsv4.cmd",
        "claude-haha-dsv4",
        "claude-haha-glm51.cmd",
        "claude-haha-glm51",
        "claude-haha-minimax27.cmd",
        "claude-haha-minimax27",
        "claude-haha.cmd",
        "claude-haha",
    ]
    .iter()
    .any(|name| bin_dir.join(name).exists())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAgentInput {
    pub name: String,
    pub description: Option<String>,
    pub driver_type: DriverType,
    pub config: AgentConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAgentInput {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub config: Option<AgentConfig>,
}

impl Agent {
    pub fn new(input: CreateAgentInput) -> Self {
        let id = Uuid::new_v4().to_string();
        let address = format!("agent://local/{}", input.name);
        let created_at = chrono::Utc::now().to_rfc3339();

        Self {
            id,
            name: input.name,
            description: input.description,
            driver_type: input.driver_type,
            address,
            config: input.config,
            is_online: false,
            session_id: None,
            last_active: None,
            created_at,
        }
    }
}
