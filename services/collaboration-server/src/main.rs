use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    response::IntoResponse,
    routing::{get, post, delete},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use omni_auth::{AccessControl, Permission, Role, ProjectMember};
use omni_collaboration::{
    CollaborationSession, CrdtOpType, CrdtOperation, GddMetadata, VersionStore,
};
use omni_notify::{EventBus, EventType, NotificationEvent, WebhookConfig};
use omni_workflow::{ApprovalConfig, ReviewDecision, WorkflowEngine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

struct AppState {
    versions: RwLock<VersionStore>,
    workflows: RwLock<WorkflowEngine>,
    access_control: RwLock<AccessControl>,
    event_bus: RwLock<EventBus>,
    sessions: RwLock<HashMap<Uuid, CollaborationSession>>,
    broadcast: broadcast::Sender<BroadcastMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum BroadcastMessage {
    Operation(CrdtOperation),
    CursorUpdate { user_id: Uuid, section: String, field_path: String },
    UserJoined { user_id: Uuid, gdd_id: Uuid },
    UserLeft { user_id: Uuid, gdd_id: Uuid },
}

#[derive(Deserialize)]
struct CreateProjectRequest {
    title: String,
    genre: String,
    target_platform: String,
    description: String,
    owner_id: Uuid,
}

#[derive(Serialize)]
struct CreateProjectResponse {
    gdd_id: Uuid,
    project_id: Uuid,
}

async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProjectRequest>,
) -> Json<CreateProjectResponse> {
    let project_id = Uuid::new_v4();
    let metadata = GddMetadata {
        title: req.title,
        genre: req.genre,
        target_platform: req.target_platform,
        description: req.description,
    };

    let doc = state.versions.write().await.create_document(project_id, metadata);

    let mut ac = state.access_control.write().await;
    ac.add_member(ProjectMember {
        user_id: req.owner_id,
        project_id,
        role: Role::Owner,
        invited_by: req.owner_id,
        joined_at: chrono::Utc::now(),
    });

    let mut wf = state.workflows.write().await;
    wf.create_workflow(doc.id, ApprovalConfig::default());

    Json(CreateProjectResponse {
        gdd_id: doc.id,
        project_id,
    })
}

#[derive(Deserialize)]
struct InviteMemberRequest {
    user_id: Uuid,
    role: Role,
    invited_by: Uuid,
}

async fn invite_member(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Json(req): Json<InviteMemberRequest>,
) -> Json<serde_json::Value> {
    let ac = state.access_control.read().await;
    if !ac.check_permission(req.invited_by, project_id, Permission::ManageMembers) {
        return Json(serde_json::json!({"error": "insufficient permissions"}));
    }
    drop(ac);

    let mut ac = state.access_control.write().await;
    ac.add_member(ProjectMember {
        user_id: req.user_id,
        project_id,
        role: req.role,
        invited_by: req.invited_by,
        joined_at: chrono::Utc::now(),
    });

    let event_bus = state.event_bus.read().await;
    event_bus
        .emit(NotificationEvent::new(
            project_id,
            EventType::MemberInvited,
            "New member invited".into(),
            format!("User {} was invited with role {:?}", req.user_id, req.role),
        ))
        .await;

    Json(serde_json::json!({"status": "ok"}))
}

async fn remove_member(
    State(state): State<Arc<AppState>>,
    Path((project_id, user_id)): Path<(Uuid, Uuid)>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let removed_by: Uuid = params
        .get("removed_by")
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();

    let mut ac = state.access_control.write().await;
    ac.remove_member(user_id, project_id, removed_by);
    Json(serde_json::json!({"status": "ok"}))
}

async fn get_members(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
) -> Json<serde_json::Value> {
    let ac = state.access_control.read().await;
    let members: Vec<_> = ac
        .get_project_members(project_id)
        .iter()
        .map(|m| serde_json::json!({"user_id": m.user_id, "role": m.role}))
        .collect();
    Json(serde_json::json!({"members": members}))
}

#[derive(Deserialize)]
struct SaveVersionRequest {
    author_id: Uuid,
    message: String,
    branch: Option<String>,
}

async fn save_version(
    State(state): State<Arc<AppState>>,
    Path(gdd_id): Path<Uuid>,
    Json(req): Json<SaveVersionRequest>,
) -> Json<serde_json::Value> {
    let branch = req.branch.unwrap_or_else(|| "main".into());
    let mut store = state.versions.write().await;
    match store.save_version(gdd_id, req.author_id, req.message, &branch) {
        Some(version) => Json(serde_json::json!({
            "version_id": version.id,
            "version_number": version.version_number,
        })),
        None => Json(serde_json::json!({"error": "GDD not found"})),
    }
}

async fn list_versions(
    State(state): State<Arc<AppState>>,
    Path(gdd_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let branch = params.get("branch").cloned().unwrap_or_else(|| "main".into());
    let store = state.versions.read().await;
    let versions: Vec<_> = store
        .list_versions(gdd_id, &branch)
        .iter()
        .map(|v| serde_json::json!({
            "id": v.id,
            "version_number": v.version_number,
            "author_id": v.author_id,
            "message": v.message,
            "created_at": v.created_at,
        }))
        .collect();
    Json(serde_json::json!({"versions": versions}))
}

#[derive(Deserialize)]
struct DiffRequest {
    from_version: Uuid,
    to_version: Uuid,
}

async fn diff_versions(
    State(state): State<Arc<AppState>>,
    Path(_gdd_id): Path<Uuid>,
    Json(req): Json<DiffRequest>,
) -> Json<serde_json::Value> {
    let store = state.versions.read().await;
    match store.diff(&req.from_version, &req.to_version) {
        Some(diff) => Json(serde_json::json!({"diff": diff.entries})),
        None => Json(serde_json::json!({"error": "version not found"})),
    }
}

#[derive(Deserialize)]
struct RollbackRequest {
    version_id: Uuid,
}

async fn rollback(
    State(state): State<Arc<AppState>>,
    Path(_gdd_id): Path<Uuid>,
    Json(req): Json<RollbackRequest>,
) -> Json<serde_json::Value> {
    let mut store = state.versions.write().await;
    match store.rollback(&req.version_id) {
        Some(_) => Json(serde_json::json!({"status": "rolled back"})),
        None => Json(serde_json::json!({"error": "version not found"})),
    }
}

#[derive(Deserialize)]
struct CreateBranchRequest {
    branch_name: String,
    from_version: Uuid,
}

async fn create_branch(
    State(state): State<Arc<AppState>>,
    Path(gdd_id): Path<Uuid>,
    Json(req): Json<CreateBranchRequest>,
) -> Json<serde_json::Value> {
    let mut store = state.versions.write().await;
    match store.create_branch(gdd_id, &req.branch_name, req.from_version) {
        Some(branch) => Json(serde_json::json!({
            "branch": branch.name,
            "head_version": branch.head_version,
        })),
        None => Json(serde_json::json!({"error": "version not found"})),
    }
}

#[derive(Deserialize)]
struct SubmitReviewRequest {
    requested_by: Uuid,
    version_id: Uuid,
}

async fn submit_for_review(
    State(state): State<Arc<AppState>>,
    Path(gdd_id): Path<Uuid>,
    Json(req): Json<SubmitReviewRequest>,
) -> Json<serde_json::Value> {
    let mut wf = state.workflows.write().await;
    if let Some(workflow) = wf.get_mut(&gdd_id) {
        match workflow.submit_for_review(req.requested_by, req.version_id) {
            Ok(()) => {
                let event_bus = state.event_bus.read().await;
                let store = state.versions.read().await;
                let project_id = store
                    .get_document(&gdd_id)
                    .map(|d| d.project_id)
                    .unwrap_or_default();
                event_bus
                    .emit(NotificationEvent::new(
                        project_id,
                        EventType::ReviewRequested,
                        "Review requested".into(),
                        format!("GDD {} submitted for review", gdd_id),
                    ))
                    .await;
                Json(serde_json::json!({"status": "in_review"}))
            }
            Err(e) => Json(serde_json::json!({"error": e.to_string()})),
        }
    } else {
        Json(serde_json::json!({"error": "workflow not found"}))
    }
}

#[derive(Deserialize)]
struct ReviewSubmitRequest {
    reviewer_id: Uuid,
    decision: ReviewDecision,
    comment: Option<String>,
}

async fn submit_review(
    State(state): State<Arc<AppState>>,
    Path(gdd_id): Path<Uuid>,
    Json(req): Json<ReviewSubmitRequest>,
) -> Json<serde_json::Value> {
    let mut wf = state.workflows.write().await;
    if let Some(workflow) = wf.get_mut(&gdd_id) {
        match workflow.submit_review(req.reviewer_id, req.decision, req.comment) {
            Ok(()) => {
                if workflow.try_approve(req.reviewer_id).is_ok() {
                    let event_bus = state.event_bus.read().await;
                    let store = state.versions.read().await;
                    let project_id = store
                        .get_document(&gdd_id)
                        .map(|d| d.project_id)
                        .unwrap_or_default();
                    event_bus
                        .emit(NotificationEvent::new(
                            project_id,
                            EventType::ReviewApproved,
                            "Review approved".into(),
                            format!("GDD {} has been approved", gdd_id),
                        ))
                        .await;
                    return Json(serde_json::json!({"status": "approved"}));
                }
                Json(serde_json::json!({"status": "review_submitted"}))
            }
            Err(e) => Json(serde_json::json!({"error": e.to_string()})),
        }
    } else {
        Json(serde_json::json!({"error": "workflow not found"}))
    }
}

async fn get_workflow_status(
    State(state): State<Arc<AppState>>,
    Path(gdd_id): Path<Uuid>,
) -> Json<serde_json::Value> {
    let wf = state.workflows.read().await;
    match wf.get(&gdd_id) {
        Some(workflow) => Json(serde_json::json!({
            "status": workflow.status,
            "history": workflow.history,
        })),
        None => Json(serde_json::json!({"error": "workflow not found"})),
    }
}

#[derive(Deserialize)]
struct RegisterWebhookRequest {
    project_id: Uuid,
    url: String,
    events: Vec<EventType>,
    adapter_type: omni_notify::AdapterType,
}

async fn register_webhook(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterWebhookRequest>,
) -> Json<serde_json::Value> {
    let config = WebhookConfig {
        id: Uuid::new_v4(),
        project_id: req.project_id,
        url: req.url,
        events: req.events,
        adapter_type: req.adapter_type,
        enabled: true,
    };
    let id = config.id;
    state.event_bus.write().await.register_webhook(config);
    Json(serde_json::json!({"webhook_id": id}))
}

async fn delete_webhook(
    State(state): State<Arc<AppState>>,
    Path(webhook_id): Path<Uuid>,
) -> Json<serde_json::Value> {
    state.event_bus.write().await.remove_webhook(webhook_id);
    Json(serde_json::json!({"status": "deleted"}))
}

async fn get_feed(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let limit: usize = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);
    let format = params.get("format").cloned().unwrap_or_else(|| "json".into());

    let event_bus = state.event_bus.read().await;
    let entries = event_bus.get_feed(project_id, limit).await;

    if format == "atom" {
        let atom = event_bus.generate_atom_feed(&entries, "OmniAGP Project");
        axum::response::Response::builder()
            .header("content-type", "application/atom+xml")
            .body(atom)
            .unwrap()
            .into_response()
    } else {
        Json(serde_json::json!({"entries": entries})).into_response()
    }
}

async fn get_audit_log(
    State(state): State<Arc<AppState>>,
    Path(project_id): Path<Uuid>,
) -> Json<serde_json::Value> {
    let ac = state.access_control.read().await;
    let log: Vec<_> = ac
        .get_audit_log(project_id)
        .iter()
        .map(|e| serde_json::json!({
            "id": e.id,
            "user_id": e.user_id,
            "action": e.action,
            "details": e.details,
            "timestamp": e.timestamp,
        }))
        .collect();
    Json(serde_json::json!({"audit_log": log}))
}

#[derive(Deserialize)]
struct WsQuery {
    user_id: Uuid,
    gdd_id: Uuid,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state, query.user_id, query.gdd_id))
}

