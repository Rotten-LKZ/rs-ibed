use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, RwLock as StdRwLock};
use tokio::sync::RwLock;

use aws_config::BehaviorVersion;
use aws_sdk_s3::config::{Credentials, Region};

use crate::config::{DirectMode, EndpointConfig, EndpointType, FitMode, PresetConfig, Secrets};
use crate::error::{AppError, AppResult};
use crate::path_parser::DynamicParams;
use crate::storage_backend::local::LocalBackend;
use crate::storage_backend::s3::S3Backend;
use crate::storage_backend::StorageBackend;

// ── Path utilities ────────────────────────────────────────────────────────────

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

// ── Backend-agnostic object key helpers ──────────────────────────────────────

/// Compute the object key for an original image file.
/// Layout: `origin/{hash[0:2]}/{hash[2:4]}/{hash}.{ext}`
pub fn object_key_original(hash: &str, ext: &str) -> String {
    format!("origin/{}/{}/{}.{}", &hash[..2], &hash[2..4], hash, ext)
}

/// Compute the object key for a cached (processed) image file.
/// Layout: `cache/{variant_key}/{hash[0:2]}/{hash[2:4]}/{hash}.{ext}`
pub fn object_key_cache(variant_key: &str, hash: &str, ext: &str) -> String {
    format!("cache/{}/{}/{}/{}.{}", variant_key, &hash[..2], &hash[2..4], hash, ext)
}

// ── DB pool abstraction ───────────────────────────────────────────────────────

/// Holds either a SQLite or PostgreSQL pool for storage endpoint operations.
pub enum StoragePool {
    Sqlite(sqlx::SqlitePool),
    Postgres(sqlx::PgPool),
}

