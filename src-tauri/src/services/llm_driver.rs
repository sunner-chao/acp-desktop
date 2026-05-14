use crate::models::{ACPContent, AgentConfig};
use reqwest::Client;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LLMError {
    #[error("Request failed: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("API error: {0}")]
    ApiError(String),
}

pub struct LLMDriver {
    client: Client,
}

impl LLMDriver {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn chat(
        &self,
        config: &AgentConfig,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, LLMError> {
        let api_format = config.api_format.as_deref().unwrap_or("openai");

        match api_format {
            "anthropic" => {
                self.chat_anthropic(config, system_prompt, user_message)
                    .await
            }
            _ => self.chat_openai(config, system_prompt, user_message).await,
        }
    }

    async fn chat_openai(
        &self,
        config: &AgentConfig,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, LLMError> {
        let endpoint = config
            .endpoint
            .as_deref()
            .unwrap_or("https://api.openai.com/v1/chat/completions");
        let api_key = config.api_key.as_deref().unwrap_or("");
        let model = config.model.as_deref().unwrap_or("gpt-3.5-turbo");
        let temperature = config.temperature.unwrap_or(0.7);
        let max_tokens = config.max_tokens.unwrap_or(4096);

        let request_body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_message}
            ],
            "temperature": temperature,
            "max_tokens": max_tokens,
        });

        let response = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError(format!(
                "Status {}: {}",
                status, error_body
            )));
        }

        let response_json: Value = response.json().await?;

        let content = response_json["choices"]
            .get(0)
            .and_then(|c| c["message"]["content"].as_str())
            .ok_or_else(|| LLMError::InvalidResponse("Missing content in response".to_string()))?
            .to_string();

        Ok(content)
    }

    async fn chat_anthropic(
        &self,
        config: &AgentConfig,
        system_prompt: &str,
        user_message: &str,
    ) -> Result<String, LLMError> {
        let endpoint = config
            .endpoint
            .as_deref()
            .unwrap_or("https://api.anthropic.com/v1/messages");
        let api_key = config.api_key.as_deref().unwrap_or("");
        let model = config.model.as_deref().unwrap_or("claude-3-haiku-20240307");
        let temperature = config.temperature.unwrap_or(0.7);
        let max_tokens = config.max_tokens.unwrap_or(4096);

        let request_body = serde_json::json!({
            "model": model,
            "system": system_prompt,
            "messages": [
                {"role": "user", "content": user_message}
            ],
            "temperature": temperature,
            "max_tokens": max_tokens,
        });

        let response = self
            .client
            .post(endpoint)
            .header("x-api-key", api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError(format!(
                "Status {}: {}",
                status, error_body
            )));
        }

        let response_json: Value = response.json().await?;

        let content = response_json["content"]
            .get(0)
            .and_then(|c| c["text"].as_str())
            .ok_or_else(|| LLMError::InvalidResponse("Missing content in response".to_string()))?
            .to_string();

        Ok(content)
    }

    pub fn process_message(&self, _content: ACPContent) -> ACPContent {
        // Placeholder for message processing
        // In a real implementation, this would route messages based on content
        ACPContent {
            action: Some("process".to_string()),
            result: Some(serde_json::json!({"status": "processed"})),
            ..Default::default()
        }
    }
}

impl Default for LLMDriver {
    fn default() -> Self {
        Self::new()
    }
}
