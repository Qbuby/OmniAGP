use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::types::{TemplateInstance, TemplateManifest, TemplateParam};

pub struct TemplateEngine {
    templates_dir: PathBuf,
    templates: HashMap<String, TemplateManifest>,
}

impl TemplateEngine {
    pub fn new(templates_dir: PathBuf) -> Self {
        Self {
            templates_dir,
            templates: HashMap::new(),
        }
    }

    pub fn load_templates(&mut self) -> Result<()> {
        if !self.templates_dir.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(&self.templates_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let manifest_path = entry.path().join("template.toml");
            if manifest_path.exists() {
                let content = std::fs::read_to_string(&manifest_path)?;
                let manifest: TemplateManifest =
                    toml::from_str(&content).context("failed to parse template.toml")?;
                info!(template = %manifest.id, "loaded template");
                self.templates.insert(manifest.id.clone(), manifest);
            }
        }
        Ok(())
    }

    pub fn get_template(&self, id: &str) -> Option<&TemplateManifest> {
        self.templates.get(id)
    }

    pub fn list_templates(&self) -> Vec<&TemplateManifest> {
        self.templates.values().collect()
    }

    pub fn instantiate(
        &self,
        template_id: &str,
        params: HashMap<String, serde_json::Value>,
        output_dir: &Path,
    ) -> Result<TemplateInstance> {
        let manifest = self
            .templates
            .get(template_id)
            .context("template not found")?;

        self.validate_params(&manifest.params, &params)?;

        let template_dir = self.templates_dir.join(template_id);
        self.copy_template_files(&template_dir, output_dir, &params)?;

        let gdd_path = template_dir.join(&manifest.gdd_template);
        if gdd_path.exists() {
            let gdd_content = std::fs::read_to_string(&gdd_path)?;
            let rendered = self.render_template_string(&gdd_content, &params);
            std::fs::write(output_dir.join("gdd.json"), rendered)?;
        }

        info!(template = %template_id, "template instantiated");
        Ok(TemplateInstance {
            template_id: template_id.to_string(),
            params,
        })
    }

    fn validate_params(
        &self,
        schema: &[TemplateParam],
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        for param_def in schema {
            if param_def.required && !params.contains_key(&param_def.key) {
                if param_def.default_value.is_none() {
                    bail!("required parameter '{}' is missing", param_def.key);
                }
            }
        }
        Ok(())
    }

    fn copy_template_files(
        &self,
        src: &Path,
        dst: &Path,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        std::fs::create_dir_all(dst)?;

        let assets_dir = src.join("assets");
        if assets_dir.exists() {
            let dst_assets = dst.join("assets");
            copy_dir_recursive(&assets_dir, &dst_assets)?;
        }

        let scripts_dir = src.join("scripts");
        if scripts_dir.exists() {
            let dst_scripts = dst.join("scripts");
            std::fs::create_dir_all(&dst_scripts)?;
            for entry in std::fs::read_dir(&scripts_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    let content = std::fs::read_to_string(entry.path())?;
                    let rendered = self.render_template_string(&content, params);
                    std::fs::write(dst_scripts.join(entry.file_name()), rendered)?;
                }
            }
        }

        Ok(())
    }

    fn render_template_string(
        &self,
        template: &str,
        params: &HashMap<String, serde_json::Value>,
    ) -> String {
        let mut result = template.to_string();
        for (key, value) in params {
            let placeholder = format!("{{{{{}}}}}", key);
            let replacement = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }
        result
    }

    pub fn get_inherited_manifest(&self, template_id: &str) -> Result<TemplateManifest> {
        let manifest = self
            .templates
            .get(template_id)
            .context("template not found")?
            .clone();

        if let Some(ref parent_id) = manifest.inherits {
            let parent = self
                .templates
                .get(parent_id)
                .context("parent template not found")?;

            let mut merged = parent.clone();
            merged.id = manifest.id;
            merged.name = manifest.name;
            merged.description = manifest.description;
            merged.version = manifest.version;

            for param in manifest.params {
                if let Some(existing) = merged.params.iter_mut().find(|p| p.key == param.key) {
                    *existing = param;
                } else {
                    merged.params.push(param);
                }
            }

            merged.assets.extend(manifest.assets);
            merged.scripts.extend(manifest.scripts);

            return Ok(merged);
        }

        Ok(manifest)
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dest_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}