impl StoragePool {
    async fn upsert_endpoint(&self, ep: &EndpointConfig) -> AppResult<()> {
        match self {
            StoragePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT OR IGNORE INTO storage_endpoints \
                     (name, description, capacity_bytes, priority) \
                     VALUES (?, ?, ?, ?)"
                )
                .bind(&ep.name)
                .bind(&ep.description)
                .bind(ep.capacity_bytes)
                .bind(ep.priority)
                .execute(pool)
                .await
                .map_err(AppError::Db)?;

                // Update config-sourced fields (never overwrite used_size or status)
                sqlx::query(
                    "UPDATE storage_endpoints \
                     SET description = ?, capacity_bytes = ?, \
                         updated_at = CURRENT_TIMESTAMP \
                     WHERE name = ?"
                )
                .bind(&ep.description)
                .bind(ep.capacity_bytes)
                .bind(&ep.name)
                .execute(pool)
                .await
                .map_err(AppError::Db)?;

                Ok(())
            }
            StoragePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO storage_endpoints \
                     (name, description, capacity_bytes, priority) \
                     VALUES ($1, $2, $3, $4) \
                     ON CONFLICT (name) DO UPDATE \
                     SET description = EXCLUDED.description, \
                         capacity_bytes = EXCLUDED.capacity_bytes, \
                         updated_at = NOW()"
                )
                .bind(&ep.name)
                .bind(&ep.description)
                .bind(ep.capacity_bytes)
                .bind(ep.priority)
                .execute(pool)
                .await
                .map_err(AppError::Db)?;

                Ok(())
            }
        }
    }

    async fn disable_missing_endpoints(&self, active_names: &[String]) -> AppResult<()> {
        if active_names.is_empty() {
            match self {
                StoragePool::Sqlite(pool) => {
                    sqlx::query("UPDATE storage_endpoints SET status = 'disabled'")
                        .execute(pool)
                        .await
                        .map_err(AppError::Db)?;
                }
                StoragePool::Postgres(pool) => {
                    sqlx::query("UPDATE storage_endpoints SET status = 'disabled'")
                        .execute(pool)
                        .await
                        .map_err(AppError::Db)?;
                }
            }
            return Ok(());
        }

        match self {
            StoragePool::Sqlite(pool) => {
                let ph: Vec<String> = active_names.iter().map(|_| "?".to_string()).collect();
                let sql = format!(
                    "UPDATE storage_endpoints SET status = 'disabled' WHERE name NOT IN ({})",
                    ph.join(", ")
                );
                let mut q = sqlx::query(&sql);
                for name in active_names {
                    q = q.bind(name);
                }
                q.execute(pool).await.map_err(AppError::Db)?;
            }
            StoragePool::Postgres(pool) => {
                let ph: Vec<String> = (1..=active_names.len())
                    .map(|i| format!("${i}"))
                    .collect();
                let sql = format!(
                    "UPDATE storage_endpoints SET status = 'disabled' WHERE name NOT IN ({})",
                    ph.join(", ")
                );
                let mut q = sqlx::query(&sql);
                for name in active_names {
                    q = q.bind(name);
                }
                q.execute(pool).await.map_err(AppError::Db)?;
            }
        }

        Ok(())
    }

    async fn load_active_endpoints(
        &self,
    ) -> AppResult<Vec<crate::models::storage_endpoint::StorageEndpointModel>> {
        use crate::models::storage_endpoint::StorageEndpointModel;
        match self {
            StoragePool::Sqlite(pool) => {
                let rows = sqlx::query_as::<_, StorageEndpointModel>(
                    "SELECT name, capacity_bytes, used_size, \
                     priority, status \
                     FROM storage_endpoints WHERE status = 'active' ORDER BY priority ASC, name ASC",
                )
                .fetch_all(pool)
                .await
                .map_err(AppError::Db)?;
                Ok(rows)
            }
            StoragePool::Postgres(pool) => {
                let rows = sqlx::query_as::<_, StorageEndpointModel>(
                    "SELECT name, capacity_bytes, used_size, \
                     priority, status \
                     FROM storage_endpoints WHERE status = 'active' ORDER BY priority ASC, name ASC",
                )
                .fetch_all(pool)
                .await
                .map_err(AppError::Db)?;
                Ok(rows)
            }
        }
    }

    async fn update_used_size(&self, name: &str, delta: i64) -> AppResult<()> {
        match self {
            StoragePool::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE storage_endpoints SET used_size = used_size + ?, \
                     updated_at = CURRENT_TIMESTAMP WHERE name = ?",
                )
                .bind(delta)
                .bind(name)
                .execute(pool)
                .await
                .map_err(AppError::Db)?;
            }
            StoragePool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE storage_endpoints SET used_size = used_size + $1, \
                     updated_at = NOW() WHERE name = $2",
                )
                .bind(delta)
                .bind(name)
                .execute(pool)
                .await
                .map_err(AppError::Db)?;
            }
        }
        Ok(())
    }

    /// Overwrite `used_size` with an absolute value (for reconciliation).
    async fn set_used_size(&self, name: &str, size: i64) -> AppResult<()> {
        match self {
            StoragePool::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE storage_endpoints SET used_size = ?, \
                     updated_at = CURRENT_TIMESTAMP WHERE name = ?",
                )
                .bind(size)
                .bind(name)
                .execute(pool)
                .await
                .map_err(AppError::Db)?;
            }
            StoragePool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE storage_endpoints SET used_size = $1, \
                     updated_at = NOW() WHERE name = $2",
                )
                .bind(size)
                .bind(name)
                .execute(pool)
                .await
                .map_err(AppError::Db)?;
            }
        }
        Ok(())
    }

    /// Update only `description` and/or `status` — the two UI-mutable fields.
    pub async fn update_endpoint_fields(
        &self,
        name: &str,
        description: Option<&str>,
        status: Option<&str>,
    ) -> AppResult<()> {
        match self {
            StoragePool::Sqlite(pool) => {
                if let Some(v) = description {
                    sqlx::query(
                        "UPDATE storage_endpoints SET description = ?, updated_at = CURRENT_TIMESTAMP WHERE name = ?",
                    )
                    .bind(v)
                    .bind(name)
                    .execute(pool)
                    .await
                    .map_err(AppError::Db)?;
                }
                if let Some(v) = status {
                    sqlx::query(
                        "UPDATE storage_endpoints SET status = ?, updated_at = CURRENT_TIMESTAMP WHERE name = ?",
                    )
                    .bind(v)
                    .bind(name)
                    .execute(pool)
                    .await
                    .map_err(AppError::Db)?;
                }
            }
            StoragePool::Postgres(pool) => {
                if let Some(v) = description {
                    sqlx::query(
                        "UPDATE storage_endpoints SET description = $1, updated_at = NOW() WHERE name = $2",
                    )
                    .bind(v)
                    .bind(name)
                    .execute(pool)
                    .await
                    .map_err(AppError::Db)?;
                }
                if let Some(v) = status {
                    sqlx::query(
                        "UPDATE storage_endpoints SET status = $1, updated_at = NOW() WHERE name = $2",
                    )
                    .bind(v)
                    .bind(name)
                    .execute(pool)
                    .await
                    .map_err(AppError::Db)?;
                }
            }
        }
        Ok(())
    }
}