async fn handle_ws(socket: WebSocket, state: Arc<AppState>, user_id: Uuid, gdd_id: Uuid) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.broadcast.subscribe();

    {
        let mut sessions = state.sessions.write().await;
        let session = sessions.entry(gdd_id).or_insert_with(|| CollaborationSession::new(gdd_id));
        session.join(user_id);
    }

    let _ = state.broadcast.send(BroadcastMessage::UserJoined { user_id, gdd_id });

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let text = serde_json::to_string(&msg).unwrap_or_default();
            if sender.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    let state_clone = state.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                let text_str: &str = &text;
                if let Ok(op) = serde_json::from_str::<ClientMessage>(text_str) {
                    match op {
                        ClientMessage::Operation { path, op_type, value } => {
                            let crdt_op = CrdtOperation {
                                id: Uuid::new_v4(),
                                author_id: user_id,
                                gdd_id,
                                op_type,
                                path,
                                value,
                                timestamp: chrono::Utc::now(),
                                clock: 0,
                            };
                            let mut sessions = state_clone.sessions.write().await;
                            if let Some(session) = sessions.get_mut(&gdd_id) {
                                let applied = session.apply_operation(crdt_op);
                                let _ = state_clone.broadcast.send(BroadcastMessage::Operation(applied));
                            }
                        }
                        ClientMessage::CursorMove { section, field_path } => {
                            let mut sessions = state_clone.sessions.write().await;
                            if let Some(session) = sessions.get_mut(&gdd_id) {
                                session.update_cursor(user_id, section.clone(), field_path.clone());
                            }
                            let _ = state_clone.broadcast.send(BroadcastMessage::CursorUpdate {
                                user_id,
                                section,
                                field_path,
                            });
                        }
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    {
        let mut sessions = state.sessions.write().await;
        if let Some(session) = sessions.get_mut(&gdd_id) {
            session.leave(&user_id);
        }
    }
    let _ = state.broadcast.send(BroadcastMessage::UserLeft { user_id, gdd_id });
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    Operation {
        path: String,
        op_type: CrdtOpType,
        value: serde_json::Value,
    },
    CursorMove {
        section: String,
        field_path: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let (tx, _) = broadcast::channel(1024);

    let state = Arc::new(AppState {
        versions: RwLock::new(VersionStore::new()),
        workflows: RwLock::new(WorkflowEngine::new()),
        access_control: RwLock::new(AccessControl::new()),
        event_bus: RwLock::new(EventBus::new()),
        sessions: RwLock::new(HashMap::new()),
        broadcast: tx,
    });

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(ws_handler))
        // Project management
        .route("/api/v1/projects", post(create_project))
        .route("/api/v1/projects/{project_id}/members", post(invite_member))
        .route("/api/v1/projects/{project_id}/members", get(get_members))
        .route("/api/v1/projects/{project_id}/members/{user_id}", delete(remove_member))
        .route("/api/v1/projects/{project_id}/audit", get(get_audit_log))
        .route("/api/v1/projects/{project_id}/feed", get(get_feed))
        // Version control
        .route("/api/v1/gdd/{gdd_id}/versions", post(save_version))
        .route("/api/v1/gdd/{gdd_id}/versions", get(list_versions))
        .route("/api/v1/gdd/{gdd_id}/diff", post(diff_versions))
        .route("/api/v1/gdd/{gdd_id}/rollback", post(rollback))
        .route("/api/v1/gdd/{gdd_id}/branches", post(create_branch))
        // Workflow
        .route("/api/v1/gdd/{gdd_id}/review", post(submit_for_review))
        .route("/api/v1/gdd/{gdd_id}/review/submit", post(submit_review))
        .route("/api/v1/gdd/{gdd_id}/workflow", get(get_workflow_status))
        // Webhooks
        .route("/api/v1/webhooks", post(register_webhook))
        .route("/api/v1/webhooks/{webhook_id}", delete(delete_webhook))
        .with_state(state);

    let addr = "0.0.0.0:8081";
    tracing::info!("collaboration server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
