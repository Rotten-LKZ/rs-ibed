pub mod repo;
pub mod sqlite;
pub mod postgres;

use std::sync::Arc;
use crate::config::{AppConfig, DatabaseDriver, Secrets};
use crate::error::{AppError, AppResult};
use repo::ImageRepo;

pub async fn init_repo(config: &AppConfig, secrets: &Secrets) -> AppResult<Arc<dyn ImageRepo>> {
    match config.database.driver {
        DatabaseDriver::Sqlite => {
            let opts = secrets.database_url
                .parse::<sqlx::sqlite::SqliteConnectOptions>()
                .map_err(|e| AppError::Internal(e.to_string()))?
                .create_if_missing(true);

            // create_if_missing only creates the .db file, not parent dirs
            if let Some(parent) = opts.get_filename().parent() {
                if !parent.as_os_str().is_empty() {
                    tokio::fs::create_dir_all(parent).await?;
                }
            }

            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(config.database.max_connections)
                .min_connections(config.database.min_connections)
                .connect_with(opts)
                .await?;

            sqlx::migrate!("./migrations/sqlite")
                .run(&pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            Ok::<Arc<dyn ImageRepo>, AppError>(Arc::new(sqlite::SqliteImageRepo::new(pool)))
        }
        DatabaseDriver::Postgres => {
            ensure_pg_database(&secrets.database_url).await?;

            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(config.database.max_connections)
                .min_connections(config.database.min_connections)
                .connect(&secrets.database_url)
                .await?;

            sqlx::migrate!("./migrations/pgsql")
                .run(&pool)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            Ok::<Arc<dyn ImageRepo>, AppError>(Arc::new(postgres::PgImageRepo::new(pool)))
        }
    }
}

/// Parse dbname in URL and connect to maintenance db; if target db doesn't exist, create it; ignore "database already exists" error.
async fn ensure_pg_database(url: &str) -> AppResult<()> {
    use sqlx::postgres::PgConnectOptions;
    use sqlx::ConnectOptions;

    let opts: PgConnectOptions = url
        .parse()
        .map_err(|e: sqlx::Error| AppError::Internal(e.to_string()))?;

    let db_name = opts
        .get_database()
        .unwrap_or("postgres")
        .to_owned();

    // Default database in PostgreSQL
    if db_name == "postgres" {
        return Ok(());
    }

    // connect to maintenance db to create target db if it doesn't exist
    let maintenance = opts.clone().database("postgres");
    let mut conn = maintenance
        .connect()
        .await
        .map_err(|e| AppError::Internal(format!("cannot connect to maintenance db: {e}")))?;


    let sql = format!("CREATE DATABASE \"{}\"", db_name);
    match sqlx::query(&sql).execute(&mut conn).await {
        Ok(_) => {
            tracing::info!(db = %db_name, "created database");
        }
        Err(e) => {
            // 42P04 = duplicate_database
            if let sqlx::Error::Database(ref dbe) = e {
                if dbe.code().as_deref() == Some("42P04") {
                    tracing::debug!(db = %db_name, "database already exists");
                    return Ok(());
                }
            }
            return Err(AppError::Internal(format!("failed to create database: {e}")));
        }
    }

    Ok(())
}