// ── EndpointEntry ─────────────────────────────────────────────────────────────

pub struct EndpointEntry {
    pub name: String,
    pub priority: i32,
    pub capacity_bytes: i64,
    /// Backend type string ("Local" or "S3") — for API responses only.
    pub endpoint_type: String,
    pub status: String,
    pub used_size: Arc<AtomicI64>,
    pub backend: Arc<dyn StorageBackend>,
    pub direct_mode: DirectMode,
    pub direct_url_ttl: Option<u64>,
}

// ── StorageRouter ─────────────────────────────────────────────────────────────

pub struct StorageRouter {
    /// priority → ring (virtual_node_u32 → endpoint_name)
    rings: BTreeMap<i32, BTreeMap<u32, String>>,
    endpoints: HashMap<String, Arc<EndpointEntry>>,
}

impl StorageRouter {
    fn build(endpoints: &HashMap<String, Arc<EndpointEntry>>) -> Self {
        let mut rings: BTreeMap<i32, BTreeMap<u32, String>> = BTreeMap::new();

        for (name, entry) in endpoints {
            if entry.status != "active" {
                continue;
            }
            let ring = rings.entry(entry.priority).or_default();
            let node_count = (entry.capacity_bytes / 1_073_741_824).max(1) * 100;
            for i in 0..node_count {
                let key_str = format!("{}#{}", name, i);
                let hash = blake3::hash(key_str.as_bytes());
                let bytes = hash.as_bytes();
                let pos = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                ring.insert(pos, name.clone());
            }
        }

        Self {
            rings,
            endpoints: endpoints.clone(),
        }
    }

    /// Hash the content hash string to a ring position.
    fn position_for(hash: &str) -> u32 {
        let h = blake3::hash(hash.as_bytes());
        let bytes = h.as_bytes();
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    /// Select an endpoint for writing a file (original or cache).
    /// Uses clockwise probing with capacity check and priority waterfall.
    /// Returns `None` if all endpoints are full or disabled.
    pub fn select_for_write(
        &self,
        content_hash: &str,
        file_size: i64,
    ) -> Option<Arc<EndpointEntry>> {
        let pos = Self::position_for(content_hash);

        for (_priority, ring) in &self.rings {
            let candidate = ring
                .range(pos..)
                .chain(ring.iter())
                .find_map(|(_, name)| {
                    let ep = self.endpoints.get(name)?;
                    if ep.status != "active" {
                        return None;
                    }
                    let used = ep.used_size.load(Ordering::Relaxed);
                    if used + file_size > ep.capacity_bytes {
                        return None;
                    }
                    Some(Arc::clone(ep))
                });

            if candidate.is_some() {
                return candidate;
            }
        }
        None
    }

    /// Return all active endpoints in clockwise probing order across all priority tiers.
    /// Used by view/download handlers to sequentially HEAD-probe for a file.
    pub fn probe_for_read(&self, content_hash: &str) -> Vec<Arc<EndpointEntry>> {
        let pos = Self::position_for(content_hash);
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for (_priority, ring) in &self.rings {
            for (_, name) in ring.range(pos..).chain(ring.iter()) {
                if seen.insert(name.clone()) {
                    if let Some(ep) = self.endpoints.get(name) {
                        if ep.status == "active" {
                            result.push(Arc::clone(ep));
                        }
                    }
                }
            }
        }
        result
    }
}

// ── StorageManager ────────────────────────────────────────────────────────────

pub struct StorageManager {
    router: StdRwLock<StorageRouter>,
    endpoints: RwLock<HashMap<String, Arc<EndpointEntry>>>,
    pool: StoragePool,
}

impl StorageManager {
    pub fn router_read(&self) -> std::sync::RwLockReadGuard<'_, StorageRouter> {
        self.router.read().expect("storage router lock poisoned")
    }

