use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PluginType {
    Style,
    PostProcess,
    Generator,
    Exporter,
}

#[derive(Debug, Clone)]
pub struct PluginContext {
    pub project_dir: PathBuf,
    pub plugin_data_dir: PathBuf,
    pub config: HashMap<String, serde_json::Value>,
}

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn plugin_type(&self) -> PluginType;
    fn activate(&mut self, ctx: &PluginContext) -> Result<()>;
    fn deactivate(&mut self) -> Result<()>;
}

pub trait StylePlugin: Plugin {
    fn apply_style(
        &self,
        input_path: &PathBuf,
        output_path: &PathBuf,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<()>;

    fn supported_formats(&self) -> Vec<String>;

    fn preview(
        &self,
        input_path: &PathBuf,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<u8>>;
}

pub trait PostProcessPlugin: Plugin {
    fn process(
        &self,
        input_path: &PathBuf,
        output_path: &PathBuf,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<()>;

    fn supported_input_types(&self) -> Vec<String>;
}

pub trait GeneratorPlugin: Plugin {
    fn generate(
        &self,
        output_dir: &PathBuf,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<PathBuf>>;

    fn generator_type(&self) -> &str;
}

pub trait ExporterPlugin: Plugin {
    fn export(
        &self,
        project_dir: &PathBuf,
        output_path: &PathBuf,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<()>;

    fn export_format(&self) -> &str;
}
