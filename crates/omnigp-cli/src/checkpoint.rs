use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub description: String,
    pub description_hash: String,
    pub platform: String,
    pub quality: String,
    pub completed_stages: Vec<String>,
    pub current_stage: Option<String>,
    pub stage_outputs: serde_json::Value,
    pub config_snapshot: serde_json::Value,
}

impl Checkpoint {
    pub fn new(description: &str, platform: &str, quality: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            description: description.to_string(),
            description_hash: hash_description(description),
            platform: platform.to_string(),
            quality: quality.to_string(),
            completed_stages: Vec::new(),
            current_stage: None,
            stage_outputs: serde_json::json!({}),
            config_snapshot: serde_json::json!({}),
        }
    }

    pub fn checkpoint_dir(output_dir: &Path) -> PathBuf {
        output_dir.join(".omnigp_checkpoint")
    }

    pub fn save(&self, output_dir: &Path) -> Result<()> {
        let dir = Self::checkpoint_dir(output_dir);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("checkpoint.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn load(output_dir: &Path) -> Result<Option<Self>> {
        let path = Self::checkpoint_dir(output_dir).join("checkpoint.json");
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read checkpoint: {}", path.display()))?;
        let checkpoint: Self = serde_json::from_str(&content)?;
        Ok(Some(checkpoint))
    }

    pub fn mark_stage_complete(&mut self, stage: &str, output: serde_json::Value) {
        self.completed_stages.push(stage.to_string());
        self.current_stage = None;
        if let Some(map) = self.stage_outputs.as_object_mut() {
            map.insert(stage.to_string(), output);
        }
    }

    pub fn set_current_stage(&mut self, stage: &str) {
        self.current_stage = Some(stage.to_string());
    }

    pub fn is_stage_complete(&self, stage: &str) -> bool {
        self.completed_stages.contains(&stage.to_string())
    }

    pub fn should_regenerate(&self, new_description: &str) -> bool {
        hash_description(new_description) != self.description_hash
    }

    pub fn cleanup(output_dir: &Path) -> Result<()> {
        let dir = Self::checkpoint_dir(output_dir);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }
}

fn hash_description(description: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(description.as_bytes());
    hex::encode(hasher.finalize())
}
