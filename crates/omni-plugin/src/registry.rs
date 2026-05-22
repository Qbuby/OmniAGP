use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::lifecycle::PluginInstance;
use crate::manifest::PluginManifest;
use crate::traits::PluginType;

pub struct PluginRegistry {
    plugins_dir: PathBuf,
    data_dir: PathBuf,
    plugins: HashMap<String, PluginInstance>,
}

impl PluginRegistry {
    pub fn new(plugins_dir: PathBuf, data_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            data_dir,
            plugins: HashMap::new(),
        }
    }

    pub fn install(&mut self, manifest: PluginManifest, source_path: &Path) -> Result<()> {
        let plugin_id = format!("{}@{}", manifest.name, manifest.version);
        if self.plugins.contains_key(&plugin_id) {
            bail!("plugin {} is already installed", plugin_id);
        }

        let install_path = self.plugins_dir.join(&manifest.name).join(&manifest.version);
        std::fs::create_dir_all(&install_path)
            .context("failed to create plugin install directory")?;

        copy_dir_recursive(source_path, &install_path)?;

        let data_dir = self.data_dir.join(&manifest.name);
        std::fs::create_dir_all(&data_dir).context("failed to create plugin data directory")?;

        let instance = PluginInstance::new(manifest.clone(), install_path, data_dir);
        self.plugins.insert(plugin_id.clone(), instance);
        info!(plugin = %plugin_id, "plugin installed");
        Ok(())
    }

    pub fn uninstall(&mut self, name: &str, version: &str) -> Result<()> {
        let plugin_id = format!("{}@{}", name, version);
        let instance = self
            .plugins
            .remove(&plugin_id)
            .context("plugin not found")?;

        if std::fs::metadata(&instance.install_path).is_ok() {
            std::fs::remove_dir_all(&instance.install_path)?;
        }
        info!(plugin = %plugin_id, "plugin uninstalled");
        Ok(())
    }

    pub fn get(&self, name: &str, version: &str) -> Option<&PluginInstance> {
        let plugin_id = format!("{}@{}", name, version);
        self.plugins.get(&plugin_id)
    }

    pub fn get_mut(&mut self, name: &str, version: &str) -> Option<&mut PluginInstance> {
        let plugin_id = format!("{}@{}", name, version);
        self.plugins.get_mut(&plugin_id)
    }

    pub fn list_by_type(&self, plugin_type: &PluginType) -> Vec<&PluginInstance> {
        self.plugins
            .values()
            .filter(|p| &p.manifest.plugin_type == plugin_type)
            .collect()
    }

    pub fn list_all(&self) -> Vec<&PluginInstance> {
        self.plugins.values().collect()
    }

    pub fn check_compatibility(&self, manifest: &PluginManifest, core_version: &str) -> Result<()> {
        let core_ver: semver::Version = core_version.parse().context("invalid core version")?;
        let min_ver: semver::Version = manifest
            .min_omniagp_version
            .parse()
            .context("invalid min version in manifest")?;

        if core_ver < min_ver {
            bail!(
                "plugin requires OmniAGP >= {}, current is {}",
                min_ver,
                core_ver
            );
        }

        if let Some(ref max) = manifest.max_omniagp_version {
            let max_ver: semver::Version = max.parse().context("invalid max version in manifest")?;
            if core_ver > max_ver {
                bail!(
                    "plugin requires OmniAGP <= {}, current is {}",
                    max_ver,
                    core_ver
                );
            }
        }

        Ok(())
    }

    pub fn resolve_dependencies(&self, manifest: &PluginManifest) -> Result<Vec<String>> {
        let mut missing = Vec::new();
        for (dep_name, dep_version_req) in &manifest.dependencies {
            let req: semver::VersionReq =
                dep_version_req.parse().context("invalid dependency version requirement")?;
            let found = self.plugins.values().any(|p| {
                p.manifest.name == *dep_name
                    && p.manifest
                        .version
                        .parse::<semver::Version>()
                        .map(|v| req.matches(&v))
                        .unwrap_or(false)
            });
            if !found {
                missing.push(format!("{} {}", dep_name, dep_version_req));
            }
        }
        if !missing.is_empty() {
            bail!("missing dependencies: {}", missing.join(", "));
        }
        Ok(missing)
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}
