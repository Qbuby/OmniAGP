use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfigSchema {
    pub key: String,
    pub label: String,
    pub description: String,
    pub field_type: ConfigFieldType,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConfigFieldType {
    #[serde(rename = "string")]
    String { min_length: Option<usize>, max_length: Option<usize> },
    #[serde(rename = "number")]
    Number { min: Option<f64>, max: Option<f64> },
    #[serde(rename = "integer")]
    Integer { min: Option<i64>, max: Option<i64> },
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "select")]
    Select { options: Vec<SelectOption> },
    #[serde(rename = "color")]
    Color,
    #[serde(rename = "file_path")]
    FilePath { extensions: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}
