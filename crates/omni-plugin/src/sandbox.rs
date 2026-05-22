use anyhow::{bail, Result};
use std::path::PathBuf;
use tracing::warn;

use crate::manifest::PluginPermissions;

pub struct SandboxConfig {
    pub permissions: PluginPermissions,
    pub plugin_dir: PathBuf,
    pub project_dir: PathBuf,
}

pub struct Sandbox {
    config: SandboxConfig,
}

impl Sandbox {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config }
    }

    pub fn validate_file_read(&self, path: &PathBuf) -> Result<()> {
        let resolved = self.resolve_path_pattern(&self.config.permissions.filesystem.read_paths);
        if !self.path_matches(path, &resolved) {
            bail!(
                "sandbox violation: read access denied for {:?}",
                path
            );
        }
        Ok(())
    }

    pub fn validate_file_write(&self, path: &PathBuf) -> Result<()> {
        let resolved = self.resolve_path_pattern(&self.config.permissions.filesystem.write_paths);
        if !self.path_matches(path, &resolved) {
            bail!(
                "sandbox violation: write access denied for {:?}",
                path
            );
        }
        Ok(())
    }

    pub fn validate_network_access(&self, host: &str) -> Result<()> {
        if !self.config.permissions.network.allow_outbound {
            bail!("sandbox violation: network access is disabled");
        }
        if !self.config.permissions.network.allowed_hosts.is_empty()
            && !self.config.permissions.network.allowed_hosts.contains(&host.to_string())
        {
            bail!(
                "sandbox violation: network access denied for host {}",
                host
            );
        }
        Ok(())
    }

    pub fn max_memory_mb(&self) -> u64 {
        self.config.permissions.max_memory_mb
    }

    pub fn max_cpu_seconds(&self) -> u64 {
        self.config.permissions.max_cpu_seconds
    }

    fn resolve_path_pattern(&self, patterns: &[String]) -> Vec<PathBuf> {
        patterns
            .iter()
            .map(|p| {
                p.replace("$PROJECT_DIR", self.config.project_dir.to_str().unwrap_or(""))
                    .replace("$PLUGIN_DATA_DIR", self.config.plugin_dir.to_str().unwrap_or(""))
            })
            .map(PathBuf::from)
            .collect()
    }

    fn path_matches(&self, path: &PathBuf, allowed: &[PathBuf]) -> bool {
        let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
        for allowed_path in allowed {
            let allowed_canonical =
                std::fs::canonicalize(allowed_path).unwrap_or_else(|_| allowed_path.clone());
            if canonical.starts_with(&allowed_canonical) {
                return true;
            }
        }
        if allowed.is_empty() {
            warn!("no paths configured in sandbox — denying access");
        }
        false
    }
}

pub fn create_sandbox(
    permissions: PluginPermissions,
    plugin_dir: PathBuf,
    project_dir: PathBuf,
) -> Sandbox {
    Sandbox::new(SandboxConfig {
        permissions,
        plugin_dir,
        project_dir,
    })
}
