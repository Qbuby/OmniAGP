use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub category: TemplateCategory,
    pub engine: String,
    pub inherits: Option<String>,
    pub params: Vec<TemplateParam>,
    pub gdd_template: String,
    pub assets: Vec<TemplateAsset>,
    pub scripts: Vec<TemplateScript>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateCategory {
    Platformer,
    Shooter,
    Puzzle,
    VisualNovel,
    Idle,
    RPG,
    Strategy,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParam {
    pub key: String,
    pub label: String,
    pub description: String,
    pub param_type: TemplateParamType,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TemplateParamType {
    #[serde(rename = "string")]
    String,
    #[serde(rename = "integer")]
    Integer { min: Option<i64>, max: Option<i64> },
    #[serde(rename = "select")]
    Select { options: Vec<String> },
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "theme")]
    Theme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateAsset {
    pub path: String,
    pub asset_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateScript {
    pub path: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInstance {
    pub template_id: String,
    pub params: HashMap<String, serde_json::Value>,
}
