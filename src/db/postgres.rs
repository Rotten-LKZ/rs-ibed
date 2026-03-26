use async_trait::async_trait;
use sqlx::PgPool;

use super::repo::ImageRepo;
use crate::error::AppResult;
use crate::models::image::{ImageModel, NewImage};

pub struct PgImageRepo {
    pool: PgPool,
}

impl PgImageRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ImageRepo for PgImageRepo {
    async fn insert_image(&self, img: &NewImage) -> AppResult<ImageModel> {
        let row = sqlx::query_as::<_, ImageModel>(
            "INSERT INTO images (hash, display_name, file_name, extension, mime_type, size, width, height, user_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
             FROM images WHERE hash = $1 AND is_deleted = FALSE LIMIT 1",
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn find_by_id(&self, id: i64) -> AppResult<Option<ImageModel>> {
        let row = sqlx::query_as::<_, ImageModel>(
            "SELECT id, hash, display_name, file_name, extension, mime_type, size, width, height, user_id, is_deleted, created_at, updated_at
             FROM images WHERE id = $1 LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn soft_delete(&self, id: i64) -> AppResult<()> {
        sqlx::query("UPDATE images SET is_deleted = TRUE, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn restore(&self, id: i64) -> AppResult<()> {
        sqlx::query("UPDATE images SET is_deleted = FALSE, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn rename(&self, id: i64, display_name: &str) -> AppResult<()> {
        sqlx::query("UPDATE images SET display_name = $1, updated_at = NOW() WHERE id = $2")
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

        let mut conditions = vec!["is_deleted = FALSE".to_string()];
        let name_pattern = name.map(|n| format!("%{n}%"));
        let mut param_idx: u32 = 1;

        let name_idx;
        if name.is_some() {
            name_idx = param_idx;
            conditions.push(format!("display_name LIKE ${param_idx}"));
            param_idx += 1;
        } else {
            name_idx = 0;
        }

        let date_from_idx;
        if date_from.is_some() {
            date_from_idx = param_idx;
            conditions.push(format!("created_at >= ${param_idx}::timestamptz"));
            param_idx += 1;
        } else {
            date_from_idx = 0;
        }

        let date_to_idx;
        if date_to.is_some() {
            date_to_idx = param_idx;
            conditions.push(format!(
                "created_at < (${param_idx}::date + interval '1 day')"
            ));
            param_idx += 1;
        } else {
            date_to_idx = 0;
        }

        let where_clause = conditions.join(" AND ");

        let count_sql = format!("SELECT COUNT(*)::bigint FROM images WHERE {where_clause}");
        let list_sql = format!(
            "SELECT id, hash, display_name, file_name, extension, mime_type, size, width, height, user_id, is_deleted, created_at, updated_at
             FROM images WHERE {where_clause}
             ORDER BY created_at DESC
             LIMIT ${param_idx} OFFSET ${}",
            param_idx + 1
        );

        // Count query
        let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref p) = name_pattern {
            let _ = name_idx;
            count_q = count_q.bind(p);
        }
        if let Some(df) = date_from {
            let _ = date_from_idx;
            count_q = count_q.bind(df);
        }
        if let Some(dt) = date_to {
            let _ = date_to_idx;
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
