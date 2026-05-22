use anyhow::Result;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Deserialize)]
struct AudioGenRequest {
    prompt: String,
    audio_type: String,
    duration_sec: Option<f64>,
    output_dir: Option<String>,
}

#[derive(Serialize)]
struct AudioGenResponse {
    file_path: String,
    audio_type: String,
    duration_sec: f64,
    sample_rate: u32,
    loop_point_samples: Option<u64>,
    validation: AudioValidation,
}

#[derive(Serialize)]
struct AudioValidation {
    valid: bool,
    issues: Vec<String>,
}

#[derive(Deserialize)]
struct DirectorRequest {
    design_doc: serde_json::Value,
}

#[derive(Serialize)]
struct DirectorResponse {
    total_tasks: u32,
    succeeded: u32,
    failed: u32,
    failures: Vec<serde_json::Value>,
    asset_registry: serde_json::Value,
}

async fn mock_audio_generate(Json(req): Json<AudioGenRequest>) -> Json<AudioGenResponse> {
    let duration = req.duration_sec.unwrap_or(5.0);
    let output_dir = req.output_dir.unwrap_or_else(|| ".".into());
    let filename = format!("{}/mock_{}.ogg", output_dir, req.audio_type);
    let is_bgm = req.audio_type == "bgm";

    Json(AudioGenResponse {
        file_path: filename,
        audio_type: req.audio_type,
        duration_sec: duration,
        sample_rate: 44100,
        loop_point_samples: if is_bgm { Some(0) } else { None },
        validation: AudioValidation {
            valid: true,
            issues: vec![],
        },
    })
}

async fn mock_director_execute(Json(_req): Json<DirectorRequest>) -> Json<DirectorResponse> {
    Json(DirectorResponse {
        total_tasks: 5,
        succeeded: 5,
        failed: 0,
        failures: vec![],
        asset_registry: serde_json::json!({
            "sprites": ["player_knight.png", "slime.png", "dragon.png"],
            "tilesets": ["dungeon_tileset.png"],
            "audio": ["bgm_level.ogg", "bgm_boss.ogg", "sfx_jump.ogg", "sfx_attack.ogg", "sfx_dragon_roar.ogg"],
        }),
    })
}

pub async fn start_mock_services() -> Result<(SocketAddr, SocketAddr)> {
    let audio_app = Router::new().route("/generate", post(mock_audio_generate));
    let director_app = Router::new().route("/execute", post(mock_director_execute));

    let audio_listener = TcpListener::bind("127.0.0.1:0").await?;
    let audio_addr = audio_listener.local_addr()?;

    let director_listener = TcpListener::bind("127.0.0.1:0").await?;
    let director_addr = director_listener.local_addr()?;

    tokio::spawn(async move {
        axum::serve(audio_listener, audio_app).await.ok();
    });

    tokio::spawn(async move {
        axum::serve(director_listener, director_app).await.ok();
    });

    info!(audio = %audio_addr, director = %director_addr, "mock services started");
    Ok((audio_addr, director_addr))
}
