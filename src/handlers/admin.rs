use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::Datelike;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::image::{
    ImageDetailResponse, ImageListQuery, ImageListResponse, OkResponse, RenameRequest,
};
use crate::state::AppState;

/// GET /api/admin/images
#[utoipa::path(
    get,
    path = "/api/admin/images",
    params(ImageListQuery),
    responses(
        (status = 200, description = "Paginated image list", body = ImageListResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("cookieAuth" = []), ("bearerAuth" = []))
)]
pub async fn list_images(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ImageListQuery>,
) -> AppResult<Json<ImageListResponse>> {
    let (items, total) = state
        .repo
        .list_images(
            query.page,
            query.per_page,
            query.name.as_deref(),
            query.date_from.as_deref(),
            query.date_to.as_deref(),
        )
        .await?;

    Ok(Json(ImageListResponse {
        items,
        total,
        page: query.page,
        per_page: query.per_page,
    }))
}

/// GET /api/admin/images/:id
#[utoipa::path(
    get,
    path = "/api/admin/images/{id}",
    params(("id" = i64, Path, description = "Image ID")),
    responses(
        (status = 200, description = "Image detail", body = ImageDetailResponse),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("cookieAuth" = []), ("bearerAuth" = []))
)]
pub async fn get_image(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ImageDetailResponse>> {
    let record = state
        .repo
        .find_by_id(id)
        .await?
        .ok_or(AppError::NotFound)?;

    let created = record.created_at;
    let view_url = state.config.server.url_pattern.view_path(
        created.year() as u16,
        created.month() as u8,
        created.day() as u8,
        &record.hash,
        &record.extension,
    );

    Ok(Json(ImageDetailResponse {
        image: record,
        view_url,
    }))
}

/// POST /api/admin/images/:id/rename
#[utoipa::path(
    post,
    path = "/api/admin/images/{id}/rename",
    params(("id" = i64, Path, description = "Image ID")),
    request_body = RenameRequest,
    responses(
        (status = 200, description = "Renamed", body = OkResponse),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("cookieAuth" = []), ("bearerAuth" = []))
)]
pub async fn rename_image(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<RenameRequest>,
) -> AppResult<Json<OkResponse>> {
    // Check exists
    state
        .repo
        .find_by_id(id)
        .await?
        .ok_or(AppError::NotFound)?;

    state.repo.rename(id, &body.display_name).await?;

    Ok(Json(OkResponse { ok: true }))
}

/// POST /api/admin/images/:id/delete
#[utoipa::path(
    post,
    path = "/api/admin/images/{id}/delete",
    params(("id" = i64, Path, description = "Image ID")),
    responses(
        (status = 200, description = "Soft deleted", body = OkResponse),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("cookieAuth" = []), ("bearerAuth" = []))
)]
pub async fn delete_image(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<OkResponse>> {
    state
        .repo
        .find_by_id(id)
        .await?
        .ok_or(AppError::NotFound)?;

    state.repo.soft_delete(id).await?;

    Ok(Json(OkResponse { ok: true }))
}

/// POST /api/admin/images/:id/restore
#[utoipa::path(
    post,
    path = "/api/admin/images/{id}/restore",
    params(("id" = i64, Path, description = "Image ID")),
    responses(
        (status = 200, description = "Restored", body = OkResponse),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("cookieAuth" = []), ("bearerAuth" = []))
)]
pub async fn restore_image(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<OkResponse>> {
    state
        .repo
        .find_by_id(id)
        .await?
        .ok_or(AppError::NotFound)?;

    state.repo.restore(id).await?;

    Ok(Json(OkResponse { ok: true }))
}
