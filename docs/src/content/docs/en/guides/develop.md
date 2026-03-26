---
title: Development Guide
description: Local development notes for the RS-IBED backend, frontend, and documentation site.
---

This page explains how to work on RS-IBED locally, including the backend, frontend, OpenAPI workflow, and docs site.

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

### Useful files to inspect

When working on upload, resizing, or image delivery, these files are usually the most relevant:

- `src/handlers/upload.rs`
- `src/handlers/view.rs`
- `src/router.rs`
- `src/config.rs`
- `src/auth.rs`

### Common things to verify

If an API request fails, check:

- whether `IMG_AUTH_TOKEN` is correct
- whether `IMG_DATABASE_URL` is reachable
- whether `url_pattern` in `config.toml` matches the paths you are testing
- whether `image.allow_show_origin` affects original file access
- whether OpenAPI has been re-exported after backend changes

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