    /// Atomically update the in-memory used_size and persist to DB.
    pub async fn adjust_used_size(&self, name: &str, delta: i64) -> AppResult<()> {
        {
            let endpoints = self.endpoints.read().await;
            if let Some(ep) = endpoints.get(name) {
                ep.used_size.fetch_add(delta, Ordering::Relaxed);
            }
        }
        self.pool.update_used_size(name, delta).await
    }

    /// Rebuild the router from the current in-memory endpoints.
    pub async fn rebuild_router(&self) {
        let endpoints = self.endpoints.read().await;
        let new_router = StorageRouter::build(&endpoints);
        drop(endpoints);
        let mut guard = self.router.write().expect("storage router lock poisoned");
        *guard = new_router;
    }

    pub async fn get_endpoint(&self, name: &str) -> Option<Arc<EndpointEntry>> {
        let endpoints = self.endpoints.read().await;
        endpoints.get(name).map(Arc::clone)
    }

    pub async fn list_active(&self) -> Vec<Arc<EndpointEntry>> {
        let endpoints = self.endpoints.read().await;
        endpoints
            .values()
            .filter(|ep| ep.status == "active")
            .map(Arc::clone)
            .collect()
    }

    /// Return all endpoints (active and disabled) for admin listing.
    pub async fn list_all(&self) -> Vec<Arc<EndpointEntry>> {
        let endpoints = self.endpoints.read().await;
        endpoints.values().map(Arc::clone).collect()
    }

    /// Update `description` and/or `status` in DB and in-memory.
    /// `priority` and `capacity_bytes` are config-immutable and rejected at the handler layer.
    pub async fn update_endpoint_fields(
        &self,
        name: &str,
        description: Option<&str>,
        status: Option<&str>,
    ) -> AppResult<()> {
        self.pool
            .update_endpoint_fields(name, description, status)
            .await?;

        let mut endpoints = self.endpoints.write().await;
        if let Some(entry) = endpoints.get(name) {
            let updated = Arc::new(EndpointEntry {
                name: entry.name.clone(),
                priority: entry.priority,
                capacity_bytes: entry.capacity_bytes,
                endpoint_type: entry.endpoint_type.clone(),
                status: status.map(|s| s.to_string()).unwrap_or_else(|| entry.status.clone()),
                used_size: Arc::clone(&entry.used_size),
                backend: Arc::clone(&entry.backend),
                direct_mode: entry.direct_mode.clone(),
                direct_url_ttl: entry.direct_url_ttl,
                // description is not stored in EndpointEntry (it's cosmetic)
            });
            endpoints.insert(name.to_string(), updated);
        }
        drop(endpoints);

        Ok(())
    }

