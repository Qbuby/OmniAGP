use anyhow::Result;
use omni_assets::{AudioClient, AudioType};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum WorkerTask {
    GenerateAudio {
        prompt: String,
        audio_type: String,
        duration_sec: Option<f64>,
        output_dir: Option<String>,
    },
    GenerateSprite {
        prompt: String,
        output_dir: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskResult {
    success: bool,
    file_path: Option<String>,
    error: Option<String>,
}

async fn process_audio_task(
    client: &AudioClient,
    prompt: &str,
    audio_type_str: &str,
    duration_sec: Option<f64>,
    output_dir: Option<&str>,
) -> TaskResult {
    let audio_type = match audio_type_str {
        "bgm" => AudioType::Bgm,
        "sfx" => AudioType::Sfx,
        _ => {
            return TaskResult {
                success: false,
                file_path: None,
                error: Some(format!("invalid audio_type: {}", audio_type_str)),
            };
        }
    };

    match client.generate(prompt, audio_type, duration_sec, output_dir).await {
        Ok(resp) => TaskResult {
            success: resp.validation.valid,
            file_path: Some(resp.file_path),
            error: if resp.validation.issues.is_empty() {
                None
            } else {
                Some(resp.validation.issues.join(", "))
            },
        },
        Err(e) => TaskResult {
            success: false,
            file_path: None,
            error: Some(e.to_string()),
        },
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

    info!("OmniAGP worker starting");

    let audio_client = AudioClient::from_env();

    match audio_client.health().await {
        Ok(true) => info!("audio pipeline connected"),
        Ok(false) => info!("audio pipeline returned unhealthy status"),
        Err(e) => info!(error = %e, "audio pipeline not reachable (will retry on task)"),
    }

    info!("worker ready — waiting for tasks");
    tokio::signal::ctrl_c().await?;
    info!("shutting down");
    Ok(())
}
