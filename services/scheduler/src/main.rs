use anyhow::Result;
use axum::{extract::State, routing::{get, post}, Json, Router};
use omni_core::{WorkerHeartbeat, WorkerRegistration, WorkerStatus};
use omni_scheduler::Scheduler;
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    workers_online: usize,
}

#[derive(Serialize)]
struct NodeListResponse {
    nodes: Vec<omni_core::WorkerNode>,
}

async fn health(State(scheduler): State<Arc<Scheduler>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        workers_online: scheduler.nodes.online_nodes().len(),
    })
}

async fn register_worker(
    State(scheduler): State<Arc<Scheduler>>,
    Json(reg): Json<WorkerRegistration>,
) -> Json<omni_core::WorkerNode> {
    let node = scheduler.nodes.register(reg);
    scheduler.metrics.online_workers.set(scheduler.nodes.online_nodes().len() as i64);
    Json(node)
}

async fn worker_heartbeat(
    State(scheduler): State<Arc<Scheduler>>,
    Json(hb): Json<WorkerHeartbeat>,
) -> &'static str {
    scheduler.nodes.heartbeat(hb);
    "ok"
}

async fn list_nodes(State(scheduler): State<Arc<Scheduler>>) -> Json<NodeListResponse> {
    Json(NodeListResponse {
        nodes: scheduler.nodes.all_nodes(),
    })
}

async fn set_node_offline(
    State(scheduler): State<Arc<Scheduler>>,
    axum::extract::Path(worker_id): axum::extract::Path<Uuid>,
) -> &'static str {
    scheduler.nodes.set_status(worker_id, WorkerStatus::Offline);
    "ok"
}

async fn set_node_online(
    State(scheduler): State<Arc<Scheduler>>,
    axum::extract::Path(worker_id): axum::extract::Path<Uuid>,
) -> &'static str {
    scheduler.nodes.set_status(worker_id, WorkerStatus::Online);
    "ok"
}

async fn metrics_handler(State(scheduler): State<Arc<Scheduler>>) -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = scheduler.metrics.registry.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let nats_url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".into());
    let scheduler = Arc::new(Scheduler::new(&nats_url).await?);

    let dispatch_scheduler = scheduler.clone();
    tokio::spawn(async move { dispatch_scheduler.run_dispatch_loop().await });

    let health_scheduler = scheduler.clone();
    tokio::spawn(async move {
        loop {
            health_scheduler.nodes.check_health();
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/workers/register", post(register_worker))
        .route("/api/v1/workers/heartbeat", post(worker_heartbeat))
        .route("/api/v1/workers", get(list_nodes))
        .route("/api/v1/workers/{id}/offline", post(set_node_offline))
        .route("/api/v1/workers/{id}/online", post(set_node_online))
        .route("/metrics", get(metrics_handler))
        .with_state(scheduler);

    let addr = "0.0.0.0:8081";
    tracing::info!("scheduler service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
