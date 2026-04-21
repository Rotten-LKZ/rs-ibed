use std::sync::Arc;

use axum::extract::{Multipart, State};
use axum::Json;
use chrono::Datelike;

use utoipa::ToSchema;

use crate::auth::AuthUser;
use crate::config::{MetadataField, PresetConfig};
use crate::error::{AppError, AppResult};
use crate::exif;
use crate::file_type::detect_image_type;
use crate::image_proc::{decode_image, format_to_ext, process_image};
use crate::models::image::{NewImage, UploadResponse};
use crate::state::AppState;
use crate::storage;

#[allow(dead_code)]
#[derive(ToSchema)]
pub struct UploadRequest {
    /// Image file to upload
    #[schema(value_type = String, format = Binary)]
    pub file: Vec<u8>,
    /// Comma-separated list of metadata fields to keep (e.g. "camera,time").
    /// Overrides the server-side `keep_metadata_fields` config for this upload.
    #[schema(nullable)]
    pub keep_metadata_fields: Option<String>,
}

/// POST /api/upload
#[utoipa::path(
    post,
    path = "/api/upload",
    request_body(content_type = "multipart/form-data", content = UploadRequest),
    responses(
        (status = 200, description = "Upload successful", body = UploadResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
    ),
    security(("cookieAuth" = []), ("bearerAuth" = []))
)]
pub async fn upload(
    _auth: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<UploadResponse>> {
    // 1. Extract file field and optional keep_metadata_fields override
    let mut file_data: Option<(String, axum::body::Bytes)> = None;
    let mut keep_metadata_override: Option<Vec<MetadataField>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        match field.name() {
            Some("file") => {
                let name = field.file_name().unwrap_or("unknown").to_string();
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
                file_data = Some((name, data));
            }
            Some("keep_metadata_fields") => {
                let text = field
                    .text()
                    .await
                    .map_err(|e| AppError::BadRequest(e.to_string()))?;
                let fields = parse_metadata_fields(&text)?;
                keep_metadata_override = Some(fields);
            }
            _ => continue,
        }
    }

    let (original_name, data) =
        file_data.ok_or_else(|| AppError::BadRequest("missing 'file' field".into()))?;

    if data.is_empty() {
        return Err(AppError::BadRequest("empty file".into()));
    }

    // 2. Detect real file type from bytes
    let detected = detect_image_type(&data)?;

    // 3. Handle EXIF metadata based on config or per-request override.
    let processed_data = if let Some(ref keep_fields) = keep_metadata_override {
        // Request-level override: always strip according to these fields
        exif::strip_metadata(&data, &detected.extension, keep_fields)
    } else if state.config.image.enable {
        // No override + processing enabled: store original as-is
        data.to_vec()
    } else {
        // No override + processing disabled: use config's keep list
        exif::strip_metadata(
            &data,
            &detected.extension,
            &state.config.image.keep_metadata_fields,
        )
    };

    // 4. Compute BLAKE3 hash on the stored bytes
    let hash = blake3::hash(&processed_data).to_hex().to_string();

    // 5. Dedup check
    if let Some(existing) = state.repo.find_by_hash(&hash).await? {
        if existing.is_deleted {
            state.repo.restore(existing.id).await?;
        }
        let created = existing.created_at;
        let url = state.config.server.url_pattern.view_path(
            created.year() as u16,
            created.month() as u8,
            created.day() as u8,
            &existing.hash,
            &existing.extension,
        );
        return Ok(Json(UploadResponse {
            id: existing.id,
            hash: existing.hash,
            url,
            file_name: existing.file_name,
            mime_type: existing.mime_type,
            size: existing.size,
            width: existing.width,
            height: existing.height,
        }));
    }

    // 6. Decode image to get dimensions
    let decoded = decode_image(&processed_data, &detected.extension)?;
    let width = decoded.width() as i32;
    let height = decoded.height() as i32;

    // 7. Save cleaned file to disk: base_dir/hash[0:2]/hash[2:4]/hash.ext
    let file_path = storage::original_path(
        &state.config.storage.base_dir,
        &hash,
        &detected.extension,
    );

    if let Some(parent) = file_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&file_path, &processed_data).await?;

    // 8. Insert into DB
    let new_img = NewImage {
        hash: hash.clone(),
        display_name: strip_extension(&original_name),
        file_name: original_name.clone(),
        extension: detected.extension.clone(),
        mime_type: detected.mime_type.clone(),
        size: processed_data.len() as i64,
        width,
        height,
        user_id: None,
    };

    let record = state.repo.insert_image(&new_img).await?;

    // 9. Build URL
    let created = record.created_at;
    let url = state.config.server.url_pattern.view_path(
        created.year() as u16,
        created.month() as u8,
        created.day() as u8,
        &record.hash,
        &record.extension,
    );

    // 10. Spawn background tasks for eager presets (non-blocking).
    // Only if image processing is enabled.
    if state.config.image.enable {
        spawn_eager_presets(state.clone(), hash.clone(), detected.extension.clone(), processed_data);
    }

    Ok(Json(UploadResponse {
        id: record.id,
        hash: record.hash,
        url,
        file_name: record.file_name,
        mime_type: record.mime_type,
        size: record.size,
        width: record.width,
        height: record.height,
    }))
}

