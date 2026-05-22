use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use rust_embed::RustEmbed;
use std::sync::Arc;

use crate::state::AppState;

#[derive(RustEmbed)]
#[folder = "../../web/dist"]
#[allow(dead_code)]
struct WebAssets;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().fallback(get(static_handler))
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == "index.html" {
        return serve_index();
    }

    match WebAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime.as_ref())
                .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
                .body(Body::from(content.data.to_vec()))
                .unwrap()
        }
        None => serve_index(),
    }
}

fn serve_index() -> Response<Body> {
    match WebAssets::get("index.html") {
        Some(content) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header(header::CACHE_CONTROL, "no-cache")
            .body(Body::from(content.data.to_vec()))
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(
                "<html><body><h1>OmniAGP Dashboard</h1><p>Frontend not built. Run: cd web && npm run build</p></body></html>",
            ))
            .unwrap(),
    }
}
