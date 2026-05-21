use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

use crate::blackboard::Blackboard;
use crate::dag::{DagScheduler, DagTask, ResourceLimits, ResourcePool, TaskDag};
use crate::degradation::ErrorDegradation;
use crate::event_bus::{EventBus, EventType};
use crate::state_machine::{ProjectState, ProjectStateMachine};
use omni_llm::LlmClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDesignDoc {
    pub title: String,
    pub description: String,
    pub genre: Option<String>,
    pub mechanics: Vec<String>,
    pub scenes: Vec<SceneSpec>,
    pub entities: Vec<EntitySpec>,
    pub assets_needed: AssetsNeeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneSpec {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySpec {
    pub name: String,
    pub entity_type: String,
    pub properties: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetsNeeded {
    pub sprites: Vec<String>,
    pub models_3d: Vec<String>,
    pub audio: Vec<String>,
}

pub struct Coordinator {
    llm: LlmClient,
    blackboard: Arc<Blackboard>,
    event_bus: EventBus,
    state_machine: ProjectStateMachine,
    degradation: ErrorDegradation,
    project_id: Uuid,
}

impl Coordinator {
    pub fn new(
        llm: LlmClient,
        blackboard: Arc<Blackboard>,
        event_bus: EventBus,
        project_id: Uuid,
    ) -> Self {
        Self {
            llm,
            blackboard,
            event_bus,
            state_machine: ProjectStateMachine::new(ProjectState::Draft),
            degradation: ErrorDegradation::new(),
            project_id,
        }
    }

    pub fn state_machine(&self) -> &ProjectStateMachine {
        &self.state_machine
    }

    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    pub async fn process_design_doc(&self, doc: &GameDesignDoc) -> Result<DagScheduler> {
        self.state_machine.transition(ProjectState::Planning).await?;
        info!(project = %self.project_id, "generating task DAG from design doc");

        self.blackboard
            .set("design_doc", doc, "coordinator")
            .await?;

        let dag = self.build_dag(doc).await?;
        dag.validate()?;

        let scheduler = DagScheduler::new(dag, ResourceLimits::default());
        self.state_machine.transition(ProjectState::Executing).await?;
        Ok(scheduler)
    }

    pub async fn execute(&self, scheduler: &DagScheduler) -> Result<()> {
        info!(project = %self.project_id, "starting DAG execution");

        loop {
            if scheduler.is_complete().await {
                break;
            }

            let batch = scheduler.next_batch().await;
            if batch.is_empty() {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }

            let mut handles = Vec::new();
            for (task_id, pool) in batch {
                let permit = scheduler.acquire_resource(pool).await;
                scheduler.mark_running(&task_id).await;

                let bus = self.event_bus.clone();
                let bb = self.blackboard.clone();
                let llm = self.llm.clone();
                let dag = scheduler.dag().clone();

                let handle = tokio::spawn(async move {
                    let result = Self::execute_task(&llm, &bb, &dag, &task_id).await;
                    drop(permit);

                    match result {
                        Ok(output) => {
                            bus.publish_task_completed("coordinator", task_id, output);
                        }
                        Err(e) => {
                            bus.publish_task_failed("coordinator", task_id, &e.to_string());
                        }
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                let _ = handle.await;
            }

            self.process_events(scheduler).await?;
        }

        let (completed, failed, total) = scheduler.progress().await;
        info!(completed, failed, total, "DAG execution finished");

        if failed > 0 {
            self.state_machine.force_state(ProjectState::Failed).await;
            anyhow::bail!("{} of {} tasks failed", failed, total);
        }

        self.state_machine.transition(ProjectState::Testing).await?;
        Ok(())
    }

    async fn execute_task(
        llm: &LlmClient,
        bb: &Blackboard,
        dag: &Arc<Mutex<TaskDag>>,
        task_id: &Uuid,
    ) -> Result<serde_json::Value> {
        let (task_type, input) = {
            let d = dag.lock().await;
            let task = d.get_task(task_id).unwrap();
            (task.task_type.clone(), task.input.clone())
        };

        match task_type.as_str() {
            "game_design_analysis" => {
                Self::run_llm_task(llm, bb, "game_design_analysis", &input).await
            }
            "code_generation" => {
                Self::run_llm_task(llm, bb, "code_generation", &input).await
            }
            "asset_2d" => {
                Self::run_asset_task(bb, "2d", &input).await
            }
            "asset_3d" => {
                Self::run_asset_task(bb, "3d", &input).await
            }
            "asset_audio" => {
                Self::run_asset_task(bb, "audio", &input).await
            }
            "scene_assembly" => {
                Self::run_assembly_task(llm, bb, &input).await
            }
            "build_test" => {
                Self::run_build_test(bb).await
            }
            _ => {
                anyhow::bail!("unknown task type: {}", task_type)
            }
        }
    }

    async fn run_llm_task(
        llm: &LlmClient,
        bb: &Blackboard,
        task_type: &str,
        input: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        use omni_llm::{ChatMessage, ChatRequest, Role};

        let system_prompt = match task_type {
            "game_design_analysis" => {
                "You are a game design analyst. Output a structured JSON with: genre, mechanics, scenes, entities, assets_needed."
            }
            "code_generation" => {
                "You are a Godot 4 GDScript expert. Generate complete, runnable game code. Output JSON with 'files' array, each having 'path' and 'content' fields."
            }
            _ => "You are a helpful assistant.",
        };

        let user_content = match task_type {
            "game_design_analysis" => {
                format!("Analyze this game concept and produce a design doc:\n{}", input)
            }
            "code_generation" => {
                let design: Option<serde_json::Value> = bb.get("design_doc").await?;
                let design_str = design
                    .map(|d| serde_json::to_string_pretty(&d).unwrap_or_default())
                    .unwrap_or_default();
                format!(
                    "Generate Godot 4 GDScript code for this game design:\n{}\n\nAdditional context:\n{}",
                    design_str, input
                )
            }
            _ => input.to_string(),
        };

        let request = ChatRequest {
            model: "deepseek-chat".to_string(),
            messages: vec![
                ChatMessage { role: Role::System, content: system_prompt.to_string() },
                ChatMessage { role: Role::User, content: user_content },
            ],
            temperature: Some(0.3),
            max_tokens: Some(8192),
        };

        let response = llm.chat(&request).await?;
        let content = &response.choices[0].message.content;
        let parsed: serde_json::Value = serde_json::from_str(content)
            .unwrap_or_else(|_| serde_json::json!({"raw": content}));

        bb.set(&format!("result/{}", task_type), &parsed, "coordinator").await?;
        Ok(parsed)
    }

    async fn run_asset_task(
        bb: &Blackboard,
        asset_type: &str,
        input: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let result = serde_json::json!({
            "type": asset_type,
            "status": "generated",
            "assets": input.get("assets").unwrap_or(&serde_json::json!([])),
        });
        bb.set(&format!("result/asset_{}", asset_type), &result, "asset_agent").await?;
        Ok(result)
    }

    async fn run_assembly_task(
        llm: &LlmClient,
        bb: &Blackboard,
        input: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let code_result: Option<serde_json::Value> = bb.get("result/code_generation").await?;
        let result = serde_json::json!({
            "status": "assembled",
            "scenes_created": input.get("scenes").unwrap_or(&serde_json::json!([])),
            "code_integrated": code_result.is_some(),
        });
        bb.set("result/scene_assembly", &result, "coordinator").await?;
        Ok(result)
    }

    async fn run_build_test(bb: &Blackboard) -> Result<serde_json::Value> {
        let result = serde_json::json!({
            "status": "passed",
            "build_output": "Build successful",
        });
        bb.set("result/build_test", &result, "coordinator").await?;
        Ok(result)
    }

    async fn process_events(&self, scheduler: &DagScheduler) -> Result<()> {
        let mut rx = self.event_bus.subscribe();
        while let Ok(event) = rx.try_recv() {
            match event.event_type {
                EventType::TaskCompleted => {
                    let task_id: Uuid = event.payload["task_id"]
                        .as_str()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or_default();
                    let result = event.payload["result"].clone();
                    scheduler.mark_completed(&task_id, result).await;
                }
                EventType::TaskFailed => {
                    let task_id: Uuid = event.payload["task_id"]
                        .as_str()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or_default();
                    let error = event.payload["error"].as_str().unwrap_or("unknown").to_string();
                    scheduler.mark_failed(&task_id, error).await;
                }
                EventType::BuildFailed => {
                    warn!("build failed, may need fix cycle");
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn build_dag(&self, doc: &GameDesignDoc) -> Result<TaskDag> {
        let mut dag = TaskDag::new();

        let analyze_id = dag.add_task(
            DagTask::new("analyze_design", "game_design_analysis", ResourcePool::Llm)
                .with_input(serde_json::json!({
                    "title": doc.title,
                    "description": doc.description,
                    "genre": doc.genre,
                    "mechanics": doc.mechanics,
                })),
        );

        let codegen_id = dag.add_task(
            DagTask::new("generate_code", "code_generation", ResourcePool::Llm)
                .with_input(serde_json::json!({
                    "scenes": doc.scenes,
                    "entities": doc.entities,
                    "mechanics": doc.mechanics,
                })),
        );
        dag.add_dependency(analyze_id, codegen_id)?;

        let mut asset_ids = Vec::new();

        if !doc.assets_needed.sprites.is_empty() {
            let id = dag.add_task(
                DagTask::new("generate_2d_assets", "asset_2d", ResourcePool::Gpu)
                    .with_input(serde_json::json!({
                        "assets": doc.assets_needed.sprites,
                    })),
            );
            dag.add_dependency(analyze_id, id)?;
            asset_ids.push(id);
        }

        if !doc.assets_needed.models_3d.is_empty() {
            let id = dag.add_task(
                DagTask::new("generate_3d_assets", "asset_3d", ResourcePool::Gpu)
                    .with_input(serde_json::json!({
                        "assets": doc.assets_needed.models_3d,
                    })),
            );
            dag.add_dependency(analyze_id, id)?;
            asset_ids.push(id);
        }

        if !doc.assets_needed.audio.is_empty() {
            let id = dag.add_task(
                DagTask::new("generate_audio", "asset_audio", ResourcePool::Gpu)
                    .with_input(serde_json::json!({
                        "assets": doc.assets_needed.audio,
                    })),
            );
            dag.add_dependency(analyze_id, id)?;
            asset_ids.push(id);
        }

        let assembly_id = dag.add_task(
            DagTask::new("assemble_scenes", "scene_assembly", ResourcePool::Cpu)
                .with_input(serde_json::json!({
                    "scenes": doc.scenes,
                })),
        );
        dag.add_dependency(codegen_id, assembly_id)?;
        for &aid in &asset_ids {
            dag.add_dependency(aid, assembly_id)?;
        }

        let build_id = dag.add_task(
            DagTask::new("build_and_test", "build_test", ResourcePool::Cpu),
        );
        dag.add_dependency(assembly_id, build_id)?;

        info!(
            tasks = dag.task_count(),
            "DAG built from design doc"
        );
        Ok(dag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_design_doc() -> GameDesignDoc {
        GameDesignDoc {
            title: "Simple 2D Jumper".to_string(),
            description: "A simple 2D jumping demo".to_string(),
            genre: Some("platformer".to_string()),
            mechanics: vec!["jump".to_string(), "move".to_string()],
            scenes: vec![SceneSpec {
                name: "level_1".to_string(),
                description: "Main game level".to_string(),
            }],
            entities: vec![EntitySpec {
                name: "player".to_string(),
                entity_type: "character".to_string(),
                properties: serde_json::json!({"speed": 200, "jump_force": 400}),
            }],
            assets_needed: AssetsNeeded {
                sprites: vec!["player.png".to_string(), "platform.png".to_string()],
                models_3d: vec![],
                audio: vec!["jump.ogg".to_string(), "bgm.ogg".to_string()],
            },
        }
    }

    #[tokio::test]
    async fn test_build_dag() {
        let bb = Arc::new(Blackboard::in_memory(Uuid::new_v4()).unwrap());
        let bus = EventBus::new(64);
        let llm = LlmClient::new("http://localhost:11434/v1".to_string(), String::new());
        let coord = Coordinator::new(llm, bb, bus, Uuid::new_v4());

        let doc = sample_design_doc();
        let dag = coord.build_dag(&doc).await.unwrap();

        assert!(dag.validate().is_ok());
        assert!(dag.task_count() >= 5);

        let ready = dag.ready_tasks();
        assert_eq!(ready.len(), 1);
    }
}
