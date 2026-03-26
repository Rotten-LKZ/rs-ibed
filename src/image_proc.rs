use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::{DynamicImage, ImageEncoder, ImageFormat as ImgFmt, imageops::FilterType};
use tokio::sync::Semaphore;

use crate::config::{FitMode, ImageConfig, ImageFormat, PresetConfig};
use crate::error::{AppError, AppResult};

/// Returns the file extension string for a given ImageFormat.
pub fn format_to_ext(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Webp => "webp",
        ImageFormat::Avif => "avif",
        ImageFormat::Jpeg => "jpg",
        ImageFormat::Png => "png",
        ImageFormat::Original => "bin",
    }
}

/// Returns the MIME type string for a given ImageFormat.
pub fn format_to_mime(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Webp => "image/webp",
        ImageFormat::Avif => "image/avif",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::Png => "image/png",
        ImageFormat::Original => "application/octet-stream",
    }
}

/// Process image bytes according to a preset and global config.
///
/// Acquires a semaphore permit first so that at most `max_workers`
/// CPU-intensive image operations run concurrently, preventing the
/// blocking thread pool from being overwhelmed and keeping normal
/// API requests responsive.
///
/// Returns `(output_bytes, mime_type)`.
pub async fn process_image(
    data: &[u8],
    preset: &PresetConfig,
    global: &ImageConfig,
    semaphore: &Semaphore,
) -> AppResult<(Vec<u8>, String)> {
    // Acquire worker permit — this is the concurrency gate.
    // Other requests can proceed on the async runtime while we wait.
    let _permit = semaphore
        .acquire()
        .await
        .map_err(|_| AppError::Internal("worker pool closed".into()))?;

    let data = data.to_vec();
    let preset_width = preset.width;
    let preset_height = preset.height;
    let fit = preset.effective_fit();
    let format = preset.effective_format(global.default_format);
    let quality = preset.effective_quality(global.quality);

    // Move the heavy decode/resize/encode work off the async runtime
    // onto a dedicated blocking thread.
    tokio::task::spawn_blocking(move || {
        let img = image::load_from_memory(&data).map_err(AppError::Image)?;
        let img = resize_image(img, preset_width, preset_height, fit);
        encode_image(img, format, quality)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

fn resize_image(
    img: DynamicImage,
    width: Option<u32>,
    height: Option<u32>,
    fit: FitMode,
) -> DynamicImage {
    match (width, height) {
        (None, None) => img,
        (Some(w), None) => img.resize(w, u32::MAX, FilterType::Lanczos3),
        (None, Some(h)) => img.resize(u32::MAX, h, FilterType::Lanczos3),
        (Some(w), Some(h)) => match fit {
            FitMode::Cover => img.resize_to_fill(w, h, FilterType::Lanczos3),
            FitMode::Contain | FitMode::Inside => img.resize(w, h, FilterType::Lanczos3),
            FitMode::Fill => img.resize_exact(w, h, FilterType::Lanczos3),
            FitMode::Outside => {
                let ratio_w = w as f64 / img.width() as f64;
                let ratio_h = h as f64 / img.height() as f64;
                let ratio = ratio_w.max(ratio_h);
                let new_w = (img.width() as f64 * ratio).ceil() as u32;
                let new_h = (img.height() as f64 * ratio).ceil() as u32;
                img.resize_exact(new_w, new_h, FilterType::Lanczos3)
            }
        },
    }
}

fn encode_image(
    img: DynamicImage,
    format: ImageFormat,
    quality: u8,
) -> AppResult<(Vec<u8>, String)> {
    let mime = format_to_mime(format).to_string();

    let bytes = match format {
        ImageFormat::Jpeg => {
            let mut buf = Vec::new();
            let encoder = JpegEncoder::new_with_quality(&mut buf, quality);
            let rgba = img.to_rgba8();
            encoder
                .write_image(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(AppError::Image)?;
            buf
        }
        ImageFormat::Png => {
            let mut buf = Vec::new();
            let encoder = PngEncoder::new(&mut buf);
            let rgba = img.to_rgba8();
            encoder
                .write_image(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(AppError::Image)?;
            buf
        }
        ImageFormat::Webp => {
            let mut buf = Cursor::new(Vec::new());
            img.write_to(&mut buf, ImgFmt::WebP)?;
            buf.into_inner()
        }
        ImageFormat::Avif => {
            let mut buf = Cursor::new(Vec::new());
            img.write_to(&mut buf, ImgFmt::Avif)?;
            buf.into_inner()
        }
        ImageFormat::Original => {
            // No re-encoding; encode as PNG as a safe fallback.
            let mut buf = Cursor::new(Vec::new());
            img.write_to(&mut buf, ImgFmt::Png)?;
            buf.into_inner()
        }
    };

    Ok((bytes, mime))
}
