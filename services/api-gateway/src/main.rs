use anyhow::Result;
use axum::{extract::State, http::StatusCode, routing::{get, post}, Json, Router};
use omni_assets::pipeline_2d::{Asset2DClient, Generate2DRequest, Generate2DResponse};
use omni_core::{AssetQuality, GameEngine, GameProject, LlmProviderConfig, PipelineConfig, ProjectStatus};
use omni_orchestrator::Pipeline;
use omni_llm::LlmClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

struct AppState {
    llm: LlmClient,
    default_model: String,
    asset_2d: Asset2DClient,
}

#[derive(Deserialize)]
struct CreateGameRequest {
    name: String,
    description: String,
}

#[derive(Serialize)]
struct CreateGameResponse {
    project_id: Uuid,
    status: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn health() -> &'static str {
    "ok"
}

async fn create_game(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateGameRequest>,
) -> Json<CreateGameResponse> {
    let project = GameProject {
        id: Uuid::new_v4(),
        name: req.name,
        description: req.description,
        status: ProjectStatus::Created,
        pipeline_config: PipelineConfig {
            target_engine: GameEngine::Godot4,
            asset_quality: AssetQuality::Medium,
            llm_provider: LlmProviderConfig {
                base_url: std::env::var("LLM_BASE_URL").unwrap_or_default(),
                model: state.default_model.clone(),
                api_key_env: "LLM_API_KEY".into(),
            },
        },
    };

    let project_id = project.id;

    tokio::spawn(async move {
        let llm = LlmClient::from_env("LLM_BASE_URL", "LLM_API_KEY").unwrap();
        let mut pipeline = Pipeline::new(project, llm);
        if let Err(e) = pipeline.run().await {
            tracing::error!(error = %e, "pipeline failed");
        }
    });

    Json(CreateGameResponse {
        project_id,
        status: "created".into(),
    })
}

async fn generate_2d(
    State(state): State<Arc<AppState>>,
    Json(req): Json<Generate2DRequest>,
) -> Result<Json<Generate2DResponse>, (StatusCode, Json<ErrorResponse>)> {
    state
        .asset_2d
        .generate(&req)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!(error = %e, "2D asset generation failed");
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

async fn generate_2d_health(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    state
        .asset_2d
        .health()
        .await
        .map(|h| Json(serde_json::to_value(h).unwrap()))
        .map_err(|e| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let llm = LlmClient::from_env("LLM_BASE_URL", "LLM_API_KEY")?;
    let default_model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "qwen2.5-coder-7b".into());
    let asset_2d = Asset2DClient::from_env();

    let state = Arc::new(AppState {
        llm,
        default_model,
        asset_2d,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/games", post(create_game))
        .route("/api/v1/generate/2d", post(generate_2d))
        .route("/api/v1/generate/2d/health", get(generate_2d_health))
        .with_state(state);

    let addr = "0.0.0.0:8080";
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
