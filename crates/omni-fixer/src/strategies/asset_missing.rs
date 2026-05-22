use anyhow::Result;
use async_trait::async_trait;
use omni_llm::{ChatMessage, ChatRequest, LlmClient, Role};
use tracing::info;

use crate::types::{BugReport, DegradationLevel, Patch};
use super::FixStrategy;

pub struct AssetMissingStrategy {
    llm: LlmClient,
    model: String,
}

impl AssetMissingStrategy {
    pub fn new(llm: LlmClient, model: String) -> Self {
        Self { llm, model }
    }
}

#[async_trait]
impl FixStrategy for AssetMissingStrategy {
    fn name(&self) -> &str {
        "asset_missing"
    }

    async fn generate_fix(
        &self,
        bug: &BugReport,
        level: DegradationLevel,
        _previous_attempts: &[Patch],
    ) -> Result<Patch> {
        info!(bug_id = %bug.id, level = ?level, "generating asset missing fix");

        let missing_asset = bug.context.get("missing_asset")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_asset");

        let asset_type = bug.context.get("asset_type")
            .and_then(|v| v.as_str())
            .unwrap_or("texture");

        match level {
            DegradationLevel::Retry | DegradationLevel::AugmentedContext => {
                let request = ChatRequest {
                    model: self.model.clone(),
                    messages: vec![
                        ChatMessage {
                            role: Role::System,
                            content: "You are a Godot 4 project expert. Generate a placeholder resource \
                                     definition or fix the resource path reference. Output ONLY the corrected code."
                                .to_string(),
                        },
                        ChatMessage {
                            role: Role::User,
                            content: format!(
                                "Missing asset: {}\nType: {}\nReferenced in: {}\nError: {}\n\n\
                                 Generate code that either:\n\
                                 1. Creates a placeholder resource inline\n\
                                 2. Fixes the resource path\n\
                                 3. Adds a null check with fallback",
                                missing_asset,
                                asset_type,
                                bug.related_files.first().map(|s| s.as_str()).unwrap_or("unknown"),
                                bug.error_message.as_deref().unwrap_or("resource not found"),
                            ),
                        },
                    ],
                    temperature: Some(0.1),
                    max_tokens: Some(2048),
                };

                let response = self.llm.chat(&request).await?;
                let fixed_code = response.choices[0].message.content.trim().to_string();

                Ok(Patch {
                    file_path: bug.related_files.first().cloned().unwrap_or_default(),
                    original: bug.context.get("source_code")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    modified: fixed_code,
                    description: format!("Asset missing fix: regenerate/placeholder for {}", missing_asset),
                })
            }
            DegradationLevel::TemplateFallback => {
                let placeholder = generate_placeholder_resource(asset_type);
                let file_path = bug.related_files.first().cloned().unwrap_or_default();
                let original = bug.context.get("source_code")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let modified = original.replace(
                    &format!("preload(\"{}\")", missing_asset),
                    &placeholder,
                );

                Ok(Patch {
                    file_path,
                    original,
                    modified,
                    description: format!("Template fallback: placeholder for {}", missing_asset),
                })
            }
            DegradationLevel::Escalate => {
                Ok(Patch {
                    file_path: bug.related_files.first().cloned().unwrap_or_default(),
                    original: String::new(),
                    modified: String::new(),
                    description: format!("ESCALATE: Cannot resolve missing asset '{}'", missing_asset),
                })
            }
        }
    }
}

fn generate_placeholder_resource(asset_type: &str) -> String {
    match asset_type {
        "texture" | "sprite" => "PlaceholderTexture2D.new()".to_string(),
        "mesh" => "BoxMesh.new()".to_string(),
        "material" => "StandardMaterial3D.new()".to_string(),
        "audio" | "sound" => "AudioStream.new()".to_string(),
        "scene" | "packed_scene" => "PackedScene.new()".to_string(),
        "font" => "SystemFont.new()".to_string(),
        _ => "Resource.new()".to_string(),
    }
}
