pub mod local;
pub mod s3;

use std::time::Duration;

use async_trait::async_trait;
use crate::error::AppResult;

pub struct ObjectMeta {
    pub size: i64,
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn put_object(&self, key: &str, data: Vec<u8>) -> AppResult<()>;
    async fn get_object(&self, key: &str) -> AppResult<Option<Vec<u8>>>;
    async fn head_object(&self, key: &str) -> AppResult<Option<ObjectMeta>>;
    async fn delete_object(&self, key: &str) -> AppResult<()>;
    /// Sum the size of all stored objects. Used for periodic reconciliation of `used_size`.
    async fn total_used_bytes(&self) -> AppResult<i64>;

    /// Generate a presigned GET URL for the given key, valid for `expires_in`.
    /// Returns `None` if the backend does not support presigned URLs (e.g., local filesystem).
    async fn presigned_get_url(
        &self,
        _key: &str,
        _expires_in: Duration,
    ) -> AppResult<Option<String>> {
        Ok(None)
    }

    /// Construct a direct public URL for the given key.
    /// Returns `None` if the backend does not support public URLs.
    fn public_url(&self, _key: &str) -> Option<String> {
        None
    }
}
