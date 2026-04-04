---
title: Getting Started
description: Quickly deploy RS-IBED, configure it, and complete your first upload.
---

This page helps you get RS-IBED running locally and complete your first image upload.

## What is RS-IBED?

RS-IBED is a Rust-powered image hosting service with support for:

- Original image uploads and delivery
- Preset-based resizing
- Dynamic resizing and transcoding
- WebP / AVIF / JPEG / PNG output
- SQLite or PostgreSQL for metadata storage
- Bearer Token and browser session authentication
- OpenAPI export and frontend SDK generation

## Installation options

You can use RS-IBED in two ways.

### Option 1: Download a prebuilt binary

1. Download the correct release for your system from GitHub Releases.
2. Prepare a `config.toml` file.
3. Set the required environment variables:

```bash
export IMG_AUTH_TOKEN="your-upload-token"
export IMG_JWT_SECRET="your-jwt-secret"
export IMG_DATABASE_URL="sqlite://data.db"
```

4. Start the service:

```bash
./rs-ibed
```

If your config file is not in the current directory, specify it explicitly:

```bash
./rs-ibed --config /path/to/config.toml
```

### Option 2: Run from source

From the repository root:

```bash
cargo build
cargo run
```

## Minimal working configuration

The following configuration is enough for a local test setup:

```toml
[server]
host = "0.0.0.0"
port = 3000
log_level = "info"
env_prefix = "IMG"
url_pattern = "ymd"
cors_allow_origins = ["*"]
cors_max_age = 3600
enable_negotiated_cache = true
cache_max_age = 3600
trash_retention_days = 30

[database]
driver = "sqlite"
max_connections = 5
min_connections = 1

[[storage.endpoints]]
name           = "local-primary"
description    = "Primary local storage"
type           = "Local"
path           = "./data/storage"
capacity_bytes = 10737418240    # 10 GB
priority       = 1

[image]
enable = true
default_format = "webp"
quality = 75
max_workers = 2
allow_show_origin = true
cache_ttl = 25200
keep_metadata_fields = ["copyright", "settings", "time"]

[image.dynamic]
allow = false
allow_enlargement = false
max_width = 3840
max_height = 2160
```

For a full explanation of every option, see [Configuration File](/en/guides/config/).

## Required environment variables

By default, the service reads environment variables using the `IMG_` prefix:

| Variable | Description |
| :--- | :--- |
| `IMG_AUTH_TOKEN` | Bearer token for protected operations such as upload and delete |
| `IMG_JWT_SECRET` | Secret used to sign browser session JWTs |
| `IMG_DATABASE_URL` | Database connection string |

If you change `server.env_prefix`, update these names accordingly.

### S3 storage endpoint credentials

For each S3 `[[storage.endpoints]]` entry in config.toml, provide credentials using the pattern:
- `IMG_ENDPOINT_{UPPER_NAME}__ACCESS_KEY`
- `IMG_ENDPOINT_{UPPER_NAME}__SECRET_KEY`

Optional fields:
- `IMG_ENDPOINT_{UPPER_NAME}__BUCKET`
- `IMG_ENDPOINT_{UPPER_NAME}__ENDPOINT_URL`
- `IMG_ENDPOINT_{UPPER_NAME}__REGION`

The name is uppercased and hyphens replaced with underscores. For example, `name = "minio-cache"` becomes `IMG_ENDPOINT_MINIO_CACHE__ACCESS_KEY`.

## Available endpoints after startup

After the service starts, these are the most common endpoints:

- `POST /api/upload`: upload an image
- `GET /v/...`: view an image or processed variant
- `GET /d/...`: download the original image
- `GET /api/openapi.json`: fetch the OpenAPI document
- `/login?token=...`: browser login entry used by the CLI login flow

## Uploading images

### Using the HTTP API

The upload endpoint is:

```text
POST /api/upload
```

Supported authentication methods:

- `Authorization: Bearer <AUTH_TOKEN>`
- Browser cookie after login

The request body uses `multipart/form-data` with:

- `file`: the image file
- `keep_metadata_fields`: optional comma-separated metadata fields to preserve, for example `camera,time`

### Upload with curl

```bash
curl -X POST "http://localhost:3000/api/upload" \
  -H "Authorization: Bearer $IMG_AUTH_TOKEN" \
  -F "file=@./photo.jpg"
```

To override the default metadata retention rules for a single upload:

```bash
curl -X POST "http://localhost:3000/api/upload" \
  -H "Authorization: Bearer $IMG_AUTH_TOKEN" \
  -F "file=@./photo.jpg" \
  -F "keep_metadata_fields=camera,time"
```

### Upload with the `upload` CLI

Release artifacts include a standalone `upload` binary, which is convenient for scripting or batch uploads.

Usage:

```bash
./upload \
  --url http://localhost:3000 \
  --token "$IMG_AUTH_TOKEN" \
  ./photo.jpg
```

Batch upload:

```bash
./upload \
  --url http://localhost:3000 \
  --token "$IMG_AUTH_TOKEN" \
  ./*.jpg
```

Override retained metadata fields:

```bash
./upload \
  --url http://localhost:3000 \
  --token "$IMG_AUTH_TOKEN" \
  --keep-metadata-fields camera,time \
  ./photo.jpg
```

You can also provide the base URL and token through environment variables:

```bash
export UPLOAD_API_URL="http://localhost:3000"
export UPLOAD_AUTH_TOKEN="$IMG_AUTH_TOKEN"

./upload ./photo.jpg
```

On success, the CLI prints only the full uploaded image URL, one per line, which makes it easy to pipe into other shell tools:

```bash
./upload ./a.jpg ./b.jpg > links.txt
```

## Upload response

A successful upload returns JSON containing the image path:

```json
{
  "id": 1,
  "hash": "...",
  "url": "/v/2026/03/26/xxxxxxxx.webp",
  "file_name": "photo.jpg",
  "mime_type": "image/jpeg",
  "size": 123456,
  "width": 1920,
  "height": 1080
}
```

The `url` field is the image path returned by the API. If you need a full URL, prepend the service base URL:

```text
http://localhost:3000/v/2026/03/26/xxxxxxxx.webp
```

## Development workflow

If you are also working on the frontend or docs, a common local workflow is:

1. Export OpenAPI:

```bash
cargo run -- export-openapi frontend/openapi.json
```

2. Generate the frontend SDK inside `frontend/`:

```bash
pnpm install
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

## Next steps

- Read [Configuration File](/en/guides/config/) for the full config reference
- Read [Development Guide](/en/guides/develop/) for local development workflow
- Open `/api/openapi.json` to inspect the current API definition
