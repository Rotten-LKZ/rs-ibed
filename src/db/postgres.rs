use async_trait::async_trait;
use sqlx::PgPool;

use super::repo::ImageRepo;
use crate::error::{AppError, AppResult};
use crate::models::image::{ImageCountResponse, ImageModel, NewImage};

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
        let result = sqlx::query(
            "UPDATE images SET is_deleted = TRUE, updated_at = NOW() WHERE id = $1 AND is_deleted = FALSE",
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
            "UPDATE images SET is_deleted = FALSE, updated_at = NOW() WHERE id = $1 AND is_deleted = TRUE",
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
            "UPDATE images SET display_name = $1, updated_at = NOW() WHERE id = $2 AND is_deleted = FALSE",
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
            Some(true) => "is_deleted = TRUE".to_string(),
            Some(false) | None => "is_deleted = FALSE".to_string(),
        }];
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

    async fn count_images(&self) -> AppResult<ImageCountResponse> {
        let (active, in_trash) = sqlx::query_as::<_, (i64, i64)>(
            "SELECT
                COUNT(*) FILTER (WHERE is_deleted = FALSE)::bigint AS active,
                COUNT(*) FILTER (WHERE is_deleted = TRUE)::bigint AS in_trash
             FROM images",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(ImageCountResponse { active, in_trash })
    }
}

#[cfg(test)]
mod tests {
    use sqlx::postgres::PgPoolOptions;

    use super::*;
    use crate::error::AppError;

    async fn setup_repo() -> Option<PgImageRepo> {
        let database_url = match std::env::var("IMG_DATABASE_URL") {
            Ok(url) if url.starts_with("postgres://") || url.starts_with("postgresql://") => url,
            _ => return None,
        };

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("connect postgres test db");

        sqlx::migrate!("./migrations/pgsql")
            .run(&pool)
            .await
            .expect("run postgres migrations");

        sqlx::query("TRUNCATE TABLE images RESTART IDENTITY")
            .execute(&pool)
            .await
            .expect("truncate images table");

        Some(PgImageRepo::new(pool))
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
        let Some(repo) = setup_repo().await else {
            return;
        };

        let image = repo
            .insert_image(&new_image("pg-rename", "before"))
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
        let Some(repo) = setup_repo().await else {
            return;
        };

        let image = repo
            .insert_image(&new_image("pg-restore", "restorable"))
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