    /// For each active endpoint, query its actual total used bytes and overwrite
    /// both the in-memory AtomicI64 and the DB `used_size`. Called periodically
    /// to reconcile drift caused by S3 lifecycle deletions or other out-of-band changes.
    pub async fn reconcile_used_sizes(&self) {
        let endpoints = self.endpoints.read().await;
        for ep in endpoints.values() {
            if ep.status != "active" {
                continue;
            }
            match ep.backend.total_used_bytes().await {
                Ok(actual) => {
                    ep.used_size.store(actual, std::sync::atomic::Ordering::Relaxed);
                    if let Err(e) = self.pool.set_used_size(&ep.name, actual).await {
                        tracing::warn!(endpoint = %ep.name, "reconcile DB write failed: {e}");
                    } else {
                        tracing::debug!(endpoint = %ep.name, used_bytes = actual, "reconciled used_size");
                    }
                }
                Err(e) => {
                    tracing::warn!(endpoint = %ep.name, "reconcile list failed: {e}");
                }
            }
        }
    }

    /// Check whether the original file for the given hash exists on at least one active node.
    pub async fn check_file_available(&self, hash: &str, ext: &str) -> bool {
        let key = object_key_original(hash, ext);
        let probes = {
            let router = self.router.read().expect("storage router lock poisoned");
            router.probe_for_read(hash)
        };
        for ep in &probes {
            if let Ok(Some(_)) = ep.backend.head_object(&key).await {
                return true;
            }
        }
        false
    }

    /// Generate a direct URL for the original image at the given hash.
    /// Returns None if the endpoint's direct_mode is Proxy or if the endpoint is Local.
    pub async fn direct_url_for(
        &self,
        hash: &str,
        ext: &str,
        file_size: i64,
    ) -> Option<String> {
        let key = object_key_original(hash, ext);
        self.direct_url_for_key(hash, file_size, &key).await
    }

    async fn direct_url_for_key(
        &self,
        hash: &str,
        file_size: i64,
        key: &str,
    ) -> Option<String> {
        let (ep, mode) = {
            let router = self.router.read().expect("storage router lock poisoned");
            let ep = router.select_for_write(hash, file_size)?;
            (Arc::clone(&ep), ep.direct_mode.clone())
        };

        match mode {
            DirectMode::Proxy => None,
            DirectMode::Presigned => {
                let ttl = ep.direct_url_ttl.unwrap_or(3600);
                ep.backend
                    .presigned_get_url(key, std::time::Duration::from_secs(ttl))
                    .await
                    .ok()
                    .flatten()
            }
            DirectMode::Public => ep.backend.public_url(key),
        }
    }
}

// ── sync_and_build ────────────────────────────────────────────────────────────

