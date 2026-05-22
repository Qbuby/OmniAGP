use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct WsQuery {
    pub project_id: Option<Uuid>,
    pub token: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/api/v1/ws", get(ws_handler))
}

async fn ws_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let validation = jsonwebtoken::decode::<crate::auth::Claims>(
        &query.token,
        &jsonwebtoken::DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &jsonwebtoken::Validation::default(),
    );

    if validation.is_err() {
        return axum::http::StatusCode::UNAUTHORIZED.into_response();
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state, query.project_id))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, project_filter: Option<Uuid>) {
    let mut rx = state.pipeline_events.subscribe();

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            event = rx.recv() => {
                match event {
                    Ok(ev) => {
                        if let Some(filter_id) = project_filter {
                            if ev.project_id != filter_id {
                                continue;
                            }
                        }
                        let json = serde_json::to_string(&ev).unwrap_or_default();
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        }
    }
}
