use anyhow::Result;
use omni_assets::{AssetDirectorClient, AudioClient, AudioType};
use omni_core::{GameProject, PipelineStep, ProjectStatus, StepStatus, StepType};
use omni_llm::LlmClient;
use std::path::{Path, PathBuf};
use tracing::{info, warn};
use uuid::Uuid;

pub struct Pipeline {
    project: GameProject,
    steps: Vec<PipelineStep>,
    llm: LlmClient,
    audio_client: AudioClient,
    director_client: AssetDirectorClient,
    output_dir: Option<PathBuf>,
}

impl Pipeline {
    pub fn new(project: GameProject, llm: LlmClient) -> Self {
        Self::with_output_dir(project, llm, None)
    }

    pub fn with_output_dir(
        project: GameProject,
        llm: LlmClient,
        output_dir: Option<PathBuf>,
    ) -> Self {
        let steps = vec![
            PipelineStep {
                id: Uuid::new_v4(),
                name: "Game Design Analysis".into(),
                step_type: StepType::GameDesignAnalysis,
                status: StepStatus::Pending,
                input: serde_json::json!({"description": &project.description}),
                output: None,
            },
            PipelineStep {
                id: Uuid::new_v4(),
                name: "Code Generation".into(),
                step_type: StepType::CodeGeneration,
                status: StepStatus::Pending,
                input: serde_json::Value::Null,
                output: None,
            },
            PipelineStep {
                id: Uuid::new_v4(),
                name: "Asset Generation".into(),
                step_type: StepType::AssetGeneration,
                status: StepStatus::Pending,
                input: serde_json::Value::Null,
                output: None,
            },
            PipelineStep {
                id: Uuid::new_v4(),
                name: "Scene Assembly".into(),
                step_type: StepType::SceneAssembly,
                status: StepStatus::Pending,
                input: serde_json::Value::Null,
                output: None,
            },
        ];

        let audio_client = AudioClient::from_env();
        let director_client = AssetDirectorClient::from_env();

        Self {
            project,
            steps,
            llm,
            audio_client,
            director_client,
            output_dir,
        }
    }

    pub fn output_dir(&self) -> Option<&Path> {
        self.output_dir.as_deref()
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(project = %self.project.name, "starting pipeline");
        self.project.status = ProjectStatus::Analyzing;

        if let Some(dir) = self.output_dir.clone() {
            if let Err(e) = self.prepare_output_dir(&dir) {
                warn!(error = %e, dir = %dir.display(), "failed to prepare output dir, continuing without artifact persistence");
            }
        }

        for i in 0..self.steps.len() {
            info!(step = %self.steps[i].name, "executing step");
            self.steps[i].status = StepStatus::Running;

            let result = match self.steps[i].step_type {
                StepType::GameDesignAnalysis => {
                    let input = self.steps[i].input.clone();
                    Some(self.analyze_game_design(&input).await?)
                }
                StepType::CodeGeneration => {
                    self.project.status = ProjectStatus::Generating;
                    Some(self.generate_code().await?)
                }
                StepType::AssetGeneration => {
                    Some(self.generate_assets().await?)
                }
                StepType::SceneAssembly => {
                    self.project.status = ProjectStatus::Assembling;
                    Some(self.assemble_scene().await?)
                }
                StepType::Testing => None,
            };

            self.steps[i].output = result.clone();
            self.steps[i].status = StepStatus::Complete;

            if let (Some(dir), Some(out)) = (self.output_dir.as_ref(), result.as_ref()) {
                if let Err(e) = persist_step_output(dir, &self.steps[i], out) {
                    warn!(error = %e, step = %self.steps[i].name, "failed to persist step output");
                }
            }

            info!(step = %self.steps[i].name, "step complete");
        }

        if let Some(dir) = self.output_dir.clone() {
            if let Err(e) = self.write_summary(&dir) {
                warn!(error = %e, "failed to write project summary files");
            }
        }

        self.project.status = ProjectStatus::Complete;
        info!(project = %self.project.name, "pipeline complete");
        Ok(())
    }

    fn prepare_output_dir(&self, dir: &Path) -> Result<()> {
        std::fs::create_dir_all(dir)?;
        std::fs::create_dir_all(dir.join("steps"))?;
        std::fs::create_dir_all(dir.join("assets"))?;
        Ok(())
    }

