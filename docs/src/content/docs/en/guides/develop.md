---
title: Development Guide
description: Local development notes for the RS-IBED backend, frontend, and documentation site. Includes storage architecture and distributed storage internals.
---

This page explains how to work on RS-IBED locally, including the backend, frontend, OpenAPI workflow, and docs site. It also covers the distributed storage architecture for developers who want to understand or extend the storage layer.

## Repository structure

This project is a monorepo with the following main directories:

```text
.
├── docs
├── frontend
├── migrations
├── src
├── Cargo.toml
├── config.toml
└── openapi.json
```

- `src/`: Rust backend source code
- `frontend/`: SvelteKit admin frontend
- `docs/`: Astro Starlight documentation site
- `migrations/`: database migration files
- `openapi.json`: exported OpenAPI specification

## Prerequisites

You should have the following tools installed:

- Rust and Cargo
- `pnpm`
- SQLite or PostgreSQL, depending on your database choice

You also need runtime environment variables:

```bash
export IMG_AUTH_TOKEN="dev-token"
export IMG_JWT_SECRET="dev-secret"
export IMG_DATABASE_URL="sqlite://data.db"
```

For S3 storage endpoints, you also need:

```bash
export IMG_ENDPOINT_MINIO1__ENDPOINT_URL="http://localhost:9000"
export IMG_ENDPOINT_MINIO1__ACCESS_KEY="minioadmin"
export IMG_ENDPOINT_MINIO1__SECRET_KEY="minioadmin"
export IMG_ENDPOINT_MINIO1__BUCKET="ibed-images"
```

## Backend development

### Common commands

```bash
cargo build
cargo test
cargo run
```

To start with a specific config file:

```bash
cargo run -- --config ./config.toml
```

### Export OpenAPI

You can export the OpenAPI schema without starting the server:

```bash
cargo run -- export-openapi
cargo run -- export-openapi frontend/openapi.json
```

By default, the schema is written to `openapi.json` in the repository root.

### Additional binaries

You can add independent binaries under `src/bin/`. For example:

- `src/bin/upload.rs`: CLI uploader for the upload API

After building or packaging, this produces a standalone `upload` executable that can be run directly:

```bash
./upload --help
```

## Storage Architecture (Technical Deep Dive)

RS-IBED uses a **consistent hash ring** with **virtual nodes** for distributed storage. This section explains how it works for developers who want to understand or modify the storage layer.

### Core Components

```text
src/
├── storage/
│   └── mod.rs           # StorageManager, StorageRouter, EndpointEntry
├── storage_backend/
│   ├── mod.rs           # StorageBackend trait
│   ├── local.rs         # LocalBackend implementation
│   └── s3.rs            # S3Backend implementation
└── models/
    └── storage_endpoint.rs  # StorageEndpointModel, DTOs
```

### Consistent Hash Ring

The router builds a hash ring where:

1. **Each endpoint** is mapped to multiple **virtual nodes** on the ring
2. **Virtual node count** is proportional to `capacity_bytes` (100 virtual nodes per GB)
3. **File placement** is determined by hashing the content hash to a position on the ring
4. **Clockwise probing** finds the next available endpoint for writes
5. **Multi-tier priority** allows capacity overflow to fall through to lower-priority endpoints

```rust
// From src/storage/mod.rs
// Virtual node count calculation
let node_count = (entry.capacity_bytes / 1_073_741_824).max(1) * 100;
```

### Write Flow

```
1. Hash content → get 32-bit position
2. Start at position, walk clockwise on priority tier
3. Check capacity: used_size + file_size ≤ capacity_bytes
4. If full, try next virtual node (same priority)
5. If all full in this priority, fall through to next priority tier
6. Write to selected endpoint
7. Update used_size in memory and DB
```

### Read Flow

```
1. Hash content → get 32-bit position
2. Collect all active endpoints in clockwise order (deduplicated)
3. For each endpoint:
   - HEAD check for cached variant
   - If exists and direct_mode ≠ Proxy → redirect/presigned URL
   - Else → proxy the bytes
4. If cache miss → read original from any endpoint
5. Process on-the-fly and optionally cache result
```

### Node Addition/Removal Behavior

| Scenario | Effect | Data Location |
|----------|--------|---------------|
| **Add node** | New virtual nodes take over adjacent hash ranges | Existing files stay on original nodes; new files may map to new node |
| **Remove node** | Virtual nodes removed; adjacent nodes take over range | Existing files remain on removed node; reads probe all nodes until found |
| **Disable node** | Status changed to "disabled"; virtual nodes excluded from write routing | Files remain; not used for new writes |

**Key property**: Only files whose hash falls in the adjacent ranges are affected. Most files continue to map to their original nodes.

### File Storage Layout

Files are stored with a content-addressed layout:

```
origin/{hash[0:2]}/{hash[2:4]}/{hash}.{ext}
cache/{variant}/{hash[0:2]}/{hash[2:4]}/{hash}.{ext}
```

Example:
```
origin/ab/cd/abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890.jpg
cache/thumb/ab/cd/abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890.webp
```

### Key Data Structures

