use anyhow::Result;
use omni_assets::{AssetDirectorClient, AudioClient, AudioType};
use omni_core::{GameProject, PipelineStep, ProjectStatus, StepStatus, StepType};
use omni_llm::LlmClient;
use tracing::info;
use uuid::Uuid;

pub struct Pipeline {
    project: GameProject,
    steps: Vec<PipelineStep>,
    llm: LlmClient,
    audio_client: AudioClient,
    director_client: AssetDirectorClient,
}

impl Pipeline {
    pub fn new(project: GameProject, llm: LlmClient) -> Self {
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
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(project = %self.project.name, "starting pipeline");
        self.project.status = ProjectStatus::Analyzing;

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

            self.steps[i].output = result;
            self.steps[i].status = StepStatus::Complete;
            info!(step = %self.steps[i].name, "step complete");
        }

        self.project.status = ProjectStatus::Complete;
        info!(project = %self.project.name, "pipeline complete");
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

        let response = self.llm.chat(&request).await?;
        let content = &response.choices[0].message.content;
        let parsed: serde_json::Value = serde_json::from_str(content)
            .unwrap_or_else(|_| serde_json::json!({"raw_analysis": content}));
        Ok(parsed)
    }

    async fn generate_code(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"status": "placeholder", "note": "GDScript codegen to be implemented"}))
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
            .unwrap_or(serde_json::json!());

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
        Ok(serde_json::json!({"status": "placeholder", "note": "Scene assembly to be implemented"}))
    }
}
