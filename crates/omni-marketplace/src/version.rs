use anyhow::{bail, Context, Result};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityMatrix {
    pub core_version: String,
    pub entries: Vec<CompatibilityEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityEntry {
    pub plugin_name: String,
    pub plugin_version: String,
    pub min_core_version: String,
    pub max_core_version: Option<String>,
    pub status: CompatibilityStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompatibilityStatus {
    Compatible,
    Degraded(String),
    Incompatible,
    Untested,
}

impl CompatibilityMatrix {
    pub fn new(core_version: &str) -> Self {
        Self {
            core_version: core_version.to_string(),
            entries: Vec::new(),
        }
    }

    pub fn check_plugin(&self, plugin_name: &str, plugin_version: &str) -> CompatibilityStatus {
        for entry in &self.entries {
            if entry.plugin_name == plugin_name && entry.plugin_version == plugin_version {
                return entry.status.clone();
            }
        }
        CompatibilityStatus::Untested
    }

    pub fn validate_install(
        &self,
        plugin_name: &str,
        plugin_version: &str,
        min_core: &str,
        max_core: Option<&str>,
    ) -> Result<()> {
        let core_ver: Version = self.core_version.parse().context("invalid core version")?;
        let min_ver: Version = min_core.parse().context("invalid min version")?;

        if core_ver < min_ver {
            bail!(
                "plugin {plugin_name}@{plugin_version} requires OmniAGP >= {min_core}, current is {}",
                self.core_version
            );
        }

        if let Some(max) = max_core {
            let max_ver: Version = max.parse().context("invalid max version")?;
            if core_ver > max_ver {
                bail!(
                    "plugin {plugin_name}@{plugin_version} requires OmniAGP <= {max}, current is {}",
                    self.core_version
                );
            }
        }

        Ok(())
    }
}

pub fn resolve_dependency_tree(
    requirements: &HashMap<String, String>,
    available: &HashMap<String, Vec<String>>,
) -> Result<HashMap<String, String>> {
    let mut resolved = HashMap::new();

    for (name, version_req_str) in requirements {
        let req: VersionReq = version_req_str
            .parse()
            .with_context(|| format!("invalid version requirement for {name}"))?;

        let versions = available
            .get(name)
            .with_context(|| format!("package {name} not found in registry"))?;

        let matching: Vec<&String> = versions
            .iter()
            .filter(|v| {
                v.parse::<Version>()
                    .map(|ver| req.matches(&ver))
                    .unwrap_or(false)
            })
            .collect();

        if matching.is_empty() {
            bail!("no compatible version found for {name} {version_req_str}");
        }

        let best = matching
            .iter()
            .max_by(|a, b| {
                let va: Version = a.parse().unwrap();
                let vb: Version = b.parse().unwrap();
                va.cmp(&vb)
            })
            .unwrap();

        resolved.insert(name.clone(), (*best).clone());
    }

    Ok(resolved)
}
