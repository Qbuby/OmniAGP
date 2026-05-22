use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use omni_core::{AssetQuality, GameEngine, GameProject, LlmProviderConfig, PipelineConfig, ProjectStatus};
use omni_llm::LlmClient;
use omni_orchestrator::Pipeline;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::state::{AppState, PipelineEvent};

#[derive(Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default)]
    pub quality: Option<String>,
}

#[derive(Serialize)]
pub struct ProjectResponse {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectResponse>,
    pub total: usize,
}

#[derive(Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize { 20 }

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/projects", get(list_projects).post(create_project))
        .route("/api/v1/projects/{id}", get(get_project).put(update_project).delete(delete_project))
        .route("/api/v1/projects/{id}/run", post(run_pipeline))
}

async fn create_project(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectResponse>), StatusCode> {
    let _claims = crate::auth::extract_claims(&state.jwt_secret, &headers)?;

    let quality = match req.quality.as_deref() {
        Some("low") => AssetQuality::Low,
        Some("high") => AssetQuality::High,
        _ => AssetQuality::Medium,
    };

    let project = GameProject {
        id: Uuid::new_v4(),
        name: req.name,
        description: req.description,
        status: ProjectStatus::Created,
        pipeline_config: PipelineConfig {
            target_engine: GameEngine::Godot4,
            asset_quality: quality,
            llm_provider: LlmProviderConfig {
                base_url: std::env::var("LLM_BASE_URL").unwrap_or_default(),
                model: state.default_model.clone(),
                api_key_env: "LLM_API_KEY".into(),
            },
        },
    };

    let resp = ProjectResponse {
        id: project.id,
        name: project.name.clone(),
        description: project.description.clone(),
        status: format!("{:?}", project.status),
        created_at: Utc::now().to_rfc3339(),
    };

    state.projects.insert(project.id, project);

    Ok((StatusCode::CREATED, Json(resp)))
}

async fn list_projects(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(query): axum::extract::Query<ListQuery>,
) -> Result<Json<ProjectListResponse>, StatusCode> {
    let _claims = crate::auth::extract_claims(&state.jwt_secret, &headers)?;

    let mut projects: Vec<ProjectResponse> = state
        .projects
        .iter()
        .filter(|entry| {
            if let Some(ref search) = query.search {
                entry.value().name.contains(search.as_str())
                    || entry.value().description.contains(search.as_str())
            } else {
                true
            }
        })
        .map(|entry| {
            let p = entry.value();
            ProjectResponse {
                id: p.id,
                name: p.name.clone(),
                description: p.description.clone(),
                status: format!("{:?}", p.status),
                created_at: Utc::now().to_rfc3339(),
            }
        })
        .collect();

    let total = projects.len();
    projects = projects.into_iter().skip(query.offset).take(query.limit).collect();

    Ok(Json(ProjectListResponse { projects, total }))
}

async fn get_project(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ProjectResponse>, StatusCode> {
    let _claims = crate::auth::extract_claims(&state.jwt_secret, &headers)?;

    let project = state.projects.get(&id).ok_or(StatusCode::NOT_FOUND)?;
    let p = project.value();

    Ok(Json(ProjectResponse {
        id: p.id,
        name: p.name.clone(),
        description: p.description.clone(),
        status: format!("{:?}", p.status),
        created_at: Utc::now().to_rfc3339(),
    }))
}

async fn update_project(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProjectRequest>,
) -> Result<Json<ProjectResponse>, StatusCode> {
    let _claims = crate::auth::extract_claims(&state.jwt_secret, &headers)?;

    let mut project = state.projects.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;
    if let Some(name) = req.name {
        project.name = name;
    }
    if let Some(desc) = req.description {
        project.description = desc;
    }

    let p = project.value();
    Ok(Json(ProjectResponse {
        id: p.id,
        name: p.name.clone(),
        description: p.description.clone(),
        status: format!("{:?}", p.status),
        created_at: Utc::now().to_rfc3339(),
    }))
}

async fn delete_project(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let _claims = crate::auth::extract_claims(&state.jwt_secret, &headers)?;
    state.projects.remove(&id).ok_or(StatusCode::NOT_FOUND)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn run_pipeline(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _claims = crate::auth::extract_claims(&state.jwt_secret, &headers)?;

    let project = state
        .projects
        .get(&id)
        .ok_or(StatusCode::NOT_FOUND)?
        .value()
        .clone();

    let events_tx = state.pipeline_events.clone();
    let projects = state.projects.clone();

    tokio::spawn(async move {
        let llm = LlmClient::from_env("LLM_BASE_URL", "LLM_API_KEY").unwrap();
        let mut pipeline = Pipeline::new(project.clone(), llm);

        let steps = ["Game Design Analysis", "Code Generation", "Asset Generation", "Scene Assembly"];
        for (i, step_name) in steps.iter().enumerate() {
            let _ = events_tx.send(PipelineEvent {
                project_id: id,
                step_name: step_name.to_string(),
                status: "running".into(),
                progress: (i as f32) / (steps.len() as f32),
                timestamp: Utc::now(),
            });

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        match pipeline.run().await {
            Ok(()) => {
                if let Some(mut p) = projects.get_mut(&id) {
                    p.status = ProjectStatus::Complete;
                }
                let _ = events_tx.send(PipelineEvent {
                    project_id: id,
                    step_name: "complete".into(),
                    status: "complete".into(),
                    progress: 1.0,
                    timestamp: Utc::now(),
                });
            }
            Err(e) => {
                if let Some(mut p) = projects.get_mut(&id) {
                    p.status = ProjectStatus::Failed(e.to_string());
                }
                let _ = events_tx.send(PipelineEvent {
                    project_id: id,
                    step_name: "error".into(),
                    status: format!("failed: {}", e),
                    progress: 0.0,
                    timestamp: Utc::now(),
                });
            }
        }
    });

    Ok(Json(serde_json::json!({ "status": "pipeline_started", "project_id": id })))
}
