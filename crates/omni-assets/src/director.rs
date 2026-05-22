use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignDocRequest {
    pub design_doc: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorResponse {
    pub total_tasks: u32,
    pub succeeded: u32,
    pub failed: u32,
    pub failures: Vec<DirectorFailure>,
    pub asset_registry: HashMap<String, AssetRegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorFailure {
    pub id: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRegistryEntry {
    pub category: String,
    pub prompt: String,
    pub file_path: String,
    pub metadata: serde_json::Value,
}

pub struct AssetDirectorClient {
    client: reqwest::Client,
    base_url: String,
}

impl AssetDirectorClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("ASSET_DIRECTOR_URL")
            .unwrap_or_else(|_| "http://localhost:8092".to_string());
        Self::new(&url)
    }

    pub async fn health(&self) -> Result<bool> {
        let resp = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await?;
        Ok(resp.status().is_success())
    }

    pub async fn execute_design_doc(
        &self,
        design_doc: serde_json::Value,
    ) -> Result<DirectorResponse> {
        let request = DesignDocRequest { design_doc };

        let resp = self
            .client
            .post(format!("{}/execute", self.base_url))
            .json(&request)
            .send()
            .await
            .context("failed to reach asset director")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("asset director returned {}: {}", status, body);
        }

        let result: DirectorResponse = resp
            .json()
            .await
            .context("failed to parse asset director response")?;
        Ok(result)
    }
}
