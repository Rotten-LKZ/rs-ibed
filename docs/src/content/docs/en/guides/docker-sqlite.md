---
title: Docker Deployment (SQLite)
description: Pull the published production image and run RS-IBED in a single container backed by SQLite.
---

This guide uses the published production image and keeps everything in one container.

## Pull the image

```bash
docker pull ghcr.io/rotten-lkz/rs-ibed:latest
```

## Prepare local directories

Create directories for the config file and persistent app data:

```bash
mkdir -p deploy data
cp config.toml deploy/config.toml
```

The sample config writes files to `./data/uploads` and `./data/cache`, so the container needs a mounted `/app/data` directory.

## Set the SQLite database URL

SQLite must point at a writable mounted path inside the container:

```bash
export IMG_DATABASE_URL="sqlite:///app/data/image_host.db?mode=rwc"
```

You also need the other required variables:

```bash
export IMG_AUTH_TOKEN="replace-with-a-long-random-token"
export IMG_JWT_SECRET="replace-with-a-long-random-secret"
```

## Run the container

```bash
docker run -d \
  --name rs-ibed \
  -p 3000:3000 \
  -v "$(pwd)/deploy/config.toml:/app/config.toml:ro" \
  -v "$(pwd)/data:/app/data" \
  -e IMG_AUTH_TOKEN="$IMG_AUTH_TOKEN" \
  -e IMG_JWT_SECRET="$IMG_JWT_SECRET" \
  -e IMG_DATABASE_URL="$IMG_DATABASE_URL" \
  ghcr.io/rotten-lkz/rs-ibed:latest \
  --config /app/config.toml
```

## Persistence caveat

When using SQLite, all of these must live on a mounted volume:

- The SQLite database file
- Uploaded originals
- Generated cache files

If you skip the `/app/data` mount, the database and image files disappear when the container is removed.

## Inspect the container

Check logs:

```bash
docker logs -f rs-ibed
```

Restart it:

```bash
docker restart rs-ibed
```

## Verify the deployment

Open `http://localhost:3000/`, upload a sample image, then restart the container and confirm the image metadata and files still exist.
