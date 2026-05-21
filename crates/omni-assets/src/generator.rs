use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetType {
    Sprite2D,
    Texture,
    Model3D,
    Audio,
    Music,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRequest {
    pub asset_type: AssetType,
    pub prompt: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetResult {
    pub request: AssetRequest,
    pub file_path: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

pub async fn generate_asset(_request: AssetRequest) -> AssetResult {
    // Placeholder — will integrate with SDXL/ComfyUI/MusicGen
    AssetResult {
        request: _request.clone(),
        file_path: None,
        success: false,
        error: Some("Asset generation not yet implemented".into()),
    }
}
