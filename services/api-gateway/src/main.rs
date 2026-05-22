use anyhow::Result;
use axum::{extract::State, routing::{get, post}, Json, Router};
use dashmap::DashMap;
use omni_api_gateway::{auth, projects, static_files, state::AppState, websocket};
use omni_core::{TaskCapability, TaskPayload, TaskPriority};
use omni_scheduler::TaskQueue;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

#[derive(Deserialize)]
struct CreateGameRequest {
    name: String,
    description: String,
    #[serde(default)]
    priority: Option<String>,
}

#[derive(Serialize)]
struct CreateGameResponse {
    project_id: Uuid,
    status: String,
    tasks: Vec<Uuid>,
}

#[derive(Deserialize)]
struct SubmitTaskRequest {
    project_id: Uuid,
    capability: TaskCapability,
    priority: Option<TaskPriority>,
    min_vram_gb: Option<u32>,
    payload: serde_json::Value,
}

#[derive(Serialize)]
struct SubmitTaskResponse {
    task_id: Uuid,
    status: String,
}

async fn create_game(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateGameRequest>,
) -> Json<CreateGameResponse> {
    let project_id = Uuid::new_v4();
    let priority = match req.priority.as_deref() {
        Some("urgent") => TaskPriority::Urgent,
        Some("batch") => TaskPriority::Batch,
        _ => TaskPriority::Normal,
    };

    let now = chrono::Utc::now().timestamp();
    let mut task_ids = Vec::new();

    let tasks = vec![
        (TaskCapability::LlmInference, "game_design_analysis", 8u32),
        (TaskCapability::LlmInference, "code_generation", 8),
        (TaskCapability::Image2D, "asset_generation_2d", 10),
        (TaskCapability::Model3D, "asset_generation_3d", 16),
        (TaskCapability::Audio, "audio_generation", 6),
    ];

    for (cap, step_name, min_vram) in tasks {
        let task = TaskPayload {
            id: Uuid::new_v4(),
            project_id,
            priority,
            capability: cap,
            min_vram_gb: min_vram,
            payload: serde_json::json!({
                "step": step_name,
                "project_name": req.name,
                "description": req.description,
            }),
            retry_count: 0,
            max_retries: 3,
            created_at: now,
        };
        task_ids.push(task.id);
        if let Err(e) = state.queue.publish(task).await {
            tracing::error!(error = %e, "failed to publish task");
        }
    }

    Json(CreateGameResponse {
        project_id,
        status: "queued".into(),
        tasks: task_ids,
    })
}

async fn submit_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SubmitTaskRequest>,
) -> Json<SubmitTaskResponse> {
    let task = TaskPayload {
        id: Uuid::new_v4(),
        project_id: req.project_id,
        priority: req.priority.unwrap_or(TaskPriority::Normal),
        capability: req.capability,
        min_vram_gb: req.min_vram_gb.unwrap_or(
            omni_scheduler::strategy::min_vram_for_capability(&req.capability)
        ),
        payload: req.payload,
        retry_count: 0,
        max_retries: 3,
        created_at: chrono::Utc::now().timestamp(),
    };

    let task_id = task.id;
    match state.queue.publish(task).await {
        Ok(_) => Json(SubmitTaskResponse { task_id, status: "queued".into() }),
        Err(e) => {
            tracing::error!(error = %e, "failed to submit task");
            Json(SubmitTaskResponse { task_id, status: "error".into() })
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".into());
    let default_model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "qwen2.5-coder-7b".into());
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "omniagp-dev-secret-change-me".into());
    let github_client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_default();
    let github_client_secret = std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_default();

    let (events_tx, _) = broadcast::channel(256);

    let queue = Arc::new(TaskQueue::connect(&nats_url).await?);
    tracing::info!("connected to NATS at {}", nats_url);

    let state = Arc::new(AppState {
        queue,
        default_model,
        projects: Arc::new(DashMap::new()),
        pipeline_events: Arc::new(events_tx),
        jwt_secret,
        github_client_id,
        github_client_secret,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/api/v1/games", post(create_game))
        .route("/api/v1/tasks", post(submit_task))
        .merge(auth::router())
        .merge(projects::router())
        .merge(websocket::router())
        .merge(static_files::router())
        .layer(cors)
        .with_state(state);

    let addr = "0.0.0.0:8080";
    tracing::info!("OmniAGP api-gateway listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
