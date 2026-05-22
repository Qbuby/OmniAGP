use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::PluginConfigSchema;
use crate::traits::PluginType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub license: String,
    pub plugin_type: PluginType,
    pub entry_point: String,
    pub min_omniagp_version: String,
    pub max_omniagp_version: Option<String>,
    pub dependencies: HashMap<String, String>,
    pub config_schema: Vec<PluginConfigSchema>,
    pub permissions: PluginPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPermissions {
    pub filesystem: FilesystemPermission,
    pub network: NetworkPermission,
    pub max_memory_mb: u64,
    pub max_cpu_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemPermission {
    pub read_paths: Vec<String>,
    pub write_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPermission {
    pub allowed_hosts: Vec<String>,
    pub allow_outbound: bool,
}

impl Default for PluginPermissions {
    fn default() -> Self {
        Self {
            filesystem: FilesystemPermission {
                read_paths: vec!["$PROJECT_DIR".into()],
                write_paths: vec!["$PLUGIN_DATA_DIR".into()],
            },
            network: NetworkPermission {
                allowed_hosts: vec![],
                allow_outbound: false,
            },
            max_memory_mb: 512,
            max_cpu_seconds: 60,
        }
    }
}
