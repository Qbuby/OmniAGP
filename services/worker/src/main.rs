use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    tracing::info!("OmniAGP worker starting");

    // Worker will poll a task queue and execute pipeline steps
    // For now, just keep alive as a placeholder
    tracing::info!("worker ready — waiting for tasks");
    tokio::signal::ctrl_c().await?;
    tracing::info!("shutting down");
    Ok(())
}
