use async_trait::async_trait;
use sqlx::SqlitePool;

use super::repo::ImageRepo;
use crate::error::AppResult;
use crate::models::image::{ImageModel, NewImage};

pub struct SqliteImageRepo {
    pool: SqlitePool,
}

impl SqliteImageRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ImageRepo for SqliteImageRepo {
    async fn insert_image(&self, img: &NewImage) -> AppResult<ImageModel> {
        let row = sqlx::query_as::<_, ImageModel>(
            "INSERT INTO images (hash, display_name, file_name, extension, mime_type, size, width, height, user_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id, hash, display_name, file_name, extension, mime_type, size, width, height, user_id, is_deleted, created_at, updated_at",
        )
        .bind(&img.hash)
        .bind(&img.display_name)
        .bind(&img.file_name)
        .bind(&img.extension)
        .bind(&img.mime_type)
        .bind(img.size)
        .bind(img.width)
        .bind(img.height)
        .bind(&img.user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn find_by_hash(&self, hash: &str) -> AppResult<Option<ImageModel>> {
        let row = sqlx::query_as::<_, ImageModel>(
            "SELECT id, hash, display_name, file_name, extension, mime_type, size, width, height, user_id, is_deleted, created_at, updated_at
             FROM images WHERE hash = ? AND is_deleted = 0 LIMIT 1",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn find_by_id(&self, id: i64) -> AppResult<Option<ImageModel>> {
        let row = sqlx::query_as::<_, ImageModel>(
            "SELECT id, hash, display_name, file_name, extension, mime_type, size, width, height, user_id, is_deleted, created_at, updated_at
             FROM images WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn soft_delete(&self, id: i64) -> AppResult<()> {
        sqlx::query(
            "UPDATE images SET is_deleted = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn restore(&self, id: i64) -> AppResult<()> {
        sqlx::query(
            "UPDATE images SET is_deleted = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn rename(&self, id: i64, display_name: &str) -> AppResult<()> {
        sqlx::query(
            "UPDATE images SET display_name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(display_name)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_images(
        &self,
        page: i64,
        per_page: i64,
        name: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
    ) -> AppResult<(Vec<ImageModel>, i64)> {
        let offset = (page - 1) * per_page;

        // Build WHERE conditions dynamically.
        // We always include is_deleted = 0.
        let mut conditions = vec!["is_deleted = 0".to_string()];
        let name_pattern = name.map(|n| format!("%{n}%"));

        if name.is_some() {
            conditions.push("display_name LIKE ?".to_string());
        }
        if date_from.is_some() {
            conditions.push("created_at >= ?".to_string());
        }
        if date_to.is_some() {
            conditions.push("created_at < datetime(?, '+1 day')".to_string());
        }

        let where_clause = conditions.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) FROM images WHERE {where_clause}");
        let list_sql = format!(
            "SELECT id, hash, display_name, file_name, extension, mime_type, size, width, height, user_id, is_deleted, created_at, updated_at
             FROM images WHERE {where_clause}
             ORDER BY created_at DESC
             LIMIT ? OFFSET ?"
        );

        // Count query
        let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref p) = name_pattern {
            count_q = count_q.bind(p);
        }
        if let Some(df) = date_from {
            count_q = count_q.bind(df);
        }
        if let Some(dt) = date_to {
            count_q = count_q.bind(dt);
        }
        let total = count_q.fetch_one(&self.pool).await?;

        // List query
        let mut list_q = sqlx::query_as::<_, ImageModel>(&list_sql);
        if let Some(ref p) = name_pattern {
            list_q = list_q.bind(p);
        }
        if let Some(df) = date_from {
            list_q = list_q.bind(df);
        }
        if let Some(dt) = date_to {
            list_q = list_q.bind(dt);
        }
        list_q = list_q.bind(per_page).bind(offset);

        let items = list_q.fetch_all(&self.pool).await?;

        Ok((items, total))
    }
}
