use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod error;
pub mod scheduler;

pub use scheduler::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameProject {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: ProjectStatus,
    pub pipeline_config: PipelineConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectStatus {
    Created,
    Analyzing,
    Generating,
    Assembling,
    Complete,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub target_engine: GameEngine,
    pub asset_quality: AssetQuality,
    pub llm_provider: LlmProviderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEngine {
    Godot4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssetQuality {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub base_url: String,
    pub model: String,
    pub api_key_env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStep {
    pub id: Uuid,
    pub name: String,
    pub step_type: StepType,
    pub status: StepStatus,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    GameDesignAnalysis,
    CodeGeneration,
    AssetGeneration,
    SceneAssembly,
    Testing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Complete,
    Failed(String),
}
