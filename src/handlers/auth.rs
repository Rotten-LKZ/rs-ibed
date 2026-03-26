use axum::extract::{Query, State};
use axum::http::header::SET_COOKIE;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::auth::{self, AuthUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub token: String,
}

#[derive(Deserialize, ToSchema)]
pub struct CliLoginQuery {
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthSuccessResponse {
    pub ok: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthCheckResponse {
    pub ok: bool,
    pub user: String,
}

/// POST /api/auth/login
/// Body: { "token": "<AUTH_TOKEN from .env>" }
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful, JWT cookie set", body = AuthSuccessResponse),
        (status = 401, description = "Invalid token"),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> AppResult<impl IntoResponse> {
    if body.token != state.secrets.auth_token {
        return Err(AppError::Unauthorized);
    }

    let jwt = auth::create_jwt(&state.secrets.jwt_secret)?;
    let cookie = auth::jwt_cookie_header(&jwt);

    Ok(Response::builder()
        .header(SET_COOKIE, cookie)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&AuthSuccessResponse { ok: true })
                .map_err(|e| AppError::Internal(e.to_string()))?,
        ))
        .map_err(|e| AppError::Internal(e.to_string()))?)
}

/// GET /api/auth/cli?token=<one-time-token>
/// Validates the one-time CLI token and sets JWT cookie.
#[utoipa::path(
    get,
    path = "/api/auth/cli",
    params(("token" = String, Query, description = "One-time CLI token")),
    responses(
        (status = 200, description = "Login successful, JWT cookie set", body = AuthSuccessResponse),
        (status = 401, description = "Invalid or expired token"),
    )
)]
pub async fn cli_login(
    State(state): State<AppState>,
    Query(query): Query<CliLoginQuery>,
) -> AppResult<impl IntoResponse> {
    if !state.cli_tokens.validate_and_consume(&query.token) {
        return Err(AppError::Unauthorized);
    }

    let jwt = auth::create_jwt(&state.secrets.jwt_secret)?;
    let cookie = auth::jwt_cookie_header(&jwt);

    Ok(Response::builder()
        .header(SET_COOKIE, cookie)
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(
            serde_json::to_vec(&AuthSuccessResponse { ok: true })
                .map_err(|e| AppError::Internal(e.to_string()))?,
        ))
        .map_err(|e| AppError::Internal(e.to_string()))?)
}

/// GET /api/auth/check
/// Returns 200 if the user is authenticated.
#[utoipa::path(
    get,
    path = "/api/auth/check",
    responses(
        (status = 200, description = "Authenticated", body = AuthCheckResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("cookieAuth" = []), ("bearerAuth" = []))
)]
pub async fn check(_auth: AuthUser) -> Json<AuthCheckResponse> {
    Json(AuthCheckResponse {
        ok: true,
        user: "admin".to_string(),
    })
}