    fn write_summary(&self, dir: &Path) -> Result<()> {
        let design_output = self
            .steps
            .iter()
            .find(|s| s.step_type == StepType::GameDesignAnalysis)
            .and_then(|s| s.output.clone())
            .unwrap_or(serde_json::json!({}));

        let design_path = dir.join("design.json");
        std::fs::write(&design_path, serde_json::to_vec_pretty(&design_output)?)?;

        let assets_output = self
            .steps
            .iter()
            .find(|s| s.step_type == StepType::AssetGeneration)
            .and_then(|s| s.output.clone())
            .unwrap_or(serde_json::json!({}));

        let manifest = build_asset_manifest(&assets_output, dir)?;
        let manifest_path = dir.join("assets").join("manifest.json");
        std::fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;

        let readme = format!(
            "# {name}\n\n{desc}\n\n## Project\n- id: {id}\n- engine: {engine:?}\n- quality: {quality:?}\n\n## Files\n- design.json — game design document\n- steps/*.json — per-step pipeline outputs\n- assets/manifest.json — generated asset manifest\n- assets/*  — copied audio assets (when produced)\n",
            name = self.project.name,
            desc = self.project.description,
            id = self.project.id,
            engine = self.project.pipeline_config.target_engine,
            quality = self.project.pipeline_config.asset_quality,
        );
        std::fs::write(dir.join("README.md"), readme)?;

        let project_meta = serde_json::json!({
            "id": self.project.id,
            "name": self.project.name,
            "description": self.project.description,
        });
        std::fs::write(dir.join("project.json"), serde_json::to_vec_pretty(&project_meta)?)?;

        Ok(())
    }

    async fn analyze_game_design(&self, input: &serde_json::Value) -> Result<serde_json::Value> {
        use omni_llm::{ChatMessage, ChatRequest, Role};

        let description = input["description"].as_str().unwrap_or("");
        let request = ChatRequest {
            model: self.project.pipeline_config.llm_provider.model.clone(),
            messages: vec![
                ChatMessage {
                    role: Role::System,
                    content: "You are a game design analyst. Analyze the game description and output a structured game design document in JSON format with fields: genre, mechanics, scenes, entities, assets_needed.".into(),
                },
                ChatMessage {
                    role: Role::User,
                    content: format!("Analyze this game concept:\n{}", description),
                },
            ],
            temperature: Some(0.3),
            max_tokens: Some(4096),
        };

        match self.llm.chat(&request).await {
            Ok(response) => {
                let content = &response.choices[0].message.content;
                let parsed: serde_json::Value = serde_json::from_str(content)
                    .unwrap_or_else(|_| serde_json::json!({"raw_analysis": content}));
                Ok(parsed)
            }
            Err(e) => {
                warn!(error = %e, "LLM unavailable, returning fallback design doc");
                Ok(serde_json::json!({
                    "status": "fallback",
                    "genre": "unknown",
                    "mechanics": [],
                    "scenes": [{"name": "main", "description": "Main game scene"}],
                    "entities": [],
                    "assets_needed": {"audio": [], "sprites": [], "models_3d": []},
                    "description": description,
                }))
            }
        }
    }

