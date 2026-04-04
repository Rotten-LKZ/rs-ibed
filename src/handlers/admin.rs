use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::Datelike;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::image::{
    ImageCountResponse, ImageDetailResponse, ImageListItem, ImageListQuery, ImageListResponse,
    OkResponse, RenameRequest,
};
use crate::models::storage_endpoint::{StorageEndpointResponse, UpdateEndpointRequest};
use crate::state::AppState;
use crate::storage;

/// Build full URL for the image view path.
/// If `public_url` is configured, returns full URL; otherwise returns relative path.
fn build_full_url(config: &crate::config::AppConfig, year: u16, month: u8, day: u8, hash: &str, ext: &str) -> String {
    let path = config.server.url_pattern.view_path(year, month, day, hash, ext);
    match &config.server.public_url {
        Some(base) => format!("{}{}", base.trim_end_matches('/'), path),
        None => path,
    }
}

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

    let mut items: Vec<ImageListItem> = items
        .into_iter()
        .map(|item| {
            let created = item.created_at;
            let view_url = build_full_url(
                &state.config,
                created.year() as u16,
                created.month() as u8,
                created.day() as u8,
                &item.hash,
                &item.extension,
            );

            ImageListItem {
                id: item.id,
                hash: item.hash.clone(),
                display_name: item.display_name,
                file_name: item.file_name,
                extension: item.extension.clone(),
                mime_type: item.mime_type,
                size: item.size,
                width: item.width,
                height: item.height,
                user_id: item.user_id,
                is_deleted: item.is_deleted,
                created_at: item.created_at,
                updated_at: item.updated_at,
                view_url,
                direct_url: None,
                storage_available: None,
            }
        })
        .collect();

    // Fill in direct_urls (async, one per item)
    for item in &mut items {
        item.direct_url = state
            .storage_manager
            .direct_url_for(&item.hash, &item.extension, item.size)
            .await;
        // Note: storage_available is intentionally left as None in list view
        // to avoid expensive HEAD checks on every list item.
        // Use the detail endpoint to check storage availability for a specific image.
    }

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
    let view_url = build_full_url(
        &state.config,
        created.year() as u16,
        created.month() as u8,
        created.day() as u8,
        &record.hash,
        &record.extension,
    );

    let direct_url = state
        .storage_manager
        .direct_url_for(&record.hash, &record.extension, record.size)
        .await;

    let storage_available = state
        .storage_manager
        .check_file_available(&record.hash, &record.extension)
        .await;

    Ok(Json(ImageDetailResponse {
        image: record,
        view_url,
        direct_url,
        storage_available: Some(storage_available),
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

    let origin_key = storage::object_key_original(&record.hash, &record.extension);
    for ep in state.storage_manager.list_active().await {
        if let Ok(Some(meta)) = ep.backend.head_object(&origin_key).await {
            if ep.backend.delete_object(&origin_key).await.is_ok() {
                let _ = state.storage_manager.adjust_used_size(&ep.name, -meta.size).await;
            }
        }
    }

    let _ = storage::remove_all_cache_for_hash(state.config.storage.cache_dir.as_deref().unwrap_or(""), &record.hash).await;

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
        let origin_key = storage::object_key_original(&img.hash, &img.extension);
        for ep in state.storage_manager.list_active().await {
            if let Ok(Some(meta)) = ep.backend.head_object(&origin_key).await {
                if ep.backend.delete_object(&origin_key).await.is_ok() {
                    let _ = state.storage_manager.adjust_used_size(&ep.name, -meta.size).await;
                }
            }
        }
        let _ = storage::remove_all_cache_for_hash(state.config.storage.cache_dir.as_deref().unwrap_or(""), &img.hash).await;
    }

    state.repo.delete_all_deleted().await?;

    Ok(Json(OkResponse { ok: true }))
}

/// GET /api/admin/storage/endpoints
#[utoipa::path(
    get,
    path = "/api/admin/storage/endpoints",
    responses(
        (status = 200, description = "List of storage endpoints", body = Vec<StorageEndpointResponse>),
        (status = 401, description = "Unauthorized"),
    ),
    security(("cookieAuth" = []), ("bearerAuth" = []))
)]
pub async fn list_storage_endpoints(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<StorageEndpointResponse>>> {
    let endpoints = state.storage_manager.list_all().await;
    let config_eps = &state.config.storage.endpoints;

    let items = endpoints
        .into_iter()
        .map(|ep| {
            let cfg = config_eps.iter().find(|c| c.name == ep.name);
            StorageEndpointResponse {
                name: ep.name.clone(),
                description: cfg.map(|c| c.description.clone()).unwrap_or_default(),
                endpoint_type: ep.endpoint_type.clone(),
                capacity_bytes: ep.capacity_bytes,
                used_size: ep.used_size.load(std::sync::atomic::Ordering::Relaxed),
                priority: ep.priority,
                status: ep.status.clone(),
                direct_mode: ep.direct_mode.clone(),
            }
        })
        .collect();

    Ok(Json(items))
}

/// POST /api/admin/storage/endpoints/{name}/update
#[utoipa::path(
    post,
    path = "/api/admin/storage/endpoints/{name}/update",
    params(("name" = String, Path, description = "Endpoint name")),
    request_body = UpdateEndpointRequest,
    responses(
        (status = 200, description = "Updated", body = OkResponse),
        (status = 400, description = "Cannot change priority or capacity via API"),
        (status = 404, description = "Not found"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("cookieAuth" = []), ("bearerAuth" = []))
)]
pub async fn update_storage_endpoint(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<UpdateEndpointRequest>,
) -> AppResult<Json<OkResponse>> {
    state
        .storage_manager
        .get_endpoint(&name)
        .await
        .ok_or(AppError::NotFound)?;

    state
        .storage_manager
        .update_endpoint_fields(
            &name,
            body.description.as_deref(),
            body.status.as_deref(),
        )
        .await?;

    // If status changed, rebuild the router so disabled endpoints are excluded
    if body.status.is_some() {
        state.storage_manager.rebuild_router().await;
    }

    Ok(Json(OkResponse { ok: true }))
}
