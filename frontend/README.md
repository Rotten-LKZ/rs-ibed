# rs-ibed Frontend

`frontend/` is the web admin interface for rs-ibed, built with SvelteKit 5, Svelte 5 runes, and Tailwind CSS 4.

It is mainly responsible for:

- handling login sessions
- browsing and viewing images
- uploading images
- managing trashed images
- communicating with the rs-ibed backend through the generated TypeScript SDK

## Tech stack

- SvelteKit 2
- Svelte 5
- Tailwind CSS 4
- TypeScript
- `@hey-api/openapi-ts` for SDK generation

## Directory overview

```text
frontend/
├── src/
│   ├── lib/
│   │   ├── api.ts
│   │   ├── components/
│   │   ├── i18n/
│   │   ├── sdk/
│   │   └── stores/
│   └── routes/
├── static/
├── package.json
└── openapi.json
```

- `src/routes/`: pages and route handlers
- `src/lib/components/`: shared UI components
- `src/lib/i18n/`: locale state and translations
- `src/lib/stores/`: frontend state such as theme
- `src/lib/sdk/`: generated API client code
- `openapi.json`: OpenAPI input used for SDK generation

## Requirements

Before developing, install:

- Node.js
- `pnpm`

This frontend is usually developed together with the rs-ibed backend because it depends on the backend API.

## Install dependencies

Run inside `frontend/`:

```bash
pnpm install
```

## Local development

Start the dev server inside `frontend/`:

```bash
pnpm dev
```

To open the browser automatically:

```bash
pnpm dev -- --open
```

## Common commands

Run all commands inside `frontend/`:

| Command        | Description                            |
| :------------- | :------------------------------------- |
| `pnpm install` | Install dependencies                   |
| `pnpm dev`     | Start the development server           |
| `pnpm build`   | Build the production app               |
| `pnpm preview` | Preview the production build locally   |
| `pnpm check`   | Run Svelte type checks                 |
| `pnpm lint`    | Run Prettier and ESLint checks         |
| `pnpm format`  | Format frontend code                   |
| `pnpm gen:api` | Regenerate the SDK from `openapi.json` |

## Working with the backend

This frontend depends on the rs-ibed backend API. The recommended local workflow is:

### 1. Export OpenAPI from the repository root

```bash
cargo run -- export-openapi frontend/openapi.json
```

### 2. Generate the frontend SDK in `frontend/`

```bash
pnpm gen:api
```

Generated files are written to:

- `src/lib/sdk/`

If backend endpoints, request parameters, or response shapes change, re-export OpenAPI and regenerate the SDK.

### 3. Start the backend

Run from the repository root:

```bash
cargo run
```

### 4. Start the frontend

Run inside `frontend/`:

```bash
pnpm dev
```

## Authentication flow

This project includes a CLI-to-browser login flow:

- the backend prints a login link after startup
- the link usually targets `/login?token=...`
- when opened in a browser, the frontend calls `/api/auth/cli`
- the backend then writes the auth cookie so the admin UI becomes authenticated

The frontend SDK also uses browser credentials for authenticated requests.

## Development notes

### Svelte 5 runes

This project uses Svelte 5 runes.

- `$state` and other runes only work in `.svelte` or `.svelte.ts` files
- if you want to use runes in a module file, use `*.svelte.ts`
- imports from those modules must include the explicit `.svelte` suffix

Example:

```ts
import { getTheme } from '$lib/stores/theme.svelte';
```

### i18n and theme

- `src/lib/i18n/`: locale switching and translations
- `src/lib/stores/theme.svelte.ts`: theme state

If you add UI text, update both English and Chinese translations.

### Generated SDK files

Files under `src/lib/sdk/` are generated outputs. When the API changes, prefer updating the backend OpenAPI spec and regenerating instead of manually maintaining generated code.

## Before submitting changes

It is recommended to run at least:

```bash
pnpm check
pnpm build
```

If you changed API-related code, also run:

```bash
pnpm gen:api
```

## Related files

- repository overview: `../README.md`
- development guide: `../docs/src/content/docs/en/guides/develop.md`
