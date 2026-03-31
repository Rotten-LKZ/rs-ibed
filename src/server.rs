use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use chrono::{Duration as ChronoDuration, Utc};
use tokio::sync::Semaphore;

use crate::auth::CliTokenStore;
use crate::config::{AppConfig, Secrets};
use crate::db;
use crate::db::repo::ImageRepo;
use crate::router;
use crate::state::AppState;
use crate::storage;

const CACHE_SWEEP_INTERVAL_SECS: u64 = 15 * 60;

pub async fn run(config: AppConfig, secrets: Secrets) {
    tokio::fs::create_dir_all(&config.storage.base_dir)
        .await
        .expect("Failed to create storage base_dir");
    tokio::fs::create_dir_all(&config.storage.cache_dir)
        .await
        .expect("Failed to create storage cache_dir");

    tracing::info!(
        base_dir = %config.storage.base_dir,
        cache_dir = %config.storage.cache_dir,
        "storage directories ready"
    );

    let repo = db::init_repo(&config, &secrets)
        .await
        .expect("Failed to initialize database");

    tracing::info!(driver = %config.database.driver, "database ready");

    let bind_addr = format!("{}:{}", config.server.host, config.server.port);
    let cli_tokens = Arc::new(CliTokenStore::new());

    let max_workers = config.image.max_workers.max(1) as usize;
    let workers = Arc::new(Semaphore::new(max_workers));
    tracing::info!(max_workers, "image worker pool ready");

    let token = cli_tokens.generate();
    let display_host = if config.server.host == "0.0.0.0" {
        "127.0.0.1"
    } else {
        &config.server.host
    };
    tracing::info!(
        "CLI login URL (valid for 3 minutes):\n  http://{}:{}/login?token={}",
        display_host,
        config.server.port,
        token
    );

    let state = AppState {
        repo,
        config: Arc::new(config),
        secrets: Arc::new(secrets),
        cli_tokens,
        workers,
    };

    spawn_cache_sweeper(Arc::clone(&state.config), Arc::clone(&state.repo));

    let app = router::build(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("Failed to bind");

    tracing::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.expect("server error");
}

fn spawn_cache_sweeper(config: Arc<AppConfig>, repo: Arc<dyn ImageRepo>) {
    let cache_dir = config.storage.cache_dir.clone();
    let dynamic_ttl = config.image.cache_ttl;
    let ttl_map = preset_ttl_map(&config);
    let trash_retention_days = config.server.trash_retention_days;
    let base_dir = config.storage.base_dir.clone();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(CACHE_SWEEP_INTERVAL_SECS));
        interval.tick().await;

        loop {
            interval.tick().await;
            match sweep_cache_dir(&cache_dir, &ttl_map, dynamic_ttl).await {
                Ok(stats) => {
                    tracing::info!(
                        scanned_files = stats.scanned_files,
                        deleted_files = stats.deleted_files,
                        deleted_dirs = stats.deleted_dirs,
                        "cache sweep finished"
                    );
                }
                Err(e) => {
                    tracing::warn!(cache_dir = %cache_dir, "cache sweep failed: {e}");
                }
            }

            if trash_retention_days > 0 {
                let cutoff = Utc::now() - ChronoDuration::days(trash_retention_days as i64);
                match repo.find_expired_deleted(cutoff).await {
                    Ok(images) => {
                        for img in &images {
                            let src = storage::original_path(&base_dir, &img.hash, &img.extension);
                            let _ = tokio::fs::remove_file(&src).await;
                            let _ = storage::remove_all_cache_for_hash(&cache_dir, &img.hash).await;
                        }
                        let count = images.len();
                        if let Err(e) = repo.delete_all_deleted().await {
                            tracing::warn!("trash sweep DB delete failed: {e}");
                        } else if count > 0 {
                            tracing::info!(expired_images = count, "trash sweep finished");
                        }
                    }
                    Err(e) => {
                        tracing::warn!("trash sweep failed: {e}");
                    }
                }
            }
        }
    });
}

fn preset_ttl_map(config: &AppConfig) -> HashMap<String, Option<u64>> {
    config
        .image
        .presets
        .iter()
        .map(|(name, preset)| {
            let ttl = if storage::preset_is_permanent(preset, config.image.cache_ttl) {
                None
            } else {
                Some(preset.effective_ttl(config.image.cache_ttl))
            };
            (name.clone(), ttl)
        })
        .collect()
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct SweepStats {
    scanned_files: usize,
    deleted_files: usize,
    deleted_dirs: usize,
}

async fn sweep_cache_dir(
    cache_dir: &str,
    preset_ttls: &HashMap<String, Option<u64>>,
    dynamic_ttl: u64,
) -> std::io::Result<SweepStats> {
    let mut stats = SweepStats::default();
    let cache_root = Path::new(cache_dir);

    if tokio::fs::metadata(cache_root).await.is_err() {
        return Ok(stats);
    }

    let mut dirs_to_check = Vec::new();
    let mut stack = vec![cache_root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        dirs_to_check.push(dir.clone());
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

            if !ty.is_file() {
                continue;
            }

            stats.scanned_files += 1;

            let Some(variant_key) = variant_key_from_cache_path(cache_root, &path) else {
                continue;
            };

            let Some(ttl) = ttl_for_variant_key(&variant_key, preset_ttls, dynamic_ttl) else {
                continue;
            };

            if is_cache_file_expired(&path, ttl).await? {
                tokio::fs::remove_file(&path).await?;
                stats.deleted_files += 1;
            }
        }
    }

    let mut seen = HashSet::new();
    dirs_to_check.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for dir in dirs_to_check {
        if !seen.insert(dir.clone()) || dir == cache_root {
            continue;
        }
        if remove_dir_if_empty(&dir).await? {
            stats.deleted_dirs += 1;
        }
    }

    Ok(stats)
}

