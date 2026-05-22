use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPack {
    pub id: Uuid,
    pub manifest: AssetPackManifest,
    pub download_url: String,
    pub download_count: u64,
    pub rating: f32,
    pub rating_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPackManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub asset_type: AssetType,
    pub tags: Vec<String>,
    pub license: String,
    pub resolution: Option<String>,
    pub file_count: u32,
    pub size_bytes: u64,
    pub preview_images: Vec<String>,
    pub min_omniagp_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AssetType {
    SpriteSheet,
    Model3D,
    SoundEffect,
    Music,
    UITheme,
    Tileset,
    Font,
    Shader,
    ParticleEffect,
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpriteSheet => write!(f, "sprite_sheet"),
            Self::Model3D => write!(f, "3d_model"),
            Self::SoundEffect => write!(f, "sound_effect"),
            Self::Music => write!(f, "music"),
            Self::UITheme => write!(f, "ui_theme"),
            Self::Tileset => write!(f, "tileset"),
            Self::Font => write!(f, "font"),
            Self::Shader => write!(f, "shader"),
            Self::ParticleEffect => write!(f, "particle_effect"),
        }
    }
}
