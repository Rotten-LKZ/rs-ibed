use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::error::AppResult;
use crate::models::image::{ImageCountResponse, ImageModel, NewImage};

#[async_trait]
pub trait ImageRepo: Send + Sync {
    async fn insert_image(&self, img: &NewImage) -> AppResult<ImageModel>;
    async fn find_by_hash(&self, hash: &str) -> AppResult<Option<ImageModel>>;
    async fn find_by_id(&self, id: i64) -> AppResult<Option<ImageModel>>;
    async fn soft_delete(&self, id: i64) -> AppResult<()>;
    async fn restore(&self, id: i64) -> AppResult<()>;
    async fn rename(&self, id: i64, display_name: &str) -> AppResult<()>;

    /// Paginated list with optional filters.
    /// `name`: LIKE search on display_name.
    /// `date_from` / `date_to`: filter on created_at.
    async fn list_images(
        &self,
        page: i64,
        per_page: i64,
        name: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
        deleted: Option<bool>,
    ) -> AppResult<(Vec<ImageModel>, i64)>;

    async fn count_images(&self) -> AppResult<ImageCountResponse>;

    async fn hard_delete(&self, id: i64) -> AppResult<()>;
    async fn find_expired_deleted(&self, cutoff: DateTime<Utc>) -> AppResult<Vec<ImageModel>>;
    async fn find_all_deleted(&self) -> AppResult<Vec<ImageModel>>;
    async fn delete_all_deleted(&self) -> AppResult<()>;
}
