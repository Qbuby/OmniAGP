use anyhow::Result;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post, delete},
    Json, Router,
};
use omni_marketplace::{
    asset_pack::AssetPackManifest,
    community::ContributionType,
    search::SearchFilter,
    store::MarketplaceStore,
};
use omni_plugin::{PluginManifest, PluginRegistry};
use omni_templates::TemplateEngine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

struct AppState {
    marketplace: RwLock<MarketplaceStore>,
    plugin_registry: RwLock<PluginRegistry>,
    template_engine: RwLock<TemplateEngine>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    asset_type: Option<String>,
    tags: Option<String>,
    min_rating: Option<f32>,
    sort: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

fn api_ok<T: Serialize>(data: T) -> Json<ApiResponse> {
    Json(ApiResponse {
        success: true,
        data: Some(serde_json::to_value(data).unwrap_or_default()),
        error: None,
    })
}

fn api_err(msg: String) -> Json<ApiResponse> {
    Json(ApiResponse {
        success: false,
        data: None,
        error: Some(msg),
    })
}

async fn health() -> &'static str {
    "ok"
}

// --- Plugin endpoints ---

#[derive(Serialize)]
struct PluginListItem {
    name: String,
    version: String,
    plugin_type: String,
    description: String,
}

async fn list_plugins(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let registry = state.plugin_registry.read().await;
    let plugins: Vec<PluginListItem> = registry
        .list_all()
        .iter()
        .map(|p| PluginListItem {
            name: p.manifest.name.clone(),
            version: p.manifest.version.clone(),
            plugin_type: format!("{:?}", p.manifest.plugin_type),
            description: p.manifest.description.clone(),
        })
        .collect();
    api_ok(plugins)
}

#[derive(Deserialize)]
struct InstallPluginRequest {
    manifest: PluginManifest,
    source_path: String,
}

async fn install_plugin(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InstallPluginRequest>,
) -> impl IntoResponse {
    let mut registry = state.plugin_registry.write().await;
    let source = PathBuf::from(&req.source_path);
    match registry.install(req.manifest, &source) {
        Ok(()) => api_ok("plugin installed"),
        Err(e) => api_err(format!("{}", e)),
    }
}

async fn uninstall_plugin(
    State(state): State<Arc<AppState>>,
    Path((name, version)): Path<(String, String)>,
) -> impl IntoResponse {
    let mut registry = state.plugin_registry.write().await;
    match registry.uninstall(&name, &version) {
        Ok(()) => api_ok("plugin uninstalled"),
        Err(e) => api_err(format!("{}", e)),
    }
}

// --- Template endpoints ---

#[derive(Serialize)]
struct TemplateListItem {
    id: String,
    name: String,
    description: String,
    category: String,
}

async fn list_templates(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let engine = state.template_engine.read().await;
    let templates: Vec<TemplateListItem> = engine
        .list_templates()
        .iter()
        .map(|t| TemplateListItem {
            id: t.id.clone(),
            name: t.name.clone(),
            description: t.description.clone(),
            category: format!("{:?}", t.category),
        })
        .collect();
    api_ok(templates)
}

#[derive(Deserialize)]
struct InstantiateTemplateRequest {
    template_id: String,
    params: std::collections::HashMap<String, serde_json::Value>,
    output_dir: String,
}

async fn instantiate_template(
    State(state): State<Arc<AppState>>,
    Json(req): Json<InstantiateTemplateRequest>,
) -> impl IntoResponse {
    let engine = state.template_engine.read().await;
    let output = PathBuf::from(&req.output_dir);
    match engine.instantiate(&req.template_id, req.params, &output) {
        Ok(instance) => api_ok(instance),
        Err(e) => api_err(format!("{}", e)),
    }
}

// --- Asset pack endpoints ---

async fn search_asset_packs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let store = state.marketplace.read().await;
    let filter = SearchFilter {
        query: query.q,
        tags: query
            .tags
            .map(|t| t.split(',').map(String::from).collect())
            .unwrap_or_default(),
        min_rating: query.min_rating,
        page: query.page.unwrap_or(0),
        per_page: query.per_page.unwrap_or(20),
        ..Default::default()
    };
    let results = store.search_asset_packs(&filter);
    api_ok(results)
}

#[derive(Deserialize)]
struct UploadAssetPackRequest {
    manifest: AssetPackManifest,
    file_path: String,
}

