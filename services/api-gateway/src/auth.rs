use axum::{
    extract::{Json, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Deserialize)]
pub struct GithubCallbackQuery {
    pub code: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Serialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GithubTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GithubUser {
    id: u64,
    login: String,
    avatar_url: Option<String>,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/auth/github", get(github_login))
        .route("/api/v1/auth/github/callback", get(github_callback))
        .route("/api/v1/auth/me", get(get_me))
}

async fn github_login(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&scope=read:user",
        state.github_client_id
    );
    (StatusCode::TEMPORARY_REDIRECT, [(header::LOCATION, url)])
}

async fn github_callback(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<GithubCallbackQuery>,
) -> Result<Json<AuthResponse>, StatusCode> {
    let client = reqwest::Client::new();

    let token_resp = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", state.github_client_id.as_str()),
            ("client_secret", state.github_client_secret.as_str()),
            ("code", query.code.as_str()),
        ])
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let github_token: GithubTokenResponse = token_resp
        .json()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let user_resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", github_token.access_token))
        .header("User-Agent", "OmniAGP")
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let github_user: GithubUser = user_resp
        .json()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: github_user.id.to_string(),
        name: github_user.login.clone(),
        avatar_url: github_user.avatar_url.clone(),
        exp: now + 86400 * 7,
        iat: now,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AuthResponse {
        token,
        user: UserInfo {
            id: github_user.id.to_string(),
            name: github_user.login,
            avatar_url: github_user.avatar_url,
        },
    }))
}

async fn get_me(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<UserInfo>, StatusCode> {
    let claims = extract_claims(&state.jwt_secret, &headers)?;
    Ok(Json(UserInfo {
        id: claims.sub,
        name: claims.name,
        avatar_url: claims.avatar_url,
    }))
}

pub fn extract_claims(
    jwt_secret: &str,
    headers: &axum::http::HeaderMap,
) -> Result<Claims, StatusCode> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(token_data.claims)
}
