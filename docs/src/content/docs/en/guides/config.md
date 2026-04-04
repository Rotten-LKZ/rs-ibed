---
title: Configuration File
description: How to configure the application using the configuration file, including distributed storage endpoints.
---

Place the `config.toml` file in the project's root directory, or specify its location using the `./rs-ibed --config <path/to/config_file.toml>` command.

## Configuration Example

```toml
[server]
host = "0.0.0.0"
port = 3000
log_level = "info"
env_prefix = "IMG"         # Environment variable prefix, e.g., IMG_AUTH_TOKEN, IMG_DATABASE_URL
# URL path pattern, all accesses are prefixed with /v/
# "ymd"  → /v/2025/03/19/{hash}.{ext}
# "ym"   → /v/2025/03/{hash}.{ext}
# "y"    → /v/2025/{hash}.{ext}
# "flat" → /v/{hash}.{ext}
# preset and download paths are automatically derived:
#   /v/thumb/2025/03/19/{hash}.{ext}
#   /v/preview/2025/03/19/{hash}.{ext}
# Changing 'v' to 'd' in the path will return the original image. Whether authentication is required depends on whether 'image.allow_show_origin' is true.
url_pattern = "ymd"
cors_allow_origins = ["*"]       # Allowed CORS origins, * means all
cors_max_age = 3600              # Cache time for preflight requests (seconds)
enable_negotiated_cache = true   # Whether to enable negotiated caching, implementing 304 Not Modified with both etag and last_modified
cache_max_age = 3600             # Maximum validity for negotiated cache (seconds). Setting to 0 is equivalent to no-cache.
trash_retention_days = 30        # Automatically permanently delete trashed images after this many days. 0 = disabled.
# strict_health_check = false    # When enabled, returns 503 (storage unavailable) instead of 404 when image metadata exists but
                                 # file is missing on all storage nodes. Helps diagnose storage node failures vs truly missing files.

[database]
driver = "sqlite"          # "sqlite" | "postgres"
max_connections = 5
min_connections = 1

# ------------------------------------------
# Storage Endpoints (Distributed Storage)
# ------------------------------------------
# RS-IBED uses a consistent hash ring to distribute files across multiple storage
# endpoints. This allows horizontal scaling and high availability.
#
# Key features:
#   - Consistent hashing: Adding/removing nodes only affects adjacent hash ranges
#   - Weighted by capacity: Larger capacity endpoints get more virtual nodes
#   - Priority tiers: Lower priority = higher preference; falls through when full
#   - Multi-backend: Supports both local filesystem and S3-compatible storage
#   - Automatic failover: Read probes all nodes in ring order until file found

[storage]
# max_payload_bytes = 52428800   # Optional: enforce maximum upload size (bytes). Returns 413 if exceeded.

# Define one or more storage endpoints. Each endpoint can be Local or S3.
[[storage.endpoints]]
name           = "local-primary"
description    = "Primary local filesystem"
type           = "Local"           # "Local" or "S3"
path           = "./data/storage"  # Required for Local type
capacity_bytes = 10737418240       # 10 GB - used for ring weighting
priority       = 1                 # Lower = higher priority tier

[[storage.endpoints]]
name           = "minio-node1"
description    = "MinIO S3 node 1"
type           = "S3"
bucket         = "ibed-images"
region         = "us-east-1"
force_path_style = true            # Required for MinIO and self-hosted S3
direct_mode    = "presigned"       # "proxy" | "presigned" | "public"
direct_url_ttl = 3600              # Presigned URL TTL in seconds
capacity_bytes = 21474836480       # 20 GB
priority       = 1

[image]
enable = true              # Whether to enable image processing. If disabled, only metadata fields specified in keep_metadata_fields will be kept and the original image stored; access will return the original image directly. If enabled, EXIF metadata won't be processed during upload; processing will happen during access according to preset dimensions.
default_format = "webp"    # "webp" | "avif" | "original"
quality = 75               # Transcoding quality, 1-100
max_workers = 2
allow_show_origin = true   # Whether to allow access to the original image (containing all EXIF info)
cache_ttl = 25200          # Default cache time (seconds) for all preset sizes and dynamically generated images. If cache_ttl is set individually in a preset, it takes precedence. Setting to 0 means permanent cache.
# "camera" | "settings" | "time" | "copyright" | "location" | "others"
keep_metadata_fields = ["copyright", "settings", "time"]

[image.dynamic]
# Whether to allow dynamic parameters. If not allowed, max_* configurations are ignored.
# If not allowed, dynamic access will return 403 Forbidden unless it matches a preset.
allow = false
allow_enlargement = false  # Whether to allow enlarging images (only works when allow = true)
max_width = 3840
max_height = 2160

# ------------------------------------------
# Presets
# ------------------------------------------
# Scaling Rules:
#   - width or height only  → Proportional scaling, 'fit' is ignored
#   - both width + height   → Scale according to 'fit' strategy (defaults to "contain")
#   - neither               → Transcode/compress only, dimensions unchanged
#
# fit values:
#   "cover"   - Crop & Fill: Scale to fully cover target dimensions, center-crop excess
#   "contain" - Fit & Contain: Scale to fit within target dimensions, maintain ratio (default)
#   "fill"    - Stretch & Fill: Force stretch to target dimensions, may distort
#   "inside"  - Same as contain, but won't enlarge images smaller than target
#   "outside" - Same as cover, but won't shrink images larger than target

# Special preset "default" is the default for all unmatched requests. If not set, global [image] defaults are used.
[image.presets.default]
# Transcode/compress only, no resizing
# quality/format omitted → inherit from [image] global defaults

[image.presets.thumb]
width = 200
height = 200
fit = "cover"
quality = 60               # Thumbnails can be compressed more aggressively
eager = true               # Generate immediately upon upload. If true, cache_ttl is ignored and it's cached permanently.

[image.presets.preview]
width = 1200
quality = 80               # Slightly higher quality for preview
# only width provided, auto proportional scaling
# eager omitted → false, generated on first access

[image.presets.legacy]
format = "jpeg"            # Override global format for compatibility with old devices not supporting webp
quality = 70
```