async fn upload_asset_pack(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UploadAssetPackRequest>,
) -> impl IntoResponse {
    let mut store = state.marketplace.write().await;
    let path = PathBuf::from(&req.file_path);
    match store.upload_asset_pack(req.manifest, &path) {
        Ok(id) => api_ok(id.to_string()),
        Err(e) => api_err(format!("{}", e)),
    }
}

#[derive(Deserialize)]
struct DownloadRequest {
    dest_dir: String,
}

async fn download_asset_pack(
    State(state): State<Arc<AppState>>,
    Path(pack_id): Path<Uuid>,
    Json(req): Json<DownloadRequest>,
) -> impl IntoResponse {
    let mut store = state.marketplace.write().await;
    let dest = PathBuf::from(&req.dest_dir);
    match store.download_asset_pack(pack_id, &dest) {
        Ok(path) => api_ok(path.to_string_lossy().to_string()),
        Err(e) => api_err(format!("{}", e)),
    }
}

#[derive(Deserialize)]
struct RateRequest {
    score: f32,
}

async fn rate_asset_pack(
    State(state): State<Arc<AppState>>,
    Path(pack_id): Path<Uuid>,
    Json(req): Json<RateRequest>,
) -> impl IntoResponse {
    let mut store = state.marketplace.write().await;
    match store.rate_asset_pack(pack_id, req.score) {
        Ok(()) => api_ok("rated"),
        Err(e) => api_err(format!("{}", e)),
    }
}

// --- Contribution endpoints ---

#[derive(Deserialize)]
struct SubmitContributionRequest {
    contributor_id: Uuid,
    contribution_type: String,
    name: String,
    version: String,
}

async fn submit_contribution(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SubmitContributionRequest>,
) -> impl IntoResponse {
    let mut store = state.marketplace.write().await;
    let ctype = match req.contribution_type.as_str() {
        "plugin" => ContributionType::Plugin,
        "template" => ContributionType::Template,
        "asset_pack" => ContributionType::AssetPack,
        _ => return api_err("invalid contribution type".to_string()),
    };
    let contribution = store.submit_contribution(req.contributor_id, ctype, req.name, req.version);
    api_ok(format!("submitted: {}", contribution.id))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let storage_dir = PathBuf::from(
        std::env::var("MARKETPLACE_STORAGE").unwrap_or_else(|_| "./data/marketplace".into()),
    );
    let plugins_dir = PathBuf::from(
        std::env::var("PLUGINS_DIR").unwrap_or_else(|_| "./data/plugins".into()),
    );
    let templates_dir = PathBuf::from(
        std::env::var("TEMPLATES_DIR").unwrap_or_else(|_| "./templates".into()),
    );

    std::fs::create_dir_all(&storage_dir)?;
    std::fs::create_dir_all(&plugins_dir)?;

    let mut template_engine = TemplateEngine::new(templates_dir);
    template_engine.load_templates()?;

    let state = Arc::new(AppState {
        marketplace: RwLock::new(MarketplaceStore::new(storage_dir)),
        plugin_registry: RwLock::new(PluginRegistry::new(
            plugins_dir.clone(),
            plugins_dir.join("data"),
        )),
        template_engine: RwLock::new(template_engine),
    });

    let app = Router::new()
        .route("/health", get(health))
        // Plugin routes
        .route("/api/v1/plugins", get(list_plugins))
        .route("/api/v1/plugins/install", post(install_plugin))
        .route("/api/v1/plugins/{name}/{version}", delete(uninstall_plugin))
        // Template routes
        .route("/api/v1/templates", get(list_templates))
        .route("/api/v1/templates/instantiate", post(instantiate_template))
        // Asset pack routes
        .route("/api/v1/assets", get(search_asset_packs))
        .route("/api/v1/assets/upload", post(upload_asset_pack))
        .route("/api/v1/assets/{pack_id}/download", post(download_asset_pack))
        .route("/api/v1/assets/{pack_id}/rate", post(rate_asset_pack))
        // Contribution routes
        .route("/api/v1/contributions", post(submit_contribution))
        .with_state(state);

    let addr = std::env::var("MARKETPLACE_ADDR").unwrap_or_else(|_| "0.0.0.0:8081".into());
    tracing::info!("marketplace service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
