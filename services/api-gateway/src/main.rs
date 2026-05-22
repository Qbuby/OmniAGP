use anyhow::Result;
use axum::{extract::State, http::StatusCode, routing::{get, post}, Json, Router};
use axum::response::IntoResponse;
use omni_assets::pipeline_2d::{Asset2DClient, Generate2DRequest, Generate2DResponse};
use omni_assets::{AssetDirectorClient, AudioClient, AudioType};
use omni_core::{AssetQuality, GameEngine, GameProject, LlmProviderConfig, PipelineConfig, ProjectStatus};
use omni_llm::LlmClient;
use omni_orchestrator::Pipeline;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

struct AppState {
    llm: LlmClient,
    default_model: String,
    asset_2d: Asset2DClient,
    pipeline_3d_url: String,
    http_client: reqwest::Client,
    audio_client: AudioClient,
    director_client: AssetDirectorClient,
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

#[derive(Deserialize)]
struct GenerateAudioRequest {
    prompt: String,
    audio_type: String,
    duration_sec: Option<f64>,
    output_dir: Option<String>,
}

#[derive(Serialize)]
struct GenerateAudioResponse {
    file_path: String,
    audio_type: String,
    duration_sec: f64,
    sample_rate: u32,
    loop_point_samples: Option<u64>,
    valid: bool,
    issues: Vec<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Deserialize)]
struct ExecuteAssetsRequest {
    design_doc: serde_json::Value,
}

#[derive(Serialize)]
struct ExecuteAssetsResponse {
    total_tasks: u32,
    succeeded: u32,
    failed: u32,
    failures: Vec<serde_json::Value>,
    asset_registry: serde_json::Value,
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

async fn unload_models(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    state
        .asset_2d
        .unload_models()
        .await
        .map(|_| Json(serde_json::json!({"status": "ok"})))
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
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

async fn generate_audio(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GenerateAudioRequest>,
) -> Result<Json<GenerateAudioResponse>, (StatusCode, Json<ErrorResponse>)> {
    let audio_type = match req.audio_type.as_str() {
        "bgm" => AudioType::Bgm,
        "sfx" => AudioType::Sfx,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "audio_type must be 'bgm' or 'sfx'".into(),
                }),
            ));
        }
    };

    let result = state
        .audio_client
        .generate(
            &req.prompt,
            audio_type,
            req.duration_sec,
            req.output_dir.as_deref(),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "audio generation failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(GenerateAudioResponse {
        file_path: result.file_path,
        audio_type: result.audio_type,
        duration_sec: result.duration_sec,
        sample_rate: result.sample_rate,
        loop_point_samples: result.loop_point_samples,
        valid: result.validation.valid,
        issues: result.validation.issues,
    }))
}

async fn execute_assets(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExecuteAssetsRequest>,
) -> Result<Json<ExecuteAssetsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let result = state
        .director_client
        .execute(&req.design_doc)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "asset director execution failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(ExecuteAssetsResponse {
        total_tasks: result.total_tasks,
        succeeded: result.succeeded,
        failed: result.failed,
        failures: result.failures,
        asset_registry: result.asset_registry,
    }))
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
    let pipeline_3d_url =
        std::env::var("PIPELINE_3D_URL").unwrap_or_else(|_| "http://localhost:8090".into());
    let audio_client = AudioClient::from_env();
    let director_client = AssetDirectorClient::from_env();

    let state = Arc::new(AppState {
        llm,
        default_model,
        asset_2d,
        pipeline_3d_url,
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(130))
            .build()?,
        audio_client,
        director_client,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/games", post(create_game))
        .route("/api/v1/generate/2d", post(generate_2d))
        .route("/api/v1/generate/2d/health", get(generate_2d_health))
        .route("/api/v1/generate/2d/unload", post(unload_models))
        .route("/generate/3d", post(generate_3d))
        .route("/api/v1/generate/audio", post(generate_audio))
        .route("/api/v1/assets/execute", post(execute_assets))
        .with_state(state);

    let addr = "0.0.0.0:8080";
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
