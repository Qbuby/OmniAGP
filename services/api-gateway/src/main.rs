use anyhow::Result;
use axum::{extract::State, routing::{get, post}, Json, Router};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use omni_core::{AssetQuality, GameEngine, GameProject, LlmProviderConfig, PipelineConfig, ProjectStatus};
use omni_orchestrator::Pipeline;
use omni_llm::LlmClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

struct AppState {
    llm: LlmClient,
    default_model: String,
    pipeline_3d_url: String,
    http_client: reqwest::Client,
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

async fn generate_3d(
    State(state): State<Arc<AppState>>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let url = format!("{}/generate/3d", state.pipeline_3d_url);

    let resp = state
        .http_client
        .post(&url)
        .header("content-type", "application/json")
        .body(body.to_vec())
        .send()
        .await;

    match resp {
        Ok(r) => {
            let status = StatusCode::from_u16(r.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let body = r.bytes().await.unwrap_or_default();
            (status, [("content-type", "application/json")], body).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to proxy to pipeline-3d");
            (
                StatusCode::BAD_GATEWAY,
                [("content-type", "application/json")],
                format!("{{\"error\":\"pipeline-3d unavailable: {}\"}}", e).into_bytes(),
            )
                .into_response()
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

    let llm = LlmClient::from_env("LLM_BASE_URL", "LLM_API_KEY")?;
    let default_model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "qwen2.5-coder-7b".into());
    let pipeline_3d_url =
        std::env::var("PIPELINE_3D_URL").unwrap_or_else(|_| "http://localhost:8090".into());

    let state = Arc::new(AppState {
        llm,
        default_model,
        pipeline_3d_url,
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(130))
            .build()?,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/games", post(create_game))
        .route("/generate/3d", post(generate_3d))
        .with_state(state);

    let addr = "0.0.0.0:8080";
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