## Configuration Details

### [server] Server Configuration

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `host` | String | `"0.0.0.0"` | Listening address |
| `port` | u16 | `3000` | Listening port |
| `log_level` | Enum | `"info"` | Log level: `trace`, `debug`, `info`, `warn`, `error` |
| `env_prefix` | String | `"IMG"` | Environment variable prefix, used for loading sensitive configs (DB URL, Tokens, etc.) |
| `url_pattern` | Enum | `"ymd"` | URL generation pattern: `ymd` (Year/Month/Day), `ym` (Year/Month), `y` (Year), `flat` (No hierarchy) |
| `cors_allow_origins` | Array | `["*"]` | List of allowed CORS origins |
| `cors_max_age` | u64 | `3600` | CORS preflight request cache time (seconds) |
| `enable_negotiated_cache` | bool | `true` | Whether to enable negotiated caching (Etag / Last-Modified) |
| `cache_max_age` | u64 | `3600` | Browser strong cache time (seconds). Set to 0 for `no-cache` |
| `trash_retention_days` | u64 | `0` | Automatically permanently delete trashed images after this many days. Set to 0 to disable auto-cleanup. |
| `strict_health_check` | bool | `false` | When enabled, returns 503 instead of 404 when metadata exists but file is missing on all storage nodes |

### [database] Database Configuration

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `driver` | Enum | `"sqlite"` | Database driver: `sqlite` or `postgres` |
| `max_connections` | u32 | `5` | Maximum connections in the database pool |
| `min_connections` | u32 | `1` | Minimum connections in the database pool |

### [storage] Storage Configuration

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `max_payload_bytes` | u64 | - | Maximum upload size in bytes. Returns HTTP 413 if exceeded. |

### [[storage.endpoints]] Storage Endpoint Configuration

