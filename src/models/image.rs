use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, ToSchema)]
pub struct ImageModel {
    pub id: i64,
    pub hash: String,
    pub display_name: String,
    pub file_name: String,
    pub extension: String,
    pub mime_type: String,
    pub size: i64,
    pub width: i32,
    pub height: i32,
    pub user_id: Option<String>,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct NewImage {
    pub hash: String,
    pub display_name: String,
    pub file_name: String,
    pub extension: String,
    pub mime_type: String,
    pub size: i64,
    pub width: i32,
    pub height: i32,
    pub user_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct ImageListQuery {
    /// Page number, starting from 1
    #[serde(default = "default_page")]
    pub page: i64,
    /// Items per page
    #[serde(default = "default_per_page")]
    pub per_page: i64,
    /// Fuzzy search by display_name
    pub name: Option<String>,
    /// Start date (YYYY-MM-DD)
    pub date_from: Option<String>,
    /// End date (YYYY-MM-DD)
    pub date_to: Option<String>,
}

fn default_page() -> i64 {
    1
}
fn default_per_page() -> i64 {
    20
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ImageListResponse {
    pub items: Vec<ImageModel>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ImageDetailResponse {
    pub image: ImageModel,
    pub view_url: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct OkResponse {
    pub ok: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RenameRequest {
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UploadResponse {
    pub id: i64,
    pub hash: String,
    pub url: String,
    pub file_name: String,
    pub mime_type: String,
    pub size: i64,
    pub width: i32,
    pub height: i32,
}
