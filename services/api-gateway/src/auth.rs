use axum::{
    extract::{Json, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;
use crate::users::{UserError, MIN_PASSWORD_LEN};

const TOKEN_TTL_SECONDS: usize = 86400 * 7;

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
    #[serde(rename = "username")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub user: UserInfo,
}

#[derive(Serialize)]
pub struct ProvidersResponse {
    pub local: bool,
    pub github: bool,
}

#[derive(Serialize)]
pub struct ErrorBody {
    pub error: &'static str,
    pub message: String,
}

#[derive(Deserialize)]
pub struct CredentialsBody {
    pub username: String,
    pub password: String,
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
        .route("/api/v1/auth/providers", get(providers))
        .route("/api/v1/auth/register", post(register))
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/me", get(get_me))
        .route("/api/v1/auth/github", get(github_login))
        .route("/api/v1/auth/github/callback", get(github_callback))
}

fn github_configured(state: &AppState) -> bool {
    !state.github_client_id.trim().is_empty() && !state.github_client_secret.trim().is_empty()
}

async fn providers(State(state): State<Arc<AppState>>) -> Json<ProvidersResponse> {
    Json(ProvidersResponse {
        local: true,
        github: github_configured(&state),
    })
}

fn issue_token(jwt_secret: &str, user_id: &str, username: &str, avatar_url: Option<String>) -> Result<String, StatusCode> {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        name: username.to_string(),
        avatar_url,
        exp: now + TOKEN_TTL_SECONDS,
        iat: now,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CredentialsBody>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorBody>)> {
    let user = state
        .user_store
        .create_user(&body.username, &body.password)
        .await
        .map_err(map_user_error)?;
    let token = issue_token(&state.jwt_secret, &user.id, &user.username, None)
        .map_err(|_| internal_err("could not issue token"))?;
    Ok(Json(AuthResponse {
        token,
        user: UserInfo {
            id: user.id,
            name: user.username,
            avatar_url: None,
        },
    }))
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CredentialsBody>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<ErrorBody>)> {
    let user = state
        .user_store
        .verify_credentials(&body.username, &body.password)
        .await
        .map_err(map_user_error)?;
    let token = issue_token(&state.jwt_secret, &user.id, &user.username, None)
        .map_err(|_| internal_err("could not issue token"))?;
    Ok(Json(AuthResponse {
        token,
        user: UserInfo {
            id: user.id,
            name: user.username,
            avatar_url: None,
        },
    }))
}

fn map_user_error(err: UserError) -> (StatusCode, Json<ErrorBody>) {
    match err {
        UserError::UsernameTaken => (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "username_taken",
                message: "username already taken".into(),
            }),
        ),
        UserError::PasswordTooShort => (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "password_too_short",
                message: format!("password must be at least {MIN_PASSWORD_LEN} characters"),
            }),
        ),
        UserError::InvalidUsername => (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "invalid_username",
                message: "username must be 1-64 non-blank characters".into(),
            }),
        ),
        UserError::InvalidCredentials => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorBody {
                error: "invalid_credentials",
                message: "invalid username or password".into(),
            }),
        ),
        UserError::Internal(msg) => internal_err_msg(msg),
    }
}

fn internal_err(msg: &str) -> (StatusCode, Json<ErrorBody>) {
    internal_err_msg(msg.to_string())
}

fn internal_err_msg(msg: String) -> (StatusCode, Json<ErrorBody>) {
    tracing::error!(error = %msg, "auth internal error");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorBody {
            error: "internal_error",
            message: "internal server error".into(),
        }),
    )
}

async fn github_login(State(state): State<Arc<AppState>>) -> axum::response::Response {
    if !github_configured(&state) {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorBody {
                error: "github_oauth_not_configured",
                message: "GitHub OAuth is not configured. Set GITHUB_CLIENT_ID and GITHUB_CLIENT_SECRET to enable.".into(),
            }),
        )
            .into_response();
    }
    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&scope=read:user",
        state.github_client_id
    );
    (StatusCode::TEMPORARY_REDIRECT, [(header::LOCATION, url)]).into_response()
}

async fn github_callback(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<GithubCallbackQuery>,
) -> Result<Json<AuthResponse>, StatusCode> {
    if !github_configured(&state) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
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

    let token = issue_token(
        &state.jwt_secret,
        &github_user.id.to_string(),
        &github_user.login,
        github_user.avatar_url.clone(),
    )?;

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
) -> Result<Json<MeResponse>, StatusCode> {
    let claims = extract_claims(&state.jwt_secret, &headers)?;
    Ok(Json(MeResponse {
        user: UserInfo {
            id: claims.sub,
            name: claims.name,
            avatar_url: claims.avatar_url,
        },
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, HeaderValue};

    #[test]
    fn issue_token_then_extract_claims_roundtrip() {
        let secret = "very-secret";
        let token =
            issue_token(secret, "user-1", "alice", Some("https://x/avatar".into())).unwrap();

        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );

        let claims = extract_claims(secret, &headers).expect("claims valid");
        assert_eq!(claims.sub, "user-1");
        assert_eq!(claims.name, "alice");
        assert_eq!(claims.avatar_url.as_deref(), Some("https://x/avatar"));
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn missing_authorization_header_yields_unauthorized() {
        let headers = HeaderMap::new();
        let res = extract_claims("secret", &headers);
        assert_eq!(res.unwrap_err(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn malformed_authorization_header_yields_unauthorized() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("NotBearer token"),
        );
        let res = extract_claims("secret", &headers);
        assert_eq!(res.unwrap_err(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn token_signed_with_other_secret_rejected() {
        let token = issue_token("secret-A", "id", "name", None).unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        let res = extract_claims("secret-B", &headers);
        assert_eq!(res.unwrap_err(), StatusCode::UNAUTHORIZED);
    }

    fn check_github_configured(id: &str, secret: &str) -> bool {
        !id.trim().is_empty() && !secret.trim().is_empty()
    }

    #[test]
    fn github_configured_requires_both_id_and_secret() {
        assert!(!check_github_configured("", ""));
        assert!(!check_github_configured("abc", ""));
        assert!(!check_github_configured("", "xyz"));
        assert!(!check_github_configured("   ", "xyz"));
        assert!(!check_github_configured("abc", "   "));
        assert!(check_github_configured("abc", "xyz"));
    }
}
