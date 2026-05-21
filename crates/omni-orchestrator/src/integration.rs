use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::blackboard::Blackboard;
use crate::coordinator::{AssetsNeeded, Coordinator, GameDesignDoc, SceneSpec};
use crate::event_bus::EventBus;
use omni_llm::LlmClient;

pub struct CompileFeedbackLoop {
    llm: LlmClient,
    blackboard: Arc<Blackboard>,
    event_bus: EventBus,
    max_fix_attempts: u32,
    project_dir: PathBuf,
}

#[derive(Debug)]
pub struct CompileResult {
    pub success: bool,
    pub output: String,
    pub errors: Vec<CompileError>,
}

#[derive(Debug, Clone)]
pub struct CompileError {
    pub file: String,
    pub line: Option<u32>,
    pub message: String,
}

impl CompileFeedbackLoop {
    pub fn new(
        llm: LlmClient,
        blackboard: Arc<Blackboard>,
        event_bus: EventBus,
        project_dir: PathBuf,
    ) -> Self {
        Self {
            llm,
            blackboard,
            event_bus,
            max_fix_attempts: 5,
            project_dir,
        }
    }

    pub async fn run_with_fixes(&self, generated_files: &serde_json::Value) -> Result<CompileResult> {
        let mut attempt = 0;
        let mut current_files = generated_files.clone();

        loop {
            attempt += 1;
            info!(attempt, "running compile check");

            let result = self.check_compile(&current_files).await?;

            if result.success {
                info!(attempt, "compile succeeded");
                self.event_bus.publish_build_succeeded("compile_loop", &self.project_dir.display().to_string());
                self.blackboard
                    .set("compile/status", &"success", "compile_loop")
                    .await?;
                return Ok(result);
            }

            if attempt >= self.max_fix_attempts {
                warn!(attempt, errors = result.errors.len(), "max fix attempts reached");
                self.event_bus.publish_build_failed(
                    "compile_loop",
                    &format!("{} errors remaining after {} attempts", result.errors.len(), attempt),
                    Uuid::new_v4(),
                );
                return Ok(result);
            }

            info!(
                attempt,
                errors = result.errors.len(),
                "compile failed, requesting fix"
            );

            current_files = self.request_fix(&current_files, &result.errors).await?;
        }
    }

    async fn check_compile(&self, files: &serde_json::Value) -> Result<CompileResult> {
        let file_list = files.get("files").and_then(|f| f.as_array());

        if let Some(files_arr) = file_list {
            let mut errors = Vec::new();
            for file in files_arr {
                let path = file.get("path").and_then(|p| p.as_str()).unwrap_or("");
                let content = file.get("content").and_then(|c| c.as_str()).unwrap_or("");

                if let Some(err) = self.validate_gdscript(path, content) {
                    errors.push(err);
                }
            }

            Ok(CompileResult {
                success: errors.is_empty(),
                output: if errors.is_empty() {
                    "All files validated successfully".to_string()
                } else {
                    format!("{} validation errors found", errors.len())
                },
                errors,
            })
        } else {
            Ok(CompileResult {
                success: false,
                output: "No files found in output".to_string(),
                errors: vec![CompileError {
                    file: "output".to_string(),
                    line: None,
                    message: "Expected 'files' array in generated output".to_string(),
                }],
            })
        }
    }

    fn validate_gdscript(&self, path: &str, content: &str) -> Option<CompileError> {
        if !path.ends_with(".gd") {
            return None;
        }

        if content.trim().is_empty() {
            return Some(CompileError {
                file: path.to_string(),
                line: Some(1),
                message: "Empty script file".to_string(),
            });
        }

        let has_extends = content.lines().any(|l| l.trim().starts_with("extends"));
        if !has_extends {
            return Some(CompileError {
                file: path.to_string(),
                line: Some(1),
                message: "GDScript file missing 'extends' declaration".to_string(),
            });
        }

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("func ") && !trimmed.ends_with(':') && !trimmed.contains("->") {
                if !trimmed.contains(':') {
                    return Some(CompileError {
                        file: path.to_string(),
                        line: Some((i + 1) as u32),
                        message: format!("Function declaration missing colon: {}", trimmed),
                    });
                }
            }
        }

        None
    }

    async fn request_fix(
        &self,
        files: &serde_json::Value,
        errors: &[CompileError],
    ) -> Result<serde_json::Value> {
        use omni_llm::{ChatMessage, ChatRequest, Role};

        let error_desc: Vec<String> = errors
            .iter()
            .map(|e| {
                format!(
                    "- {}{}: {}",
                    e.file,
                    e.line.map(|l| format!(":{}", l)).unwrap_or_default(),
                    e.message
                )
            })
            .collect();

        let request = ChatRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![
                ChatMessage {
                    role: Role::System,
                    content: "You are a Godot 4 GDScript expert. Fix the compilation errors in the provided code. Return the complete fixed files as JSON with 'files' array, each having 'path' and 'content' fields.".to_string(),
                },
                ChatMessage {
                    role: Role::User,
                    content: format!(
                        "Fix these errors:\n{}\n\nCurrent files:\n{}",
                        error_desc.join("\n"),
                        serde_json::to_string_pretty(files)?
                    ),
                },
            ],
            temperature: Some(0.2),
            max_tokens: Some(8192),
        };

        let response = self.llm.chat(&request).await?;
        let content = &response.choices[0].message.content;
        let fixed: serde_json::Value = serde_json::from_str(content)
            .unwrap_or_else(|_| files.clone());

        self.blackboard
            .set("compile/last_fix", &fixed, "compile_loop")
            .await?;

        Ok(fixed)
    }
}

pub async fn run_full_pipeline(
    llm_base_url: &str,
    api_key: &str,
    game_description: &str,
    project_dir: &Path,
) -> Result<serde_json::Value> {
    let project_id = Uuid::new_v4();
    let llm = LlmClient::new(llm_base_url.to_string(), api_key.to_string());
    let blackboard = Arc::new(Blackboard::in_memory(project_id)?);
    let event_bus = EventBus::new(256);

    let coordinator = Coordinator::new(
        llm.clone(),
        blackboard.clone(),
        event_bus.clone(),
        project_id,
    );

    let doc = GameDesignDoc {
        title: "Generated Game".to_string(),
        description: game_description.to_string(),
        genre: None,
        mechanics: vec![],
        scenes: vec![SceneSpec {
            name: "main".to_string(),
            description: "Main game scene".to_string(),
        }],
        entities: vec![],
        assets_needed: AssetsNeeded {
            sprites: vec![],
            models_3d: vec![],
            audio: vec![],
        },
    };

    let scheduler = coordinator.process_design_doc(&doc).await?;
    coordinator.execute(&scheduler).await?;

    let code_result: Option<serde_json::Value> = blackboard.get("result/code_generation").await?;

    if let Some(code) = code_result {
        let feedback_loop = CompileFeedbackLoop::new(
            llm,
            blackboard.clone(),
            event_bus,
            project_dir.to_path_buf(),
        );
        let compile_result = feedback_loop.run_with_fixes(&code).await?;

        Ok(serde_json::json!({
            "project_id": project_id.to_string(),
            "compile_success": compile_result.success,
            "compile_output": compile_result.output,
        }))
    } else {
        Ok(serde_json::json!({
            "project_id": project_id.to_string(),
            "status": "no_code_generated",
        }))
    }
}
