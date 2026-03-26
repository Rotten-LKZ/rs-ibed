use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::config::{FitMode, ImageFormat, ImageConfig, PresetConfig};
use crate::error::{AppError, AppResult};
use crate::image_proc::{format_to_ext, format_to_mime, process_image};
use crate::path_parser::{DynamicParams, PathVariant, parse_v_path};
use crate::state::AppState;
use crate::storage;

#[derive(Debug, Clone)]
struct ResolvedVariant {
    variant_key: String,
    preset: PresetConfig,
    disk_ttl: u64,
}

/// GET /v/{*path}
#[utoipa::path(
    get,
    path = "/v/{path}",
    params(("path" = String, Path, description = "Image path")),
    responses(
        (status = 200, description = "Processed image"),
        (status = 304, description = "Not modified"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn view(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<String>,
) -> AppResult<impl IntoResponse> {
    let cfg = &state.config;
    let pattern = cfg.server.url_pattern;
    let image_cfg = &cfg.image;

    let preset_keys: Vec<&str> = image_cfg.presets.keys().map(|s| s.as_str()).collect();
    let parsed = parse_v_path(pattern, &path, &preset_keys).ok_or(AppError::NotFound)?;

    let record = state
        .repo
        .find_by_hash(&parsed.hash)
        .await?
        .ok_or(AppError::NotFound)?;

    let negotiate = cfg.server.enable_negotiated_cache;

    // If image processing is disabled, only allow direct access (no preset)
    // or the "default" preset. Any other preset or dynamic parameters
    // will result in 404.
    if !image_cfg.enable {
        let is_allowed = match &parsed.variant {
            PathVariant::None => true,
            PathVariant::Preset(name) if name == "default" => true,
            _ => false,
        };

        if !is_allowed {
            return Err(AppError::NotFound);
        }

        let etag = if negotiate {
            let e = format!("\"{}\"", record.hash);
            if etag_matches(&headers, &e) {
                return Ok(not_modified(&e, cfg.server.cache_max_age).into_response());
            }
            Some(e)
        } else {
            None
        };

        let orig_path =
            storage::original_path(&cfg.storage.base_dir, &record.hash, &record.extension);
        let raw = tokio::fs::read(&orig_path).await?;

        return Ok(build_response(
            raw,
            &record.mime_type,
            negotiate,
            cfg.server.cache_max_age,
            image_cfg.cache_ttl,
            etag.as_deref(),
        )
        .into_response());
    }

    let resolved = resolve_variant(&parsed.variant, image_cfg, &record)?;

    let effective_format = resolved.preset.effective_format(image_cfg.default_format);
    let effective_quality = resolved.preset.effective_quality(image_cfg.quality);
    let effective_fit = resolved.preset.effective_fit();
    let output_ext = if effective_format == ImageFormat::Original {
        record.extension.as_str()
    } else {
        format_to_ext(effective_format)
    };

    let negotiate = cfg.server.enable_negotiated_cache;
    let etag = if negotiate {
        Some(compute_view_etag(
            &record.hash,
            resolved.preset.width,
            resolved.preset.height,
            effective_fit,
            effective_format,
            effective_quality,
        ))
    } else {
        None
    };

    if let Some(ref etag) = etag {
        if etag_matches(&headers, etag) {
            return Ok(not_modified(etag, cfg.server.cache_max_age));
        }
    }

    let cache_path = storage::cache_path(
        &cfg.storage.cache_dir,
        &resolved.variant_key,
        &record.hash,
        output_ext,
    );

    if let Ok(meta) = tokio::fs::metadata(&cache_path).await {
        let is_fresh = if resolved.disk_ttl == 0 {
            true
        } else {
            meta.modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|mtime| {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default();
                    now.as_secs().saturating_sub(mtime.as_secs()) < resolved.disk_ttl
                })
                .unwrap_or(false)
        };

        if is_fresh {
            let bytes = tokio::fs::read(&cache_path).await?;
            let mime = if effective_format == ImageFormat::Original {
                record.mime_type.as_str()
            } else {
                format_to_mime(effective_format)
            };
            return Ok(build_response(
                bytes,
                mime,
                negotiate,
                cfg.server.cache_max_age,
                resolved.disk_ttl,
                etag.as_deref(),
            ));
        }
    }

    let orig_path = storage::original_path(&cfg.storage.base_dir, &record.hash, &record.extension);
    let raw = tokio::fs::read(&orig_path).await?;

    let (processed, mime) = if effective_format == ImageFormat::Original {
        (raw, record.mime_type.clone())
    } else {
        process_image(&raw, &resolved.preset, image_cfg, &state.workers).await?
    };

    if let Some(parent) = cache_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&cache_path, &processed).await?;

    Ok(build_response(
        processed,
        &mime,
        negotiate,
        cfg.server.cache_max_age,
        resolved.disk_ttl,
        etag.as_deref(),
    ))
}

