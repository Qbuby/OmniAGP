use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::pipeline_2d::{Asset2DClient, AssetCategory, Generate2DRequest, StylePreset};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    Sprite2D,
    Texture,
    Tileset,
    Icon,
    Model3D,
    Audio,
    Music,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRequest {
    pub asset_type: AssetType,
    pub prompt: String,
    pub output_path: String,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub tile_size: Option<u32>,
    #[serde(default)]
    pub reference_image_b64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetResult {
    pub request: AssetRequest,
    pub file_paths: Vec<String>,
    pub success: bool,
    pub error: Option<String>,
    pub generation_time_ms: Option<u64>,
}

pub async fn generate_asset(client: &Asset2DClient, request: AssetRequest) -> AssetResult {
    let (style, category) = match &request.asset_type {
        AssetType::Sprite2D | AssetType::Texture => {
            let style = match request.style.as_deref() {
                Some("anime") => StylePreset::Anime,
                Some("realistic") => StylePreset::Realistic,
                _ => StylePreset::Pixel,
            };
            (style, AssetCategory::Sprite)
        }
        AssetType::Icon => {
            let style = match request.style.as_deref() {
                Some("anime") => StylePreset::Anime,
                Some("realistic") => StylePreset::Realistic,
                _ => StylePreset::Pixel,
            };
            (style, AssetCategory::Icon)
        }
        AssetType::Tileset => {
            let style = match request.style.as_deref() {
                Some("anime") => StylePreset::Anime,
                Some("realistic") => StylePreset::Realistic,
                _ => StylePreset::Pixel,
            };
            (style, AssetCategory::Tileset)
        }
        AssetType::Model3D | AssetType::Audio | AssetType::Music => {
            return AssetResult {
                request,
                file_paths: vec![],
                success: false,
                error: Some("3D/Audio generation not yet implemented".into()),
                generation_time_ms: None,
            };
        }
    };

    let gen_req = Generate2DRequest {
        prompt: request.prompt.clone(),
        negative_prompt: "blurry, low quality, watermark, text, signature".into(),
        style,
        category,
        width: request.width.unwrap_or(1024),
        height: request.height.unwrap_or(1024),
        remove_background: true,
        tile_size: request.tile_size,
        seed: -1,
        steps: 25,
        cfg_scale: 7.0,
        reference_image_b64: request.reference_image_b64.clone(),
    };

    match client.generate(&gen_req).await {
        Ok(resp) => {
            let file_paths: Vec<String> = resp
                .assets
                .iter()
                .filter_map(|a| a.file_path.clone())
                .collect();
            AssetResult {
                request,
                file_paths,
                success: resp.status == "success",
                error: if resp.errors.is_empty() {
                    None
                } else {
                    Some(resp.errors.join("; "))
                },
                generation_time_ms: Some(resp.generation_time_ms),
            }
        }
        Err(e) => AssetResult {
            request,
            file_paths: vec![],
            success: false,
            error: Some(e.to_string()),
            generation_time_ms: None,
        },
    }
}
