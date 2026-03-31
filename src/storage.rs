use std::path::{Path, PathBuf};

use crate::config::{FitMode, PresetConfig};
use crate::path_parser::DynamicParams;

/// Build the on-disk path for an original image:
/// `{base_dir}/{hash[0..2]}/{hash[2..4]}/{hash}.{ext}`
pub fn original_path(base_dir: &str, hash: &str, ext: &str) -> PathBuf {
    let (a, b) = hash_prefix(hash);
    [base_dir, a, b, &format!("{hash}.{ext}")]
        .iter()
        .collect()
}

/// Build the on-disk path for a cached (processed) image:
/// `{cache_dir}/{variant_key}/{hash[0..2]}/{hash[2..4]}/{hash}.{ext}`
pub fn cache_path(cache_dir: &str, variant_key: &str, hash: &str, ext: &str) -> PathBuf {
    let (a, b) = hash_prefix(hash);
    Path::new(cache_dir)
        .join(variant_key)
        .join(a)
        .join(b)
        .join(format!("{hash}.{ext}"))
}

pub fn preset_variant_key(preset_name: &str) -> String {
    preset_name.to_string()
}

pub fn dynamic_variant_key(params: DynamicParams) -> String {
    let mut parts = Vec::with_capacity(3);

    if let Some(width) = params.width {
        parts.push(format!("w{width}"));
    }
    if let Some(height) = params.height {
        parts.push(format!("h{height}"));
    }
    if let Some(fit) = params.fit {
        parts.push(format!("f{}", fit_suffix(fit)));
    }

    format!("dynamic{}{}", std::path::MAIN_SEPARATOR, parts.join("_"))
}

pub fn preset_is_permanent(preset: &PresetConfig, global_ttl: u64) -> bool {
    preset.eager || preset.effective_ttl(global_ttl) == 0
}

pub fn is_dynamic_variant_key(variant_key: &str) -> bool {
    Path::new(variant_key)
        .components()
        .next()
        .map(|segment| segment.as_os_str() == "dynamic")
        .unwrap_or(false)
}

/// Extract the first two directory layers from a hash: (hash[0..2], hash[2..4]).
fn hash_prefix(hash: &str) -> (&str, &str) {
    (&hash[..2], &hash[2..4])
}

/// Remove all cached files for a given hash by walking the cache_dir recursively
/// and deleting every file whose filename starts with `{hash}.`.
/// Returns the number of files deleted.
pub async fn remove_all_cache_for_hash(cache_dir: &str, hash: &str) -> std::io::Result<usize> {
    let prefix = format!("{hash}.");
    let cache_root = Path::new(cache_dir);
    let mut deleted = 0;

    if tokio::fs::metadata(cache_root).await.is_err() {
        return Ok(0);
    }

    let mut stack = vec![cache_root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut entries = match tokio::fs::read_dir(&dir).await {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let ty = entry.file_type().await?;

            if ty.is_dir() {
                stack.push(path);
                continue;
            }

            if ty.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with(&prefix) {
                        tokio::fs::remove_file(&path).await?;
                        deleted += 1;
                    }
                }
            }
        }
    }

    Ok(deleted)
}

fn fit_suffix(fit: FitMode) -> &'static str {
    match fit {
        FitMode::Cover => "cover",
        FitMode::Contain => "contain",
        FitMode::Fill => "fill",
        FitMode::Inside => "inside",
        FitMode::Outside => "outside",
    }
}

#[cfg(test)]
mod tests {
    use super::{cache_path, dynamic_variant_key, is_dynamic_variant_key, preset_is_permanent};
    use crate::config::{DynamicConfig, FitMode, ImageFormat, PresetConfig};
    use crate::path_parser::DynamicParams;

    #[test]
    fn builds_dynamic_variant_key() {
        let key = dynamic_variant_key(DynamicParams {
            width: Some(800),
            height: Some(600),
            fit: Some(FitMode::Cover),
        });

        let expected = format!("dynamic{}w800_h600_fcover", std::path::MAIN_SEPARATOR);
        assert_eq!(key, expected);
        assert!(is_dynamic_variant_key(&key));
    }

    #[test]
    fn builds_cache_path_with_nested_variant_key() {
        let path = cache_path(
            "cache",
            &format!("dynamic{}w800", std::path::MAIN_SEPARATOR),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "webp",
        );

        let rendered = path.to_string_lossy();
        assert!(rendered.contains("dynamic"));
        assert!(rendered.contains("w800"));
        assert!(rendered.ends_with(".webp"));
    }

    #[test]
    fn treats_eager_and_zero_ttl_presets_as_permanent() {
        let eager = PresetConfig {
            width: Some(100),
            height: Some(100),
            fit: Some(FitMode::Cover),
            format: Some(ImageFormat::Webp),
            quality: Some(75),
            eager: true,
            cache_ttl: Some(60),
        };
        let zero_ttl = PresetConfig {
            width: None,
            height: None,
            fit: None,
            format: None,
            quality: None,
            eager: false,
            cache_ttl: Some(0),
        };
        let normal = PresetConfig {
            width: None,
            height: None,
            fit: None,
            format: None,
            quality: None,
            eager: false,
            cache_ttl: Some(300),
        };

        assert!(preset_is_permanent(&eager, 100));
        assert!(preset_is_permanent(&zero_ttl, 100));
        assert!(!preset_is_permanent(&normal, 100));

        let _ = DynamicConfig::default();
    }
}
