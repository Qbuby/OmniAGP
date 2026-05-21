use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StylePreset {
    Pixel,
    Anime,
    Realistic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetCategory {
    Sprite,
    Icon,
    Tileset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generate2DRequest {
    pub prompt: String,
    #[serde(default = "default_negative_prompt")]
    pub negative_prompt: String,
    #[serde(default = "default_style")]
    pub style: StylePreset,
    #[serde(default = "default_category")]
    pub category: AssetCategory,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_true")]
    pub remove_background: bool,
    pub tile_size: Option<u32>,
    #[serde(default = "default_seed")]
    pub seed: i64,
    #[serde(default = "default_steps")]
    pub steps: u32,
    #[serde(default = "default_cfg")]
    pub cfg_scale: f64,
    pub reference_image_b64: Option<String>,
}

fn default_negative_prompt() -> String {
    "blurry, low quality, watermark, text, signature".into()
}
fn default_style() -> StylePreset {
    StylePreset::Pixel
}
fn default_category() -> AssetCategory {
    AssetCategory::Sprite
}
fn default_width() -> u32 {
    1024
}
fn default_height() -> u32 {
    1024
}
fn default_true() -> bool {
    true
}
fn default_seed() -> i64 {
    -1
}
fn default_steps() -> u32 {
    25
}
fn default_cfg() -> f64 {
    7.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetOutput {
    pub file_path: String,
    pub width: u32,
    pub height: u32,
    pub has_alpha: bool,
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Generate2DResponse {
    pub request_id: String,
    pub status: String,
    pub generation_time_ms: u64,
    pub assets: Vec<AssetOutput>,
    #[serde(default)]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarHealth {
    pub status: String,
    pub comfyui_connected: bool,
}

#[derive(Clone)]
pub struct Asset2DClient {
    client: Client,
    base_url: String,
}

impl Asset2DClient {
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build HTTP client");
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("ASSET_2D_URL").unwrap_or_else(|_| "http://127.0.0.1:8100".into());
        Self::new(&url)
    }

    pub async fn health(&self) -> Result<SidecarHealth> {
        let resp = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await
            .context("failed to reach 2D asset sidecar")?;
        let health: SidecarHealth = resp.json().await.context("invalid health response")?;
        Ok(health)
    }

    pub async fn generate(&self, request: &Generate2DRequest) -> Result<Generate2DResponse> {
        let resp = self
            .client
            .post(format!("{}/generate", self.base_url))
            .json(request)
            .send()
            .await
            .context("failed to call 2D asset sidecar")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("2D sidecar returned {status}: {body}");
        }

        let result: Generate2DResponse =
            resp.json().await.context("invalid generation response")?;
        Ok(result)
    }
}