Each `[[storage.endpoints]]` block defines a storage location. The router uses a consistent hash ring weighted by `capacity_bytes` to distribute files.

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `name` | String | **required** | Unique identifier (used in env credentials and admin API). Must be unique across all endpoints. |
| `description` | String | `""` | Human-readable label (editable via admin API) |
| `type` | Enum | `"S3"` | Backend type: `"Local"` (filesystem) or `"S3"` (S3-compatible). |
| `path` | String | - | **Required for Local**: Directory path for stored files. |
| `bucket` | String | - | S3 bucket name. Can also be set via env: `IMG_ENDPOINT_{UPPER_NAME}__BUCKET` |
| `region` | String | `"us-east-1"` | S3 region. Also settable via env. |
| `force_path_style` | bool | `false` | Use path-style S3 addressing (`http://host:port/bucket/key`). Required for MinIO. |
| `direct_mode` | Enum | `"proxy"` | How clients access images: `"proxy"` (server proxies), `"presigned"` (presigned URLs), `"public"` (direct public URLs). |
| `direct_url_ttl` | u64 | - | Presigned URL TTL in seconds. Only used when `direct_mode = "presigned"`. |
| `direct_url_public_endpoint` | String | - | Override S3 endpoint URL in generated direct URLs. Useful for reverse proxy setups. |
| `capacity_bytes` | i64 | **required** | Maximum storage capacity (bytes). Used for ring weighting: larger capacity = more virtual nodes. |
| `priority` | i32 | `1` | Routing priority tier (lower = higher priority). Router tries all priority=1 endpoints first; if all full, falls through to priority=2. |

#### S3 Endpoint Environment Variables

For S3 endpoints, credentials are loaded from environment variables using the pattern:
- `IMG_ENDPOINT_{UPPER_NAME}__ENDPOINT_URL` (optional; omit for AWS S3)
- `IMG_ENDPOINT_{UPPER_NAME}__ACCESS_KEY` (required)
- `IMG_ENDPOINT_{UPPER_NAME}__SECRET_KEY` (required)
- `IMG_ENDPOINT_{UPPER_NAME}__BUCKET` (optional; can use config field)
- `IMG_ENDPOINT_{UPPER_NAME}__REGION` (optional; defaults to us-east-1)
- `IMG_ENDPOINT_{UPPER_NAME}__FORCE_PATH_STYLE` (optional; defaults to false)

Name formatting: uppercased, hyphens → underscores. Example: `name = "minio-node1"` → `IMG_ENDPOINT_MINIO_NODE1__ACCESS_KEY`.

### [image] Global Image Configuration

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `enable` | bool | `true` | Whether to enable image processing. Note: if `false`, uploads will keep EXIF according to `keep_metadata_fields` and access to `/v/[default]/[YYYY/MM/DD]/<hash>.<ext>` will return the stored data as-is; if `true`, uploads will keep all info (saved as-is), and EXIF won't be processed during upload. |
| `default_format` | Enum | `"webp"` | Default output format: `webp`, `avif`, `jpeg`, `png`, `original` (keep original format) |
| `quality` | u8 | `75` | Default transcoding quality (1-100) |
| `max_workers` | u32 | `2` | Maximum concurrent worker threads for image processing |
| `allow_show_origin` | bool | `true` | Whether to allow access to original images (full EXIF) via `/d/` path. If `false`, login is required. |
| `cache_ttl` | u64 | `25200` | Cache validity (seconds). Set to 0 for permanent cache. |
| `keep_metadata_fields` | Array | `["copyright", "settings", "time"]` | Metadata fields to keep: `camera`, `settings`, `time`, `copyright`, `location`, `others` |

### [image.dynamic] Dynamic Processing Configuration

Allows dynamic image generation via URL parameters (e.g., `/v/w_100,h_80/[YYYY/MM/DD]/<hash>.<ext>`).

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `allow` | bool | `false` | Whether to allow dynamic parameter processing. If disabled, unmatched requests return 403. |
| `allow_enlargement` | bool | `false` | Whether to allow enlarging images. |
| `max_width` | u32 | `3840` | Maximum allowed width for dynamic processing. |
| `max_height` | u32 | `2160` | Maximum allowed height for dynamic processing. |

### [image.presets.NAME] Preset Dimension Configuration

Multiple presets can be defined using `[image.presets.thumb]`, etc. `default` is a special field for requests without a preset (e.g., `/v/[YYYY/MM/DD]/<hash>.<ext>`). Accessing a non-existent preset still returns `404`.

Rules for `width` and `height`:
- If both are undefined, the image is converted at its original size.
- If only one is defined, the image is scaled proportionally.
- If both are defined, the image is scaled according to the `fit` parameter.

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `width` | u32 | - | Target width |
| `height` | u32 | - | Target height |
| `fit` | Enum | `"contain"` | Scaling mode: `cover`, `contain`, `fill`, `inside`, `outside` (only works if both W & H are set) |
| `format` | Enum | Global | Specific format for this preset |
| `quality` | u8 | Global | Specific quality for this preset |
| `eager` | bool | `false` | Whether to generate this size immediately upon upload (permanently cached if true) |
| `cache_ttl` | u64 | Global | Cache validity for this preset (seconds) |

