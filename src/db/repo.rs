use async_trait::async_trait;

use crate::error::AppResult;
use crate::models::image::{ImageModel, NewImage};

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
    ) -> AppResult<(Vec<ImageModel>, i64)>;
}