/// Spawn background tokio tasks for each preset with `eager = true`.
/// These run after the upload response is already sent.
fn spawn_eager_presets(state: AppState, hash: String, ext: String, data: Vec<u8>) {
    let image_cfg = &state.config.image;

    // Collect eager presets
    let eager_presets: Vec<(String, PresetConfig)> = image_cfg
        .presets
        .iter()
        .filter(|(_, p)| p.eager)
        .map(|(name, p)| (name.clone(), p.clone()))
        .collect();

    if eager_presets.is_empty() {
        return;
    }

    let config = Arc::clone(&state.config);
    let semaphore = Arc::clone(&state.workers);
    let data = Arc::new(data);

    for (preset_name, preset) in eager_presets {
        let config = Arc::clone(&config);
        let semaphore = Arc::clone(&semaphore);
        let data = Arc::clone(&data);
        let hash = hash.clone();
        let ext = ext.clone();

        tokio::spawn(async move {
            if let Err(e) =
                generate_preset_cache(&config, &semaphore, &data, &hash, &ext, &preset_name, &preset)
                    .await
            {
                tracing::warn!(
                    preset = %preset_name,
                    hash = %hash,
                    "eager preset generation failed: {e}"
                );
            } else {
                tracing::debug!(preset = %preset_name, hash = %hash, "eager preset generated");
            }
        });
    }
}

async fn generate_preset_cache(
    config: &crate::config::AppConfig,
    semaphore: &tokio::sync::Semaphore,
    data: &[u8],
    hash: &str,
    ext: &str,
    preset_name: &str,
    preset: &PresetConfig,
) -> AppResult<()> {
    let image_cfg = &config.image;
    let effective_format = preset.effective_format(image_cfg.default_format);
    let output_ext = format_to_ext(effective_format);

    let variant_key = storage::preset_variant_key(preset_name);
    let cache_path = storage::cache_path(&config.storage.cache_dir, &variant_key, hash, output_ext);

    // Skip if already cached
    if tokio::fs::metadata(&cache_path).await.is_ok() {
        return Ok(());
    }

    let (processed, _mime) = process_image(data, ext, preset, image_cfg, semaphore).await?;

    if let Some(parent) = cache_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&cache_path, &processed).await?;

    Ok(())
}

fn parse_metadata_fields(input: &str) -> AppResult<Vec<MetadataField>> {
    input
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<MetadataField>()
                .map_err(|e| AppError::BadRequest(e))
        })
        .collect()
}

fn strip_extension(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_string(),
        _ => name.to_string(),
    }
}
