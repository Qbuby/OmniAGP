use axum::{
    body::Body,
    extract::{Json, Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use omni_core::{AssetQuality, GameEngine, GameProject, LlmProviderConfig, PipelineConfig, ProjectStatus};
use omni_llm::LlmClient;
use omni_orchestrator::Pipeline;
use serde::{Deserialize, Serialize};
use std::io::{Cursor, Read, Write};
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use uuid::Uuid;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

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
        .route("/api/v1/projects/{id}/artifact", get(download_artifact))
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
    state.artifact_dirs.remove(&id);
    Ok(StatusCode::NO_CONTENT)
}

async fn download_artifact(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<Uuid>,
) -> Response {
    if let Err(status) = crate::auth::extract_claims(&state.jwt_secret, &headers) {
        return status.into_response();
    }

    let (project_name, status_label) = match state.projects.get(&id) {
        Some(entry) => {
            let p = entry.value();
            (p.name.clone(), format!("{:?}", p.status))
        }
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    if status_label != "Complete" {
        return (
            StatusCode::CONFLICT,
            [(header::CONTENT_TYPE, HeaderValue::from_static("application/json"))],
            r#"{"error":"pipeline_not_complete"}"#,
        )
            .into_response();
    }

    let artifact_dir = match state.artifact_dirs.get(&id) {
        Some(entry) => entry.value().clone(),
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    if !artifact_dir.exists() {
        return StatusCode::NOT_FOUND.into_response();
    }

    let dir_clone = artifact_dir.clone();
    let zip_result =
        tokio::task::spawn_blocking(move || zip_directory(&dir_clone)).await;

    let bytes = match zip_result {
        Ok(Ok(b)) => b,
        Ok(Err(e)) => {
            tracing::error!(error = %e, "failed to zip artifact dir");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "zip task panicked");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let short_id = id.simple().to_string();
    let short = &short_id[..short_id.len().min(8)];
    let safe_name = sanitize_filename(&project_name);
    let filename = format!("{safe_name}-{short}.zip");
    let disposition = format!("attachment; filename=\"{filename}\"");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::CONTENT_LENGTH, bytes.len().to_string())
        .body(Body::from(bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn sanitize_filename(name: &str) -> String {
    let trimmed = name.trim();
    let mut out: String = trimmed
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '_'
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() {
        out = "project".into();
    }
    if out.len() > 64 {
        out.truncate(64);
    }
    out
}

fn zip_directory(dir: &StdPath) -> std::io::Result<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::<u8>::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buffer);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let mut wrote_any = false;
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let rel = match path.strip_prefix(dir) {
                Ok(r) => r,
                Err(_) => continue,
            };
            if rel.as_os_str().is_empty() {
                continue;
            }
            let rel_str = rel.to_string_lossy().replace('\\', "/");

            if entry.file_type().is_dir() {
                let dir_entry = format!("{rel_str}/");
                writer.add_directory(dir_entry, options)?;
            } else if entry.file_type().is_file() {
                writer.start_file(rel_str, options)?;
                let mut f = std::fs::File::open(path)?;
                let mut buf = Vec::new();
                f.read_to_end(&mut buf)?;
                writer.write_all(&buf)?;
                wrote_any = true;
            }
        }

        if !wrote_any {
            writer.start_file("EMPTY.txt", options)?;
            writer.write_all(b"No artifacts were produced for this project.\n")?;
        }

        writer.finish()?;
    }
    Ok(buffer.into_inner())
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
    let artifact_dirs = state.artifact_dirs.clone();
    let artifact_dir: PathBuf = state.artifact_root.join(id.to_string());

    if let Err(e) = std::fs::create_dir_all(&artifact_dir) {
        tracing::warn!(
            error = %e,
            dir = %artifact_dir.display(),
            "failed to pre-create project artifact dir"
        );
    }
    artifact_dirs.insert(id, artifact_dir.clone());

    tokio::spawn(async move {
        let llm = LlmClient::from_env("LLM_BASE_URL", "LLM_API_KEY").unwrap();
        let mut pipeline =
            Pipeline::with_output_dir(project.clone(), llm, Some(artifact_dir.clone()));

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
