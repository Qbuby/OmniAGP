use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::InitArgs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmnigpConfig {
    pub llm: LlmConfig,
    pub gpu: GpuConfig,
    pub output: OutputConfig,
    pub pipeline: PipelineSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,
    pub api_key_env: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    pub enabled: bool,
    pub device: String,
    pub vram_limit_mb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub format: String,
    pub godot_export_template: String,
    pub include_source: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSettings {
    pub parallel_assets: bool,
    pub max_retries: u32,
    pub qa_iterations: u32,
    pub checkpoint_enabled: bool,
}

impl Default for OmnigpConfig {
    fn default() -> Self {
        Self {
            llm: LlmConfig {
                base_url: "http://localhost:11434/v1".into(),
                model: "qwen2.5-coder:14b".into(),
                api_key_env: "OMNIGP_API_KEY".into(),
                max_tokens: 8192,
                temperature: 0.3,
            },
            gpu: GpuConfig {
                enabled: true,
                device: "cuda:0".into(),
                vram_limit_mb: 16384,
            },
            output: OutputConfig {
                format: "windows".into(),
                godot_export_template: "".into(),
                include_source: true,
            },
            pipeline: PipelineSettings {
                parallel_assets: true,
                max_retries: 3,
                qa_iterations: 2,
                checkpoint_enabled: true,
            },
        }
    }
}

impl OmnigpConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            tracing::warn!("Config file not found at {}, using defaults", path.display());
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;
        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config: {}", path.display()))?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}

pub async fn init_config(args: InitArgs) -> Result<()> {
    let config = OmnigpConfig::default();
    config.save(&args.output)?;
    println!("Created default config at: {}", args.output.display());
    Ok(())
}
