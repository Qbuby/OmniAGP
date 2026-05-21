use anyhow::Result;
use reqwest::Client;
use tracing::info;

use crate::types::{ChatRequest, ChatResponse};

pub struct LlmClient {
    http: Client,
    base_url: String,
    api_key: String,
}

impl LlmClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            http: Client::new(),
            base_url,
            api_key,
        }
    }

    pub fn from_env(base_url_env: &str, api_key_env: &str) -> Result<Self> {
        let base_url = std::env::var(base_url_env)
            .unwrap_or_else(|_| "http://localhost:11434/v1".to_string());
        let api_key = std::env::var(api_key_env).unwrap_or_default();
        Ok(Self::new(base_url, api_key))
    }

    pub async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        info!(model = %request.model, messages = request.messages.len(), "sending chat request");

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await?
            .error_for_status()?
            .json::<ChatResponse>()
            .await?;

        info!(tokens = response.usage.total_tokens, "chat response received");
        Ok(response)
    }
}