fn variant_key_from_cache_path(cache_root: &Path, file_path: &Path) -> Option<String> {
    let relative = file_path.strip_prefix(cache_root).ok()?;
    let components: Vec<String> = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();

    if components.len() < 4 {
        return None;
    }

    Some(components[..components.len() - 3].join(std::path::MAIN_SEPARATOR_STR))
}

fn ttl_for_variant_key(
    variant_key: &str,
    preset_ttls: &HashMap<String, Option<u64>>,
    dynamic_ttl: u64,
) -> Option<u64> {
    if storage::is_dynamic_variant_key(variant_key) {
        return (dynamic_ttl != 0).then_some(dynamic_ttl);
    }

    match preset_ttls.get(variant_key) {
        Some(Some(ttl)) if *ttl != 0 => Some(*ttl),
        Some(None) => None,
        Some(Some(_)) => None,
        None => Some(dynamic_ttl),
    }
}

async fn is_cache_file_expired(path: &Path, ttl_secs: u64) -> std::io::Result<bool> {
    let metadata = tokio::fs::metadata(path).await?;
    let modified = metadata.modified()?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_else(|_| Duration::from_secs(0));
    Ok(age >= Duration::from_secs(ttl_secs))
}

async fn remove_dir_if_empty(path: &PathBuf) -> std::io::Result<bool> {
    let mut entries = match tokio::fs::read_dir(path).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };

    if entries.next_entry().await?.is_none() {
        tokio::fs::remove_dir(path).await?;
        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{SweepStats, sweep_cache_dir, ttl_for_variant_key, variant_key_from_cache_path};
    use std::collections::HashMap;
    use std::path::Path;
    use std::time::Duration;

    #[test]
    fn extracts_variant_key_from_cache_path() {
        let root = Path::new("cache");
        let path = Path::new("cache/dynamic/w800/ab/cd/hash.webp");
        let key = variant_key_from_cache_path(root, path).unwrap();
        assert_eq!(key, format!("dynamic{}w800", std::path::MAIN_SEPARATOR));
    }

    #[test]
    fn resolves_ttl_for_variant_key() {
        let mut preset_ttls = HashMap::new();
        preset_ttls.insert("thumb".to_string(), Some(60));
        preset_ttls.insert("hero".to_string(), None);

        assert_eq!(ttl_for_variant_key("thumb", &preset_ttls, 120), Some(60));
        assert_eq!(ttl_for_variant_key("hero", &preset_ttls, 120), None);
        assert_eq!(
            ttl_for_variant_key(&format!("dynamic{}w800", std::path::MAIN_SEPARATOR), &preset_ttls, 120),
            Some(120)
        );
        assert_eq!(ttl_for_variant_key("default", &preset_ttls, 120), Some(120));
    }

    #[tokio::test]
    async fn removes_expired_dynamic_cache_files() {
        let base = std::env::temp_dir().join(format!(
            "rs_ibed_cache_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file = base.join("dynamic").join("w800").join("ab").join("cd").join("hash.webp");
        tokio::fs::create_dir_all(file.parent().unwrap()).await.unwrap();
        tokio::fs::write(&file, b"cached").await.unwrap();

        tokio::time::sleep(Duration::from_secs(2)).await;

        let stats = sweep_cache_dir(base.to_str().unwrap(), &HashMap::new(), 1)
            .await
            .unwrap();

        assert_eq!(
            stats,
            SweepStats {
                scanned_files: 1,
                deleted_files: 1,
                deleted_dirs: 4,
            }
        );
        assert!(!tokio::fs::try_exists(&file).await.unwrap());

        let _ = tokio::fs::remove_dir_all(&base).await;
    }

    #[tokio::test]
    async fn keeps_permanent_preset_cache_files() {
        let base = std::env::temp_dir().join(format!(
            "rs_ibed_cache_test_perm_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let file = base.join("hero").join("ab").join("cd").join("hash.webp");
        tokio::fs::create_dir_all(file.parent().unwrap()).await.unwrap();
        tokio::fs::write(&file, b"cached").await.unwrap();

        tokio::time::sleep(Duration::from_secs(2)).await;

        let mut preset_ttls = HashMap::new();
        preset_ttls.insert("hero".to_string(), None);

        let stats = sweep_cache_dir(base.to_str().unwrap(), &preset_ttls, 1)
            .await
            .unwrap();

        assert_eq!(stats.deleted_files, 0);
        assert!(tokio::fs::try_exists(&file).await.unwrap());

        let _ = tokio::fs::remove_dir_all(&base).await;
    }
}