### Environment Variables (Secrets)

Sensitive information must be passed via environment variables, with a prefix defined by `server.env_prefix` (default `IMG`).

| Variable | Description |
| :--- | :--- |
| `IMG_AUTH_TOKEN` | Authentication token for management operations (upload, delete, etc.) |
| `IMG_JWT_SECRET` | Secret key for generating and verifying authentication JWTs |
| `IMG_DATABASE_URL` | Database connection string (e.g., SQLite: `sqlite://data.db`; Postgres: `postgres://user:pass@host/db`) |

For S3 storage endpoints, see the S3 Endpoint Environment Variables section above.

## Storage Configuration Examples

### Single Local Endpoint (Minimal Setup)

```toml
[[storage.endpoints]]
name           = "local-primary"
description    = "Primary local filesystem"
type           = "Local"
path           = "./data/storage"
capacity_bytes = 10737418240    # 10 GB
priority       = 1
```

### Two Local Endpoints with Priority Tiers

```toml
# Fast NVMe storage - highest priority
[[storage.endpoints]]
name           = "nvme-fast"
description    = "NVMe primary"
type           = "Local"
path           = "/mnt/nvme/storage"
capacity_bytes = 107374182400   # 100 GB
priority       = 0              # Highest priority

# HDD fallback - used when NVMe is full
[[storage.endpoints]]
name           = "hdd-fallback"
description    = "HDD fallback"
type           = "Local"
path           = "/mnt/hdd/storage"
capacity_bytes = 1099511627776  # 1 TB
priority       = 1              # Lower priority
```

### Local Primary + S3 Failover

```toml
# Local fast storage
[[storage.endpoints]]
name           = "local-origin"
description    = "Local origin storage"
type           = "Local"
path           = "./data/storage"
capacity_bytes = 107374182400   # 100 GB
priority       = 0

# S3 cloud storage - used when local is full
[[storage.endpoints]]
name           = "cloud-s3"
description    = "S3-compatible cloud storage"
type           = "S3"
bucket         = "my-images"
region         = "us-east-1"
direct_mode    = "presigned"
direct_url_ttl = 3600
capacity_bytes = 1099511627776  # 1 TB
priority       = 1
```

### Multi-Node MinIO Cluster

```toml
[[storage.endpoints]]
name           = "minio-node1"
description    = "MinIO node 1"
type           = "S3"
bucket         = "ibed-images"
region         = "us-east-1"
force_path_style = true
direct_mode    = "presigned"
direct_url_ttl = 3600
capacity_bytes = 21474836480    # 20 GB
priority       = 1

[[storage.endpoints]]
name           = "minio-node2"
description    = "MinIO node 2"
type           = "S3"
bucket         = "ibed-images"
region         = "us-east-1"
force_path_style = true
direct_mode    = "presigned"
direct_url_ttl = 3600
capacity_bytes = 53687091200    # 50 GB
priority       = 1

[[storage.endpoints]]
name           = "minio-node3"
description    = "MinIO node 3"
type           = "S3"
bucket         = "ibed-images"
region         = "us-east-1"
force_path_style = true
direct_mode    = "public"
capacity_bytes = 107374182400   # 100 GB
priority       = 1
```

Environment variables for the above:
```bash
export IMG_ENDPOINT_MINIO_NODE1__ENDPOINT_URL="http://minio1:9000"
export IMG_ENDPOINT_MINIO_NODE1__ACCESS_KEY="minioadmin"
export IMG_ENDPOINT_MINIO_NODE1__SECRET_KEY="minioadmin"
export IMG_ENDPOINT_MINIO_NODE1__BUCKET="ibed-images"

export IMG_ENDPOINT_MINIO_NODE2__ENDPOINT_URL="http://minio2:9000"
export IMG_ENDPOINT_MINIO_NODE2__ACCESS_KEY="minioadmin"
export IMG_ENDPOINT_MINIO_NODE2__SECRET_KEY="minioadmin"

export IMG_ENDPOINT_MINIO_NODE3__ENDPOINT_URL="http://minio3:9000"
export IMG_ENDPOINT_MINIO_NODE3__ACCESS_KEY="minioadmin"
export IMG_ENDPOINT_MINIO_NODE3__SECRET_KEY="minioadmin"
```
