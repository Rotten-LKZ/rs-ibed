use crate::error::AppError;

/// Supported image extensions and their MIME types.
const SUPPORTED: &[(&str, &str)] = &[
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("png", "image/png"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("avif", "image/avif"),
    ("bmp", "image/bmp"),
    ("tiff", "image/tiff"),
    ("ico", "image/x-icon"),
    ("svg", "image/svg+xml"),
];

pub struct DetectedType {
    pub extension: String,
    pub mime_type: String,
}

/// Detect the real image type from file bytes using magic bytes.
/// Returns an error if the file is not a supported image type.
pub fn detect_image_type(data: &[u8]) -> Result<DetectedType, AppError> {
    // SVG detection: infer doesn't detect SVG reliably, check manually
    if is_svg(data) {
        return Ok(DetectedType {
            extension: "svg".into(),
            mime_type: "image/svg+xml".into(),
        });
    }

    let kind = infer::get(data).ok_or_else(|| {
        AppError::BadRequest("unable to detect file type from content".into())
    })?;

    let mime = kind.mime_type();

    // Find a matching supported type
    for &(_, supported_mime) in SUPPORTED {
        if mime == supported_mime {
            // Normalize: use canonical extension
            let canonical_ext = canonical_ext(mime);
            return Ok(DetectedType {
                extension: canonical_ext.to_string(),
                mime_type: mime.to_string(),
            });
        }
    }

    Err(AppError::BadRequest(format!(
        "unsupported image type: {mime}"
    )))
}

/// Return the canonical extension for a MIME type.
fn canonical_ext(mime: &str) -> &str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/avif" => "avif",
        "image/bmp" => "bmp",
        "image/tiff" => "tiff",
        "image/x-icon" => "ico",
        "image/svg+xml" => "svg",
        _ => "bin",
    }
}

/// Basic SVG detection: look for `<svg` in the first 1024 bytes (skipping BOM / whitespace).
fn is_svg(data: &[u8]) -> bool {
    let check = if data.len() > 1024 {
        &data[..1024]
    } else {
        data
    };
    let text = std::str::from_utf8(check).unwrap_or("");
    let trimmed = text.trim_start();
    trimmed.starts_with("<?xml") && trimmed.contains("<svg")
        || trimmed.starts_with("<svg")
}
