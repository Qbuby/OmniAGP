use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;

use crate::manifest::PluginManifest;
use crate::traits::PluginContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginState {
    Installed,
    Activated,
    Deactivated,
    Failed(String),
}

pub struct PluginInstance {
    pub manifest: PluginManifest,
    pub state: PluginState,
    pub install_path: PathBuf,
    pub data_dir: PathBuf,
    pub config: HashMap<String, serde_json::Value>,
}

impl PluginInstance {
    pub fn new(manifest: PluginManifest, install_path: PathBuf, data_dir: PathBuf) -> Self {
        Self {
            manifest,
            state: PluginState::Installed,
            install_path,
            data_dir,
            config: HashMap::new(),
        }
    }

    pub fn activate(&mut self, project_dir: &PathBuf) -> Result<()> {
        if self.state != PluginState::Installed && self.state != PluginState::Deactivated {
            bail!("plugin must be in Installed or Deactivated state to activate");
        }
        let _ctx = PluginContext {
            project_dir: project_dir.clone(),
            plugin_data_dir: self.data_dir.clone(),
            config: self.config.clone(),
        };
        self.state = PluginState::Activated;
        info!(plugin = %self.manifest.name, "plugin activated");
        Ok(())
    }

    pub fn deactivate(&mut self) -> Result<()> {
        if self.state != PluginState::Activated {
            bail!("plugin must be in Activated state to deactivate");
        }
        self.state = PluginState::Deactivated;
        info!(plugin = %self.manifest.name, "plugin deactivated");
        Ok(())
    }

    pub fn set_config(&mut self, key: String, value: serde_json::Value) {
        self.config.insert(key, value);
    }
}