```rust
// Router structure
pub struct StorageRouter {
    /// priority → ring (virtual_node_u32 → endpoint_name)
    rings: BTreeMap<i32, BTreeMap<u32, String>>,
    endpoints: HashMap<String, Arc<EndpointEntry>>,
}

// Endpoint entry
pub struct EndpointEntry {
    pub name: String,
    pub priority: i32,
    pub capacity_bytes: i64,
    pub status: String,
    pub used_size: Arc<AtomicI64>,
    pub backend: Arc<dyn StorageBackend>,
    pub direct_mode: DirectMode,
    // ...
}
```

### Admin API Endpoints

Storage endpoints can be managed via the admin API:

- `GET /api/admin/storage/endpoints` - List all endpoints with live stats
- `POST /api/admin/storage/endpoints/{name}/update` - Update description/status

Mutable fields: `description`, `status` (`active`/`disabled`)
Config-immutable fields: `priority`, `capacity_bytes` (require restart to change)

### Storage Backend Trait

To add a new storage backend, implement the `StorageBackend` trait:

```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn put_object(&self, key: &str, data: Vec<u8>) -> AppResult<()>;
    async fn get_object(&self, key: &str) -> AppResult<Option<Vec<u8>>>;
    async fn head_object(&self, key: &str) -> AppResult<Option<ObjectMeta>>;
    async fn delete_object(&self, key: &str) -> AppResult<()>;
    async fn total_used_bytes(&self) -> AppResult<i64>;
    async fn presigned_get_url(&self, key: &str, expires_in: Duration) -> AppResult<Option<String>>;
    fn public_url(&self, key: &str) -> Option<String>;
}
```

See `src/storage_backend/local.rs` and `src/storage_backend/s3.rs` for reference implementations.

## Frontend development

The frontend lives in `frontend/` and uses SvelteKit.

### Common commands

Run these inside `frontend/`:

```bash
pnpm install
pnpm gen:api
pnpm check
pnpm build
pnpm dev
```

### Recommended integration order

1. Export OpenAPI from the backend:

```bash
cargo run -- export-openapi frontend/openapi.json
```

2. Generate the frontend SDK:

```bash
cd frontend
pnpm gen:api
```

3. Start the backend:

```bash
cargo run
```

4. Start the frontend dev server:

```bash
cd frontend
pnpm dev
```

### Authentication flow

The project includes a CLI-to-browser login flow:

- When you run `cargo run`, the server prints a CLI login URL
- That link targets `/login?token=...`
- Opening it in the browser makes the frontend call `/api/auth/cli`
- The backend then sets the `ibed_token` cookie to establish a browser session

Protected endpoints also support direct Bearer token access, for example:

```text
Authorization: Bearer <AUTH_TOKEN>
```

## Documentation development

The docs site is located in `docs/` and uses Astro Starlight.

### Common commands

Run these inside `docs/`:

```bash
pnpm install
pnpm dev
pnpm build
pnpm preview
```

### Localized documentation

- `docs/src/content/docs/zh/`: Chinese documentation
- `docs/src/content/docs/en/`: English documentation

If you add a new Chinese page, you should usually add the matching English page too so navigation stays aligned across locales.

### API documentation source

The docs site uses `starlight-openapi` and reads the repository root `openapi.json`.

So after backend API changes, the usual flow is:

1. Re-export `openapi.json`
2. Sync `frontend/openapi.json` if needed
3. Then start or rebuild the docs site and frontend

## Debugging tips

### Upload API debugging

You can debug uploads with:

- `curl`
- `cargo run --bin upload -- ...`
- The browser network panel

### Storage layer debugging

Enable debug logging to see storage routing decisions:

```bash
RUST_LOG=debug cargo run
```

Key log messages:
- `reconciled used_size` - Periodic reconciliation of endpoint usage
- `eager preset generated` - Background cache generation
- `storage router lock poisoned` - Router panic (should never happen)

### Useful files to inspect

When working on upload, resizing, or image delivery, these files are usually the most relevant:

- `src/handlers/upload.rs` - Upload handler
- `src/handlers/view.rs` - View/download handler with cache logic
- `src/storage/mod.rs` - StorageManager and StorageRouter
- `src/storage_backend/` - Storage backend implementations
- `src/router.rs` - HTTP routing
- `src/config.rs` - Configuration structures
- `src/auth.rs` - Authentication

When working on storage endpoints:

- `src/storage/mod.rs` - Core routing logic
- `src/handlers/admin.rs` - Admin API for endpoint management
- `src/models/storage_endpoint.rs` - Data models

### Common things to verify

If an API request fails, check:

- whether `IMG_AUTH_TOKEN` is correct
- whether `IMG_DATABASE_URL` is reachable
- whether `url_pattern` in `config.toml` matches the paths you are testing
- whether `image.allow_show_origin` affects original file access
- whether OpenAPI has been re-exported after backend changes
- whether S3 credentials are set correctly for S3 endpoints
- whether `force_path_style` is set correctly for MinIO

## Before submitting changes

Before submitting changes, it is a good idea to run at least:

```bash
cargo test
cargo build
```

If you changed the frontend:

```bash
cd frontend
pnpm check
pnpm build
```

If you changed the documentation:

```bash
cd docs
pnpm build
```

## Related pages

- [Getting Started](/en/guides/getting-started/)
- [Configuration File](/en/guides/config/)