/// GET /d/{*path}
#[utoipa::path(
    get,
    path = "/d/{path}",
    params(("path" = String, Path, description = "Image path")),
    responses(
        (status = 200, description = "Original image download"),
        (status = 304, description = "Not modified"),
        (status = 404, description = "Not found"),
    )
)]
pub async fn download(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(path): Path<String>,
) -> AppResult<impl IntoResponse> {
    let cfg = &state.config;
    let pattern = cfg.server.url_pattern;
    let image_cfg = &cfg.image;

    if !image_cfg.allow_show_origin {
        crate::auth::verify_auth(&headers, &state.secrets)?;
    }

    let preset_keys: Vec<&str> = image_cfg.presets.keys().map(|s| s.as_str()).collect();
    let parsed = parse_v_path(pattern, &path, &preset_keys).ok_or(AppError::NotFound)?;

    let record = state
        .repo
        .find_by_hash(&parsed.hash)
        .await?
        .ok_or(AppError::NotFound)?;

    let negotiate = cfg.server.enable_negotiated_cache;
    let etag = if negotiate {
        let e = format!("\"{}\"", record.hash);
        if etag_matches(&headers, &e) {
            return Ok(not_modified(&e, cfg.server.cache_max_age));
        }
        Some(e)
    } else {
        None
    };

    let orig_path = storage::original_path(&cfg.storage.base_dir, &record.hash, &record.extension);
    let raw = tokio::fs::read(&orig_path).await?;

    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, record.mime_type.as_str())
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", record.file_name),
        );

    if let Some(ref etag) = etag {
        builder = builder
            .header(header::ETAG, etag.as_str())
            .header(
                header::CACHE_CONTROL,
                cache_control_value(true, cfg.server.cache_max_age, 0),
            );
    }

    let response = builder
        .body(Body::from(raw))
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(response)
}

fn resolve_variant(
    path_variant: &PathVariant,
    image_cfg: &ImageConfig,
    record: &crate::models::image::ImageModel,
) -> AppResult<ResolvedVariant> {
    match path_variant {
        PathVariant::Preset(name) => {
            let preset = image_cfg.presets.get(name).ok_or(AppError::NotFound)?.clone();
            Ok(ResolvedVariant {
                variant_key: storage::preset_variant_key(name),
                disk_ttl: preset.effective_ttl(image_cfg.cache_ttl),
                preset,
            })
        }
        PathVariant::Dynamic(dynamic) => {
            if let Some((preset_name, preset)) = find_matching_preset(image_cfg, *dynamic) {
                return Ok(ResolvedVariant {
                    variant_key: storage::preset_variant_key(&preset_name),
                    disk_ttl: preset.effective_ttl(image_cfg.cache_ttl),
                    preset,
                });
            }

            if !image_cfg.dynamic.allow {
                return Err(AppError::Forbidden);
            }

            validate_dynamic_request(dynamic, image_cfg, record)?;

            let preset = PresetConfig {
                width: dynamic.width,
                height: dynamic.height,
                fit: dynamic.fit,
                format: Some(image_cfg.default_format),
                quality: Some(image_cfg.quality),
                eager: false,
                cache_ttl: Some(image_cfg.cache_ttl),
            };

            Ok(ResolvedVariant {
                variant_key: storage::dynamic_variant_key(*dynamic),
                disk_ttl: image_cfg.cache_ttl,
                preset,
            })
        }
        PathVariant::None => {
            let preset = PresetConfig {
                width: None,
                height: None,
                fit: None,
                format: Some(image_cfg.default_format),
                quality: Some(image_cfg.quality),
                eager: false,
                cache_ttl: Some(image_cfg.cache_ttl),
            };

            Ok(ResolvedVariant {
                variant_key: storage::preset_variant_key("default"),
                disk_ttl: image_cfg.cache_ttl,
                preset,
            })
        }
    }
}

