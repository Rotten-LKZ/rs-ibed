---
title: Configuration File
description: How to configure the application using the configuration file.
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

[database]
driver = "sqlite"          # "sqlite" | "postgres"
max_connections = 5
min_connections = 1

[storage]
type = "local"             # "local"
base_dir = "./uploads"
cache_dir = "./cache"

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

### [database] Database Configuration

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `driver` | Enum | `"sqlite"` | Database driver: `sqlite` or `postgres` |
| `max_connections` | u32 | `5` | Maximum connections in the database pool |
| `min_connections` | u32 | `1` | Minimum connections in the database pool |

### [storage] Storage Configuration

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `base_dir` | String | `"./uploads"` | Root directory for original image storage |
| `cache_dir` | String | `"./cache"` | Cache directory for processed images (thumbnails, etc.) |

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
