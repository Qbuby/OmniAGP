use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use omni_assets::{AudioClient, AudioType};
use omni_core::{AssetQuality, GameEngine, GameProject, LlmProviderConfig, PipelineConfig, ProjectStatus};
use omni_llm::LlmClient;
use omni_orchestrator::Pipeline;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

struct AppState {
    llm: LlmClient,
    default_model: String,
    audio_client: AudioClient,
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
    let audio_client = AudioClient::from_env();

    let state = Arc::new(AppState {
        llm,
        default_model,
        audio_client,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/games", post(create_game))
        .route("/api/v1/generate/audio", post(generate_audio))
        .with_state(state);

    let addr = "0.0.0.0:8080";
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
