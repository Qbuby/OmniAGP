mod cli;
mod config;
mod pipeline;
mod progress;
mod checkpoint;
mod report;
mod export;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    match cli.command {
        Commands::Generate(args) => pipeline::run_generate(args).await,
        Commands::Resume(args) => pipeline::run_resume(args).await,
        Commands::Report(args) => report::show_report(args).await,
        Commands::Init(args) => config::init_config(args).await,
    }
}
