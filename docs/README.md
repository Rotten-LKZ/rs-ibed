# rs-ibed Docs

`docs/` is the documentation site for rs-ibed, built with Astro and Starlight.

It contains:

- usage and deployment guides
- configuration documentation
- development guides
- API reference generated from OpenAPI

## Tech stack

- Astro
- Starlight
- `starlight-openapi`

## Directory structure

```text
docs/
├── public/
├── src/
│   ├── assets/
│   ├── content/
│   │   └── docs/
│   │       ├── en/
│   │       └── zh/
│   └── content.config.ts
├── astro.config.mjs
├── package.json
└── tsconfig.json
```

- `src/content/docs/en/`: English documentation
- `src/content/docs/zh/`: Chinese documentation
- `astro.config.mjs`: Starlight, sidebar, locale, and OpenAPI integration config

## Install dependencies

Run inside `docs/`:

```bash
pnpm install
```

## Local development

Start the docs site inside `docs/`:

```bash
pnpm dev
```

This starts the local development server for previewing content and navigation changes.

## Common commands

Run all commands inside `docs/`:

| Command | Description |
| :-- | :-- |
| `pnpm install` | Install dependencies |
| `pnpm dev` | Start the local development server |
| `pnpm build` | Build the production docs site |
| `pnpm preview` | Preview the production build locally |
| `pnpm astro -- --help` | Show Astro CLI help |

## Content organization

The docs site uses a localized content structure:

- `en/`: English
- `zh/`: Simplified Chinese

In most cases:

- when adding a Chinese page, add the matching English page too
- when adding an English page, add the matching Chinese page too
- keep both locale trees aligned so sidebar structure and routes stay consistent

## API reference source

The docs site uses `starlight-openapi` and reads the repository root `openapi.json` file to generate:

- `en/api`
- `zh/api`

See `docs/astro.config.mjs` for the integration setup.

If backend endpoints change, the recommended update flow is:

### 1. Re-export OpenAPI from the repository root

```bash
cargo run -- export-openapi
```

If the frontend SDK also needs updating, export to:

```bash
cargo run -- export-openapi frontend/openapi.json
```

### 2. Restart or rebuild the docs site

Run inside `docs/`:

```bash
pnpm dev
# or
pnpm build
```

## Writing documentation

- put pages under `src/content/docs/`
- follow the existing locale directory structure
- keep titles, descriptions, slugs, and sidebar labels consistent
- keep content focused on rs-ibed instead of default template instructions

## Relationship to the main project

This is the documentation subproject inside the rs-ibed monorepo. It is usually updated together with:

- root `README.md` for project overview
- `docs/` for user-facing documentation
- `frontend/` for the admin interface
- root `openapi.json` as the API reference source

## Before submitting changes

If you changed documentation content, it is recommended to run:

```bash
pnpm build
```

If you changed API reference related content, make sure the repository root `openapi.json` is up to date first.

## Related files

- English development guide: `src/content/docs/en/guides/develop.md`
- English getting started guide: `src/content/docs/en/guides/getting-started.md`
