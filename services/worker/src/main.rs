use anyhow::Result;
use omni_core::{
    TaskPayload, TaskResult, TaskStatus,
    WorkerHeartbeat, WorkerLocation, WorkerRegistration,
};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

mod gpu;
mod executor;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let worker_id = Uuid::new_v4();
    let scheduler_url = std::env::var("SCHEDULER_URL")
        .unwrap_or_else(|_| "http://localhost:8081".into());
    let nats_url = std::env::var("NATS_URL")
        .unwrap_or_else(|_| "nats://localhost:4222".into());
    let location = match std::env::var("WORKER_LOCATION").as_deref() {
        Ok("cloud") => WorkerLocation::Cloud,
        _ => WorkerLocation::Local,
    };

    tracing::info!(%worker_id, "OmniAGP worker starting");

    let gpus = gpu::detect_gpus();
    let capabilities = gpu::infer_capabilities(&gpus);

    tracing::info!(gpu_count = gpus.len(), ?capabilities, "GPU detection complete");

    let registration = WorkerRegistration {
        worker_id,
        hostname: hostname(),
        location,
        gpus: gpus.clone(),
        capabilities: capabilities.clone(),
        labels: std::collections::HashMap::new(),
    };

    let client = reqwest::Client::new();
    client
        .post(format!("{}/api/v1/workers/register", scheduler_url))
        .json(&registration)
        .send()
        .await?;
    tracing::info!("registered with scheduler");

    let nats = async_nats::connect(&nats_url).await?;
    let subject = format!("worker.{}.tasks", worker_id);
    let mut sub = nats.subscribe(subject).await?;
    tracing::info!("subscribed to task subject");

    let heartbeat_client = client.clone();
    let heartbeat_url = scheduler_url.clone();
    let heartbeat_gpus = gpus.clone();
    let heartbeat_worker_id = worker_id;
    tokio::spawn(async move {
        loop {
            let hb = WorkerHeartbeat {
                worker_id: heartbeat_worker_id,
                gpus: heartbeat_gpus.clone(),
                active_tasks: 0,
                timestamp: now_ts(),
            };
            let _ = heartbeat_client
                .post(format!("{}/api/v1/workers/heartbeat", heartbeat_url))
                .json(&hb)
                .send()
                .await;
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });

    tracing::info!("worker ready — waiting for tasks");

    loop {
        tokio::select! {
            Some(msg) = async { use futures::StreamExt; sub.next().await } => {
                match serde_json::from_slice::<TaskPayload>(&msg.payload) {
                    Ok(task) => {
                        let task_id = task.id;
                        tracing::info!(%task_id, "received task");
                        let start = std::time::Instant::now();

                        let result = executor::execute_task(&task).await;
                        let duration = start.elapsed();

                        let task_result = TaskResult {
                            task_id,
                            worker_id,
                            status: if result.is_ok() { TaskStatus::Completed } else { TaskStatus::Failed },
                            output: result.as_ref().ok().cloned(),
                            error: result.as_ref().err().map(|e| e.to_string()),
                            duration_ms: duration.as_millis() as u64,
                            gpu_minutes: duration.as_secs_f64() / 60.0,
                        };

                        let _ = nats.publish("tasks.results".to_string(), serde_json::to_vec(&task_result)?.into()).await;
                        tracing::info!(%task_id, duration_ms = duration.as_millis(), "task completed");
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "failed to deserialize task");
                    }
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("shutting down");
                break;
            }
        }
    }

    Ok(())
}

fn hostname() -> String {
    std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into())
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
