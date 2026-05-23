use crate::users::UserStore;
use dashmap::DashMap;
use omni_core::GameProject;
use omni_scheduler::TaskQueue;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

pub struct AppState {
    pub queue: Arc<TaskQueue>,
    pub default_model: String,
    pub projects: Arc<DashMap<Uuid, GameProject>>,
    pub pipeline_events: Arc<broadcast::Sender<PipelineEvent>>,
    pub jwt_secret: String,
    pub github_client_id: String,
    pub github_client_secret: String,
    pub user_store: UserStore,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PipelineEvent {
    pub project_id: Uuid,
    pub step_name: String,
    pub status: String,
    pub progress: f32,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
