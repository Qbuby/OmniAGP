use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioType {
    Bgm,
    Sfx,
}

impl std::fmt::Display for AudioType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioType::Bgm => write!(f, "bgm"),
            AudioType::Sfx => write!(f, "sfx"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioRequest {
    pub prompt: String,
    pub audio_type: String,
    pub duration_sec: Option<f64>,
    pub output_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioValidation {
    pub valid: bool,
    pub duration_sec: f64,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioResponse {
    pub file_path: String,
    pub audio_type: String,
    pub duration_sec: f64,
    pub sample_rate: u32,
    pub loop_point_samples: Option<u64>,
    pub validation: AudioValidation,
}

pub struct AudioClient {
    client: reqwest::Client,
    base_url: String,
}

impl AudioClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("AUDIO_PIPELINE_URL")
            .unwrap_or_else(|_| "http://localhost:8090".to_string());
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

    pub async fn generate(
        &self,
        prompt: &str,
        audio_type: AudioType,
        duration_sec: Option<f64>,
        output_dir: Option<&str>,
    ) -> Result<AudioResponse> {
        let request = AudioRequest {
            prompt: prompt.to_string(),
            audio_type: audio_type.to_string(),
            duration_sec,
            output_dir: output_dir.map(|s| s.to_string()),
        };

        let resp = self
            .client
            .post(format!("{}/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .context("failed to reach audio pipeline")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("audio pipeline returned {}: {}", status, body);
        }

        let result: AudioResponse = resp.json().await.context("failed to parse audio response")?;
        Ok(result)
    }
}
