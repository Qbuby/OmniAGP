use anyhow::Result;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    let llm_base_url =
        std::env::var("LLM_BASE_URL").unwrap_or_else(|_| "http://localhost:11434/v1".to_string());
    let api_key = std::env::var("LLM_API_KEY").unwrap_or_default();

    let game_description = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "做一个简单的2D跳跃demo".to_string());

    let project_dir = std::env::var("PROJECT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./output"));

    info!(description = %game_description, "starting OmniAGP orchestrator");

    let result = omni_orchestrator::run_full_pipeline(
        &llm_base_url,
        &api_key,
        &game_description,
        &project_dir,
    )
    .await?;

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
