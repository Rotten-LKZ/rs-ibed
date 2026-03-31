use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use super::repo::ImageRepo;
use crate::error::{AppError, AppResult};
use crate::models::image::{ImageCountResponse, ImageModel, NewImage};

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
             FROM images WHERE hash = ? LIMIT 1",
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
        let result = sqlx::query(
            "UPDATE images SET is_deleted = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND is_deleted = 0",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    async fn restore(&self, id: i64) -> AppResult<()> {
        let result = sqlx::query(
            "UPDATE images SET is_deleted = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND is_deleted = 1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    async fn rename(&self, id: i64, display_name: &str) -> AppResult<()> {
        let result = sqlx::query(
            "UPDATE images SET display_name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND is_deleted = 0",
        )
        .bind(display_name)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound);
        }

        Ok(())
    }

    async fn list_images(
        &self,
        page: i64,
        per_page: i64,
        name: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
        deleted: Option<bool>,
    ) -> AppResult<(Vec<ImageModel>, i64)> {
        let offset = (page - 1) * per_page;

        let mut conditions = vec![match deleted {
            Some(true) => "is_deleted = 1".to_string(),
            Some(false) | None => "is_deleted = 0".to_string(),
        }];
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

    async fn count_images(&self) -> AppResult<ImageCountResponse> {
        let (active, in_trash) = sqlx::query_as::<_, (i64, i64)>(
            "SELECT
                CAST(COUNT(*) FILTER (WHERE is_deleted = 0) AS INTEGER) AS active,
                CAST(COUNT(*) FILTER (WHERE is_deleted = 1) AS INTEGER) AS in_trash
             FROM images",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(ImageCountResponse { active, in_trash })
    }

    async fn hard_delete(&self, id: i64) -> AppResult<()> {
        sqlx::query("DELETE FROM images WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn find_expired_deleted(&self, cutoff: DateTime<Utc>) -> AppResult<Vec<ImageModel>> {
        let rows = sqlx::query_as::<_, ImageModel>(
            "SELECT id, hash, display_name, file_name, extension, mime_type, size, width, height, user_id, is_deleted, created_at, updated_at
             FROM images WHERE is_deleted = 1 AND updated_at < ?",
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn find_all_deleted(&self) -> AppResult<Vec<ImageModel>> {
        let rows = sqlx::query_as::<_, ImageModel>(
            "SELECT id, hash, display_name, file_name, extension, mime_type, size, width, height, user_id, is_deleted, created_at, updated_at
             FROM images WHERE is_deleted = 1",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn delete_all_deleted(&self) -> AppResult<()> {
        sqlx::query("DELETE FROM images WHERE is_deleted = 1")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;
    use crate::error::AppError;

    async fn setup_repo() -> SqliteImageRepo {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("connect sqlite test db");

        sqlx::migrate!("./migrations/sqlite")
            .run(&pool)
            .await
            .expect("run sqlite migrations");

        SqliteImageRepo::new(pool)
    }

    fn new_image(hash: &str, display_name: &str) -> NewImage {
        NewImage {
            hash: hash.to_string(),
            display_name: display_name.to_string(),
            file_name: format!("{hash}.png"),
            extension: "png".to_string(),
            mime_type: "image/png".to_string(),
            size: 123,
            width: 10,
            height: 20,
            user_id: Some("tester".to_string()),
        }
    }

    #[tokio::test]
    async fn rename_returns_not_found_for_deleted_and_missing_images() {
        let repo = setup_repo().await;
        let image = repo
            .insert_image(&new_image("sqlite-rename", "before"))
            .await
            .expect("insert image");

        repo.rename(image.id, "after").await.expect("rename active image");
        assert_eq!(
            repo.find_by_id(image.id)
                .await
                .expect("find renamed image")
                .expect("renamed image exists")
                .display_name,
            "after"
        );

        repo.soft_delete(image.id).await.expect("soft delete image");

        let deleted_err = repo
            .rename(image.id, "should fail")
            .await
            .expect_err("renaming deleted image should fail");
        assert!(matches!(deleted_err, AppError::NotFound));

        let missing_err = repo
            .rename(image.id + 1, "missing")
            .await
            .expect_err("renaming missing image should fail");
        assert!(matches!(missing_err, AppError::NotFound));
    }

    #[tokio::test]
    async fn soft_delete_and_restore_enforce_deletion_state() {
        let repo = setup_repo().await;
        let image = repo
            .insert_image(&new_image("sqlite-restore", "restorable"))
            .await
            .expect("insert image");

        let restore_err = repo
            .restore(image.id)
            .await
            .expect_err("restoring active image should fail");
        assert!(matches!(restore_err, AppError::NotFound));

        repo.soft_delete(image.id).await.expect("soft delete active image");

        let second_delete_err = repo
            .soft_delete(image.id)
            .await
            .expect_err("deleting trashed image should fail");
        assert!(matches!(second_delete_err, AppError::NotFound));

        let (trash_items, trash_total) = repo
            .list_images(1, 10, None, None, None, Some(true))
            .await
            .expect("list trash");
        assert_eq!(trash_total, 1);
        assert_eq!(trash_items.len(), 1);
        assert_eq!(trash_items[0].id, image.id);
        assert!(trash_items[0].is_deleted);

        let counts = repo.count_images().await.expect("count images");
        assert_eq!(counts.active, 0);
        assert_eq!(counts.in_trash, 1);

        repo.restore(image.id).await.expect("restore trashed image");

        let second_restore_err = repo
            .restore(image.id)
            .await
            .expect_err("restoring active image again should fail");
        assert!(matches!(second_restore_err, AppError::NotFound));

        let active = repo
            .find_by_id(image.id)
            .await
            .expect("find restored image")
            .expect("restored image exists");
        assert!(!active.is_deleted);

        let (active_items, active_total) = repo
            .list_images(1, 10, None, None, None, Some(false))
            .await
            .expect("list active images");
        assert_eq!(active_total, 1);
        assert_eq!(active_items.len(), 1);
        assert_eq!(active_items[0].id, image.id);
        assert!(!active_items[0].is_deleted);

        let counts = repo.count_images().await.expect("count images after restore");
        assert_eq!(counts.active, 1);
        assert_eq!(counts.in_trash, 0);
    }
}