fn find_matching_preset(
    image_cfg: &ImageConfig,
    dynamic: DynamicParams,
) -> Option<(String, PresetConfig)> {
    let mut matches: Vec<(String, PresetConfig, bool)> = image_cfg
        .presets
        .iter()
        .filter_map(|(name, preset)| match_preset(dynamic, name, preset))
        .collect();

    matches.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    matches.into_iter().next().map(|(name, preset, _)| (name, preset))
}

fn match_preset(
    dynamic: DynamicParams,
    name: &str,
    preset: &PresetConfig,
) -> Option<(String, PresetConfig, bool)> {
    let exact_fit = match (dynamic.width, dynamic.height, dynamic.fit) {
        (Some(w), None, None) if preset.width == Some(w) && preset.height.is_none() => false,
        (None, Some(h), None) if preset.height == Some(h) && preset.width.is_none() => false,
        (Some(w), Some(h), Some(fit))
            if preset.width == Some(w)
                && preset.height == Some(h)
                && preset.effective_fit() == fit =>
        {
            true
        }
        (Some(w), Some(h), None) if preset.width == Some(w) && preset.height == Some(h) => {
            preset.fit.is_none()
        }
        _ => return None,
    };

    Some((name.to_string(), preset.clone(), exact_fit))
}

fn validate_dynamic_request(
    dynamic: &DynamicParams,
    image_cfg: &ImageConfig,
    record: &crate::models::image::ImageModel,
) -> AppResult<()> {
    if let Some(width) = dynamic.width {
        if width > image_cfg.dynamic.max_width {
            return Err(AppError::Forbidden);
        }
    }

    if let Some(height) = dynamic.height {
        if height > image_cfg.dynamic.max_height {
            return Err(AppError::Forbidden);
        }
    }

    if !image_cfg.dynamic.allow_enlargement {
        let original_width = u32::try_from(record.width).map_err(|_| AppError::Forbidden)?;
        let original_height = u32::try_from(record.height).map_err(|_| AppError::Forbidden)?;

        if dynamic.width.is_some_and(|width| width > original_width)
            || dynamic.height.is_some_and(|height| height > original_height)
        {
            return Err(AppError::Forbidden);
        }
    }

    Ok(())
}

// ── ETag helpers ────────────────────────────────────────────

/// Compute ETag for a processed view by BLAKE3-hashing the image hash
/// combined with preset compression parameters in fixed order.
fn compute_view_etag(
    image_hash: &str,
    width: Option<u32>,
    height: Option<u32>,
    fit: FitMode,
    format: ImageFormat,
    quality: u8,
) -> String {
    let fit_str = match fit {
        FitMode::Cover => "cover",
        FitMode::Contain => "contain",
        FitMode::Fill => "fill",
        FitMode::Inside => "inside",
        FitMode::Outside => "outside",
    };
    let fmt_str = format_to_ext(format);

    let mut input = String::with_capacity(96);
    let _ = write!(input, "{}:w", image_hash);
    match width {
        Some(w) => {
            let _ = write!(input, "{w}");
        }
        None => input.push('-'),
    }
    input.push_str(":h");
    match height {
        Some(h) => {
            let _ = write!(input, "{h}");
        }
        None => input.push('-'),
    }
    let _ = write!(input, ":f{fit_str}:fmt{fmt_str}:q{quality}");

    let hash = blake3::hash(input.as_bytes());
    format!("\"{}\"", hash.to_hex())
}

/// Check whether the `If-None-Match` request header matches the given ETag.
fn etag_matches(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            let trimmed = v.trim();
            trimmed == "*" || trimmed.split(',').any(|t| t.trim() == etag)
        })
        .unwrap_or(false)
}

