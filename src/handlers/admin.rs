use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::Datelike;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult};
use crate::models::image::{
    ImageCountResponse, ImageDetailResponse, ImageListItem, ImageListQuery, ImageListResponse,
    OkResponse, RenameRequest, TrashEmptyItem, TrashEmptyResponse,
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
        (status = 503, description = "Storage unavailable - deletion failed"),
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

    // Check if the primary node (based on hash ring) is disabled
    // If so, we cannot confirm whether the file exists on that node → block deletion
    {
        let router = state.storage_manager.router_read();
        if let Some(primary) = router.primary_for(&record.hash) {
            if primary.status != "active" {
                tracing::warn!(
                    endpoint = %primary.name,
                    status = %primary.status,
                    hash = %record.hash,
                    "Primary storage node is disabled - cannot safely delete"
                );
                return Err(AppError::StorageUnavailable);
            }
        }
    }

    // Only check ACTIVE endpoints - disabled endpoints are treated as offline
    let mut deletions_required = Vec::new();
    for ep in state.storage_manager.list_active().await {
        match ep.backend.head_object(&origin_key).await {
            Ok(Some(meta)) => {
                deletions_required.push((ep.name.clone(), meta.size));
            }
            Ok(None) => {
                // File not on this endpoint, no action needed
            }
            Err(e) => {
                // Active endpoint is unreachable - cannot safely delete
                tracing::error!(
                    endpoint = %ep.name,
                    hash = %record.hash,
                    error = %e,
                    "Active storage endpoint unreachable - cannot verify file existence"
                );
                return Err(AppError::StorageUnavailable);
            }
        }
    }

    // Attempt to delete from all active endpoints that have the file
    let mut failed_endpoints = Vec::new();
    for (ep_name, size) in deletions_required {
        let ep = state.storage_manager.get_endpoint(&ep_name).await
            .ok_or_else(|| AppError::Internal(format!("Endpoint {} disappeared", ep_name)))?;

        match ep.backend.delete_object(&origin_key).await {
            Ok(_) => {
                // Update used_size tracking
                let _ = state.storage_manager.adjust_used_size(&ep_name, -size).await;
            }
            Err(e) => {
                tracing::error!(
                    endpoint = %ep_name,
                    hash = %record.hash,
                    error = %e,
                    "Failed to delete object from storage"
                );
                failed_endpoints.push(ep_name);
            }
        }
    }

    // If any deletions failed, DO NOT delete the database record
    if !failed_endpoints.is_empty() {
        return Err(AppError::StorageUnavailable);
    }

    // All storage deletions successful - now delete from database
    let _ = storage::remove_all_cache_for_hash(
        state.config.storage.cache_dir.as_deref().unwrap_or(""),
        &record.hash,
    ).await;

    state.repo.hard_delete(id).await?;

    Ok(Json(OkResponse { ok: true }))
}

/// POST /api/admin/images/trash/empty
#[utoipa::path(
    post,
    path = "/api/admin/images/trash/empty",
    responses(
        (status = 200, description = "Trash emptied (possibly partial)", body = TrashEmptyResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(("cookieAuth" = []), ("bearerAuth" = []))
)]
pub async fn empty_trash(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> AppResult<Json<TrashEmptyResponse>> {
    let images = state.repo.find_all_deleted().await?;

    let mut results: Vec<TrashEmptyItem> = Vec::new();

    for img in &images {
        let origin_key = storage::object_key_original(&img.hash, &img.extension);

        // Check if the primary node (based on hash ring) is disabled
        // If so, we cannot confirm whether the file exists on that node → skip this image
        {
            let router = state.storage_manager.router_read();
            if let Some(primary) = router.primary_for(&img.hash) {
                if primary.status != "active" {
                    tracing::warn!(
                        endpoint = %primary.name,
                        status = %primary.status,
                        hash = %img.hash,
                        "Primary storage node is disabled - cannot safely delete"
                    );
                    results.push(TrashEmptyItem {
                        image_id: img.id,
                        hash: img.hash.clone(),
                        success: false,
                        error: Some("Primary storage node is disabled - cannot safely delete".to_string()),
                    });
                    continue;
                }
            }
        }

        // Only check ACTIVE endpoints - disabled endpoints are treated as offline
        let mut deletions_required = Vec::new();
        let mut check_failed = false;

        for ep in state.storage_manager.list_active().await {
            match ep.backend.head_object(&origin_key).await {
                Ok(Some(meta)) => {
                    deletions_required.push((ep.name.clone(), meta.size));
                }
                Ok(None) => {}
                Err(e) => {
                    // Active endpoint is unreachable - cannot safely delete
                    tracing::error!(
                        endpoint = %ep.name,
                        hash = %img.hash,
                        error = %e,
                        "Active storage endpoint unreachable - cannot verify file existence"
                    );
                    check_failed = true;
                    break;
                }
            }
        }

        if check_failed {
            results.push(TrashEmptyItem {
                image_id: img.id,
                hash: img.hash.clone(),
                success: false,
                error: Some("Active storage endpoint unavailable - cannot delete safely".to_string()),
            });
            continue;
        }

        // Attempt to delete from all active endpoints
        let mut failed_endpoints = Vec::new();
        for (ep_name, size) in deletions_required {
            if let Some(ep) = state.storage_manager.get_endpoint(&ep_name).await {
                match ep.backend.delete_object(&origin_key).await {
                    Ok(_) => {
                        let _ = state.storage_manager.adjust_used_size(&ep_name, -size).await;
                    }
                    Err(e) => {
                        tracing::error!(
                            endpoint = %ep_name,
                            hash = %img.hash,
                            error = %e,
                            "Failed to delete object from storage"
                        );
                        failed_endpoints.push(ep_name);
                    }
                }
            }
        }

        if !failed_endpoints.is_empty() {
            results.push(TrashEmptyItem {
                image_id: img.id,
                hash: img.hash.clone(),
                success: false,
                error: Some(format!("Failed to delete from endpoints: {:?}", failed_endpoints)),
            });
            continue;
        }

        // All storage deletions successful - delete from database and cache
        let _ = storage::remove_all_cache_for_hash(
            state.config.storage.cache_dir.as_deref().unwrap_or(""),
            &img.hash,
        ).await;

        if let Err(e) = state.repo.hard_delete(img.id).await {
            results.push(TrashEmptyItem {
                image_id: img.id,
                hash: img.hash.clone(),
                success: false,
                error: Some(format!("Database error: {}", e)),
            });
        } else {
            results.push(TrashEmptyItem {
                image_id: img.id,
                hash: img.hash.clone(),
                success: true,
                error: None,
            });
        }
    }

    let succeeded = results.iter().filter(|r| r.success).count();
    let failed = results.len() - succeeded;

    Ok(Json(TrashEmptyResponse {
        ok: failed == 0,
        total: results.len(),
        succeeded,
        failed,
        results,
    }))
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
