use anyhow::Result;
use axum::Router;
use dashmap::DashMap;
use omni_api_gateway::{auth, projects, static_files, state::AppState, websocket};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let llm = omni_llm::LlmClient::from_env("LLM_BASE_URL", "LLM_API_KEY")?;
    let default_model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "qwen2.5-coder-7b".into());
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "omniagp-dev-secret-change-me".into());
    let github_client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_default();
    let github_client_secret = std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_default();

    let (events_tx, _) = broadcast::channel(256);

    let state = Arc::new(AppState {
        llm,
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
        .route("/health", axum::routing::get(|| async { "ok" }))
        .merge(auth::router())
        .merge(projects::router())
        .merge(websocket::router())
        .merge(static_files::router())
        .layer(cors)
        .with_state(state);

    let addr = "0.0.0.0:8080";
    tracing::info!("OmniAGP Dashboard listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