/// Build a 304 Not Modified response with ETag and Cache-Control.
fn not_modified(etag: &str, http_max_age: u64) -> Response {
    let cc = cache_control_value(true, http_max_age, 0);
    Response::builder()
        .status(StatusCode::NOT_MODIFIED)
        .header(header::ETAG, etag)
        .header(header::CACHE_CONTROL, cc)
        .body(Body::empty())
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

// ── Response helpers ────────────────────────────────────────

/// Determine the `Cache-Control` header value.
///
/// When negotiated caching is enabled, `http_max_age` from server config
/// controls browser cache duration (0 → `no-cache`, forces revalidation).
/// When disabled, falls back to the preset disk TTL (0 → immutable).
fn cache_control_value(negotiate: bool, http_max_age: u64, disk_ttl: u64) -> String {
    if negotiate {
        if http_max_age == 0 {
            "no-cache".to_string()
        } else {
            format!("public, max-age={http_max_age}")
        }
    } else if disk_ttl == 0 {
        "public, max-age=31536000, immutable".to_string()
    } else {
        format!("public, max-age={disk_ttl}")
    }
}

fn build_response(
    bytes: Vec<u8>,
    mime: &str,
    negotiate: bool,
    http_max_age: u64,
    disk_ttl: u64,
    etag: Option<&str>,
) -> Response {
    let cc = cache_control_value(negotiate, http_max_age, disk_ttl);

    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, mime)
        .header(
            header::CACHE_CONTROL,
            HeaderValue::from_str(&cc)
                .unwrap_or_else(|_| HeaderValue::from_static("public, max-age=3600")),
        );

    if let Some(etag) = etag {
        builder = builder.header(header::ETAG, etag);
    }

    builder
        .body(Body::from(bytes))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

#[cfg(test)]
mod tests {
    use super::{find_matching_preset, validate_dynamic_request};
    use crate::config::{DynamicConfig, FitMode, ImageConfig, ImageFormat, PresetConfig};
    use crate::models::image::ImageModel;
    use crate::path_parser::DynamicParams;
    use chrono::Utc;
    use std::collections::HashMap;

    fn image_cfg() -> ImageConfig {
        let mut presets = HashMap::new();
        presets.insert(
            "cover".to_string(),
            PresetConfig {
                width: Some(800),
                height: Some(600),
                fit: Some(FitMode::Cover),
                format: None,
                quality: None,
                eager: false,
                cache_ttl: Some(120),
            },
        );
        presets.insert(
            "contain".to_string(),
            PresetConfig {
                width: Some(800),
                height: Some(600),
                fit: Some(FitMode::Contain),
                format: None,
                quality: None,
                eager: false,
                cache_ttl: Some(120),
            },
        );
        presets.insert(
            "w800".to_string(),
            PresetConfig {
                width: Some(800),
                height: None,
                fit: None,
                format: None,
                quality: None,
                eager: false,
                cache_ttl: Some(120),
            },
        );

        ImageConfig {
            enable: true,
            default_format: ImageFormat::Webp,
            quality: 75,
            max_workers: 2,
            allow_show_origin: true,
            cache_ttl: 300,
            keep_metadata_fields: vec![],
            dynamic: DynamicConfig {
                allow: true,
                allow_enlargement: false,
                max_width: 1000,
                max_height: 900,
            },
            presets,
        }
    }

    fn record(width: i32, height: i32) -> ImageModel {
        ImageModel {
            id: 1,
            hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
            display_name: "img".into(),
            file_name: "img.jpg".into(),
            extension: "jpg".into(),
            mime_type: "image/jpeg".into(),
            size: 1,
            width,
            height,
            user_id: None,
            is_deleted: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn matches_width_height_without_fit_to_lexicographically_first_preset() {
        let cfg = image_cfg();
        let matched = find_matching_preset(
            &cfg,
            DynamicParams {
                width: Some(800),
                height: Some(600),
                fit: None,
            },
        )
        .unwrap();

        assert_eq!(matched.0, "contain");
    }

    #[test]
    fn matches_exact_fit_first() {
        let cfg = image_cfg();
        let matched = find_matching_preset(
            &cfg,
            DynamicParams {
                width: Some(800),
                height: Some(600),
                fit: Some(FitMode::Cover),
            },
        )
        .unwrap();

        assert_eq!(matched.0, "cover");
    }

    #[test]
    fn validates_dynamic_limits_and_enlargement() {
        let cfg = image_cfg();
        let rec = record(700, 500);

        assert!(validate_dynamic_request(
            &DynamicParams {
                width: Some(600),
                height: None,
                fit: None,
            },
            &cfg,
            &rec,
        )
        .is_ok());

        assert!(validate_dynamic_request(
            &DynamicParams {
                width: Some(1200),
                height: None,
                fit: None,
            },
            &cfg,
            &rec,
        )
        .is_err());

        assert!(validate_dynamic_request(
            &DynamicParams {
                width: Some(800),
                height: None,
                fit: None,
            },
            &cfg,
            &rec,
        )
        .is_err());
    }
}
