use anyhow::Result;
use omni_core::{GameProject, PipelineStep, ProjectStatus, StepStatus, StepType};
use omni_llm::LlmClient;
use tracing::info;
use uuid::Uuid;

pub struct Pipeline {
    project: GameProject,
    steps: Vec<PipelineStep>,
    llm: LlmClient,
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

        Self {
            project,
            steps,
            llm,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!(project = %self.project.name, "starting pipeline");
        self.project.status = ProjectStatus::Analyzing;

        for i in 0..self.steps.len() {
            info!(step = %self.steps[i].name, "executing step");
            self.steps[i].status = StepStatus::Running;

            let step_type = self.steps[i].step_type.clone();
            match step_type {
                StepType::GameDesignAnalysis => {
                    let input = self.steps[i].input.clone();
                    let result = self.analyze_game_design(&input).await?;
                    self.steps[i].output = Some(result);
                }
                StepType::CodeGeneration => {
                    self.project.status = ProjectStatus::Generating;
                    let result = self.generate_code().await?;
                    self.steps[i].output = Some(result);
                }
                StepType::AssetGeneration => {
                    let result = self.generate_assets().await?;
                    self.steps[i].output = Some(result);
                }
                StepType::SceneAssembly => {
                    self.project.status = ProjectStatus::Assembling;
                    let result = self.assemble_scene().await?;
                    self.steps[i].output = Some(result);
                }
                StepType::Testing => {}
            }

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
        Ok(serde_json::json!({"status": "placeholder", "note": "Asset generation to be implemented"}))
    }

    async fn assemble_scene(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({"status": "placeholder", "note": "Scene assembly to be implemented"}))
    }
}
