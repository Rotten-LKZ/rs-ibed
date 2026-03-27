---
title: Docker Deployment (PostgreSQL)
description: Run RS-IBED with PostgreSQL using the shared production image and a compose example.
---

This guide uses the same shared production image as the SQLite guide, but pairs the app with PostgreSQL through Compose.

## Why Compose is recommended

The PostgreSQL deployment needs an app container, a database container, persistent volumes, and environment wiring. `docker compose` keeps those pieces together in one file.

## Required environment variables

The app still needs these variables:

- `IMG_AUTH_TOKEN`
- `IMG_JWT_SECRET`
- `IMG_DATABASE_URL`

The PostgreSQL container also needs its own settings, such as:

- `POSTGRES_DB`
- `POSTGRES_USER`
- `POSTGRES_PASSWORD`

In the provided example, the app uses:

```text
postgres://ibed:ibed-password@postgres:5432/ibed
```

## Compose file

Download the compose file first. It builds the app from the root `Dockerfile`, starts PostgreSQL, and mounts persistent volumes for database and application data.

```bash
curl -L https://docsib.rotcool.me/docker-compose.pgsql.yml -o docker-compose.pgsql.yml
```

Before starting it, copy your config file to `./deploy/config.toml` in the repository root:

```bash
mkdir -p deploy
cp config.toml deploy/config.toml
```

## Start the stack

```bash
docker compose -f docker-compose.pgsql.yml up -d --build
```

## Readiness and permissions caveats

- The app depends on PostgreSQL being reachable before startup completes
- The compose example includes a PostgreSQL healthcheck and waits for the database container to report healthy
- On startup, RS-IBED may connect to the maintenance database first and attempt to create the target database automatically
- That means the configured PostgreSQL user needs permission to connect to `postgres` and create the target database, unless the target database already exists or you deliberately use `postgres` as the target database name

## Inspect the stack

Check service status:

```bash
docker compose -f docker-compose.pgsql.yml ps
```

Follow app logs:

```bash
docker compose -f docker-compose.pgsql.yml logs -f app
```

Follow PostgreSQL logs:

```bash
docker compose -f docker-compose.pgsql.yml logs -f postgres
```

## Verify the deployment

Open `http://localhost:3000/`, upload a sample image, then restart the stack and confirm that both metadata and files persist.