/// On startup: sync config endpoints with the DB, then build the StorageManager.
pub async fn sync_and_build(
    config_endpoints: &[EndpointConfig],
    secrets: &Secrets,
    pool: StoragePool,
    local_base_dir: &str,
) -> AppResult<StorageManager> {

    // Upsert each config endpoint into DB
    for ep in config_endpoints {
        pool.upsert_endpoint(ep).await?;
    }

    // Disable DB entries that are no longer in config
    let active_names: Vec<String> = config_endpoints.iter().map(|e| e.name.clone()).collect();
    pool.disable_missing_endpoints(&active_names).await?;

    // Load all active endpoints from DB
    let db_rows = pool.load_active_endpoints().await?;

    // Build endpoint entries with backends
    let mut entries: HashMap<String, Arc<EndpointEntry>> = HashMap::new();
    for row in &db_rows {
        // Find the config entry to determine type and path
        let cfg_ep = config_endpoints
            .iter()
            .find(|e| e.name == row.name)
            .ok_or_else(|| {
                AppError::Internal(format!(
                    "active endpoint '{}' not found in config — this is a bug",
                    row.name
                ))
            })?;

        let backend: Arc<dyn StorageBackend> = match cfg_ep.endpoint_type {
            EndpointType::Local => {
                let path = cfg_ep.path.as_deref().unwrap_or(local_base_dir);
                Arc::new(LocalBackend::new(path))
            }
            EndpointType::S3 => {
                let ep_secrets = secrets.endpoints.get(&row.name).ok_or_else(|| {
                    AppError::Internal(format!(
                        "no credentials found for S3 endpoint '{}'",
                        row.name
                    ))
                })?;

                let access_key = ep_secrets.access_key.clone().ok_or_else(|| {
                    AppError::Internal(format!("missing access_key for endpoint '{}'", row.name))
                })?;
                let secret_key = ep_secrets.secret_key.clone().ok_or_else(|| {
                    AppError::Internal(format!("missing secret_key for endpoint '{}'", row.name))
                })?;

                // Bucket name: prefer env var, fall back to config field
                let bucket = ep_secrets
                    .bucket
                    .clone()
                    .or_else(|| cfg_ep.bucket.clone())
                    .ok_or_else(|| {
                        AppError::Internal(format!("missing bucket for endpoint '{}'", row.name))
                    })?;

                let region = ep_secrets
                    .region
                    .clone()
                    .or_else(|| cfg_ep.region.clone())
                    .unwrap_or_else(|| "us-east-1".to_string());

                let mut builder = aws_config::defaults(BehaviorVersion::latest())
                    .region(Region::new(region))
                    .credentials_provider(Credentials::new(
                        access_key,
                        secret_key,
                        None,
                        None,
                        "static",
                    ));

                if let Some(url) = &ep_secrets.endpoint_url {
                    builder = builder.endpoint_url(url);
                }

                let sdk_config = builder.load().await;

                // env overrides config; default to false (virtual-hosted style)
                let force_path_style = ep_secrets
                    .force_path_style
                    .or(cfg_ep.force_path_style)
                    .unwrap_or(false);

                let s3_config = aws_sdk_s3::config::Builder::from(&sdk_config)
                    .force_path_style(force_path_style)
                    .build();
                let client = aws_sdk_s3::Client::from_conf(s3_config);

                // Build presign client with public endpoint override if configured
                let presign_client = cfg_ep.direct_url_public_endpoint.as_ref().map(|public_url| {
                    let presign_s3_config = aws_sdk_s3::config::Builder::from(&sdk_config)
                        .endpoint_url(public_url)
                        .force_path_style(force_path_style)
                        .build();
                    aws_sdk_s3::Client::from_conf(presign_s3_config)
                });

                // Determine the endpoint URL to use for public URL construction
                let effective_endpoint_url = cfg_ep
                    .direct_url_public_endpoint
                    .as_deref()
                    .or(ep_secrets.endpoint_url.as_deref())
                    .map(|s| s.to_string());

                Arc::new(S3Backend::new(client, bucket, presign_client, effective_endpoint_url))
            }
        };

        let endpoint_type_str = match cfg_ep.endpoint_type {
            EndpointType::Local => "Local",
            EndpointType::S3 => "S3",
        }
        .to_string();

        let entry = Arc::new(EndpointEntry {
            name: row.name.clone(),
            priority: row.priority,
            capacity_bytes: row.capacity_bytes,
            endpoint_type: endpoint_type_str,
            status: row.status.clone(),
            used_size: Arc::new(AtomicI64::new(row.used_size)),
            backend,
            direct_mode: cfg_ep.direct_mode.clone(),
            direct_url_ttl: cfg_ep.direct_url_ttl,
        });
        entries.insert(row.name.clone(), entry);
    }

    let router = StorageRouter::build(&entries);

    Ok(StorageManager {
        router: StdRwLock::new(router),
        endpoints: RwLock::new(entries),
        pool,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{dynamic_variant_key, is_dynamic_variant_key, preset_is_permanent};
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
    fn builds_object_key_cache_with_nested_variant_key() {
        use super::object_key_cache;
        let key = object_key_cache(
            &format!("dynamic{}w800", std::path::MAIN_SEPARATOR),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "webp",
        );
        assert!(key.contains("dynamic"));
        assert!(key.contains("w800"));
        assert!(key.ends_with(".webp"));
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
