use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::config::DirectMode;

/// DB row — flat storage endpoint (no role flags).
#[derive(Debug, Clone, FromRow)]
pub struct StorageEndpointModel {
    pub name: String,
    pub capacity_bytes: i64,
    pub used_size: i64,
    pub priority: i32,
    pub status: String,
}

/// Response returned by GET /api/admin/storage/endpoints —
/// merges live in-memory state with config-immutable fields.
#[derive(Debug, Serialize, ToSchema)]
pub struct StorageEndpointResponse {
    pub name: String,
    pub description: String,
    /// Backend type from config: "Local" or "S3".
    pub endpoint_type: String,
    pub capacity_bytes: i64,
    pub used_size: i64,
    pub priority: i32,
    pub status: String,
    /// How clients access images: "proxy", "presigned", or "public".
    pub direct_mode: DirectMode,
}

/// Only `status` and `description` are mutable via the API.
/// `priority` and `capacity_bytes` are config-immutable.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateEndpointRequest {
    pub description: Option<String>,
    pub status: Option<String>,
}