    async fn generate_code(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "status": "placeholder",
            "note": "GDScript codegen to be implemented",
            "files": [
                {
                    "path": "scripts/main.gd",
                    "content": "extends Node\n\nfunc _ready():\n\tprint(\"OmniAGP placeholder scene\")\n"
                }
            ]
        }))
    }

    async fn generate_assets(&self) -> Result<serde_json::Value> {
        let design_step = self.steps.iter().find(|s| s.step_type == StepType::GameDesignAnalysis);
        let design_output = design_step
            .and_then(|s| s.output.as_ref())
            .cloned()
            .unwrap_or(serde_json::json!({}));

        match self.director_client.execute_design_doc(design_output.clone()).await {
            Ok(resp) => {
                info!(
                    total = resp.total_tasks,
                    succeeded = resp.succeeded,
                    failed = resp.failed,
                    "asset director completed"
                );
                Ok(serde_json::json!({
                    "status": "completed",
                    "method": "asset_director",
                    "total_tasks": resp.total_tasks,
                    "succeeded": resp.succeeded,
                    "failed": resp.failed,
                    "failures": resp.failures,
                    "asset_registry": resp.asset_registry,
                }))
            }
            Err(e) => {
                tracing::warn!(error = %e, "asset director unavailable, falling back to direct audio generation");
                self.generate_assets_fallback(&design_output).await
            }
        }
    }

    async fn generate_assets_fallback(&self, design_output: &serde_json::Value) -> Result<serde_json::Value> {
        let assets_needed = design_output
            .get("assets_needed")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let mut results = vec![];

        if let Some(audio_assets) = assets_needed.get("audio").and_then(|a| a.as_array()) {
            for asset in audio_assets {
                let description = asset.get("description").and_then(|d| d.as_str()).unwrap_or("");
                let audio_type_str = asset.get("type").and_then(|t| t.as_str()).unwrap_or("sfx");
                let duration = asset.get("duration_sec").and_then(|d| d.as_f64());

                let audio_type = match audio_type_str {
                    "bgm" => AudioType::Bgm,
                    _ => AudioType::Sfx,
                };

                match self.audio_client.generate(description, audio_type, duration, None).await {
                    Ok(resp) => {
                        results.push(serde_json::json!({
                            "type": audio_type_str,
                            "file_path": resp.file_path,
                            "valid": resp.validation.valid,
                        }));
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "audio asset generation failed, continuing");
                        results.push(serde_json::json!({
                            "type": audio_type_str,
                            "error": e.to_string(),
                        }));
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "status": "completed",
            "method": "fallback_direct",
            "audio_assets": results,
        }))
    }

    async fn assemble_scene(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "status": "placeholder",
            "note": "Scene assembly to be implemented",
            "scenes": [
                {"name": "main", "path": "scenes/main.tscn"}
            ]
        }))
    }
}

fn step_slug(step: &PipelineStep) -> String {
    match step.step_type {
        StepType::GameDesignAnalysis => "01_game_design".into(),
        StepType::CodeGeneration => "02_code_generation".into(),
        StepType::AssetGeneration => "03_asset_generation".into(),
        StepType::SceneAssembly => "04_scene_assembly".into(),
        StepType::Testing => "05_testing".into(),
    }
}

fn persist_step_output(
    dir: &Path,
    step: &PipelineStep,
    output: &serde_json::Value,
) -> Result<()> {
    let steps_dir = dir.join("steps");
    std::fs::create_dir_all(&steps_dir)?;
    let path = steps_dir.join(format!("{}.json", step_slug(step)));
    std::fs::write(&path, serde_json::to_vec_pretty(output)?)?;
    Ok(())
}

fn build_asset_manifest(
    assets_output: &serde_json::Value,
    project_dir: &Path,
) -> Result<serde_json::Value> {
    let mut entries: Vec<serde_json::Value> = Vec::new();
    let assets_dir = project_dir.join("assets");
    std::fs::create_dir_all(&assets_dir)?;

    if let Some(audio_assets) = assets_output.get("audio_assets").and_then(|a| a.as_array()) {
        for (i, asset) in audio_assets.iter().enumerate() {
            let asset_type = asset.get("type").and_then(|t| t.as_str()).unwrap_or("sfx").to_string();
            let mut entry = serde_json::json!({
                "kind": "audio",
                "type": asset_type,
            });

            if let Some(src_path) = asset.get("file_path").and_then(|p| p.as_str()) {
                let src = PathBuf::from(src_path);
                let file_name = src
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| format!("audio_{i}.bin"));
                let dest = assets_dir.join(&file_name);
                match std::fs::copy(&src, &dest) {
                    Ok(_) => {
                        entry["source"] = serde_json::Value::String(src_path.to_string());
                        entry["path"] = serde_json::Value::String(format!("assets/{file_name}"));
                    }
                    Err(e) => {
                        warn!(error = %e, src = %src.display(), "failed to copy audio asset, recording reference only");
                        entry["source"] = serde_json::Value::String(src_path.to_string());
                        entry["copy_error"] = serde_json::Value::String(e.to_string());
                    }
                }
            }

            entries.push(entry);
        }
    }

    if let Some(registry) = assets_output.get("asset_registry") {
        entries.push(serde_json::json!({
            "kind": "registry",
            "data": registry,
        }));
    }

    Ok(serde_json::json!({
        "method": assets_output.get("method").cloned().unwrap_or(serde_json::Value::Null),
        "status": assets_output.get("status").cloned().unwrap_or(serde_json::Value::Null),
        "entries": entries,
    }))
}
