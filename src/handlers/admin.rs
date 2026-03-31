use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::Datelike;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::image::{
    ImageCountResponse, ImageDetailResponse, ImageListItem, ImageListQuery, ImageListResponse,
    OkResponse, RenameRequest,
};
use crate::state::AppState;
use crate::storage;

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
            query.deleted,
        )
        .await?;

    let items = items
        .into_iter()
        .map(|item| {
            let created = item.created_at;
            let view_url = state.config.server.url_pattern.view_path(
                created.year() as u16,
                created.month() as u8,
                created.day() as u8,
                &item.hash,
                &item.extension,
            );

            ImageListItem {
                id: item.id,
                hash: item.hash,
                display_name: item.display_name,
                file_name: item.file_name,
                extension: item.extension,
                mime_type: item.mime_type,
                size: item.size,
                width: item.width,
                height: item.height,
                user_id: item.user_id,
                is_deleted: item.is_deleted,
                created_at: item.created_at,
                updated_at: item.updated_at,
                view_url,
            }
        })
        .collect();

    Ok(Json(ImageListResponse {
        items,
        total,
        page: query.page,
        per_page: query.per_page,
    }))
}

/// GET /api/admin/images/count
#[utoipa::path(
    get,
    path = "/api/admin/images/count",
    responses(
        (status = 200, description = "Image counts grouped by deletion status", body = ImageCountResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("cookieAuth" = []), ("bearerAuth" = []))
)]
pub async fn count_images(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<ImageCountResponse>> {
    let counts = state.repo.count_images().await?;
    Ok(Json(counts))
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
    let record = state.repo.find_by_id(id).await?.ok_or(AppError::NotFound)?;

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
    state.repo.restore(id).await?;

    Ok(Json(OkResponse { ok: true }))
}

/// POST /api/admin/images/{id}/permanent-delete
#[utoipa::path(
    post,
    path = "/api/admin/images/{id}/permanent-delete",
    params(("id" = i64, Path, description = "Image ID")),
    responses(
        (status = 200, description = "Permanently deleted", body = OkResponse),
        (status = 400, description = "Image not in trash"),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("cookieAuth" = []), ("bearerAuth" = []))
)]
pub async fn permanent_delete_image(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<OkResponse>> {
    let record = state.repo.find_by_id(id).await?.ok_or(AppError::NotFound)?;

    if !record.is_deleted {
        return Err(AppError::BadRequest(
            "image is not in trash; soft-delete it first".into(),
        ));
    }

    let src = storage::original_path(
        &state.config.storage.base_dir,
        &record.hash,
        &record.extension,
    );
    let _ = tokio::fs::remove_file(&src).await;

    let _ = storage::remove_all_cache_for_hash(&state.config.storage.cache_dir, &record.hash).await;

    state.repo.hard_delete(id).await?;

    Ok(Json(OkResponse { ok: true }))
}

/// POST /api/admin/images/trash/empty
#[utoipa::path(
    post,
    path = "/api/admin/images/trash/empty",
    responses(
        (status = 200, description = "Trash emptied", body = OkResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("cookieAuth" = []), ("bearerAuth" = []))
)]
pub async fn empty_trash(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<OkResponse>> {
    let images = state.repo.find_all_deleted().await?;

    for img in &images {
        let src = storage::original_path(
            &state.config.storage.base_dir,
            &img.hash,
            &img.extension,
        );
        let _ = tokio::fs::remove_file(&src).await;
        let _ = storage::remove_all_cache_for_hash(&state.config.storage.cache_dir, &img.hash).await;
    }

    state.repo.delete_all_deleted().await?;

    Ok(Json(OkResponse { ok: true }))
}
