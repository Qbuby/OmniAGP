use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "omnigp", version, about = "Generate playable games from text descriptions")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate a game from a text description
    Generate(GenerateArgs),
    /// Resume a previously interrupted generation from checkpoint
    Resume(ResumeArgs),
    /// Show generation report for a completed project
    Report(ReportArgs),
    /// Initialize a default omnigp.toml config file
    Init(InitArgs),
}

#[derive(clap::Args, Clone)]
pub struct GenerateArgs {
    /// Game description text
    pub description: String,

    /// Output directory for the generated game
    #[arg(short, long, default_value = "./output")]
    pub output: PathBuf,

    /// Target platform: windows, linux, web
    #[arg(short, long, default_value = "windows")]
    pub platform: String,

    /// Path to config file
    #[arg(short, long, default_value = "omnigp.toml")]
    pub config: PathBuf,

    /// Quality level: low, medium, high
    #[arg(short, long, default_value = "medium")]
    pub quality: String,

    /// Force full regeneration (ignore incremental cache)
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(clap::Args, Clone)]
pub struct ResumeArgs {
    /// Path to the checkpoint directory
    #[arg(short, long)]
    pub checkpoint: PathBuf,

    /// Output directory
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(clap::Args, Clone)]
pub struct ReportArgs {
    /// Path to the output directory containing the generation report
    #[arg(short, long, default_value = "./output")]
    pub path: PathBuf,
}

#[derive(clap::Args, Clone)]
pub struct InitArgs {
    /// Output path for the config file
    #[arg(short, long, default_value = "omnigp.toml")]
    pub output: PathBuf,
}
