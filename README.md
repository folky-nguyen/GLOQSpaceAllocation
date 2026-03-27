# GLOQ Space Allocation

GLOQ Space Allocation is a lean Revit-like MVP that turns a space-allocation workflow into a shared 2D floor plan and 3D model experience.

Current product scope:

- floor plan editor
- 3D view
- levels
- spaces
- US feet-inch UI
- WebGPU-based rendering

## Tech Stack

- React + Vite + TypeScript for the frontend shell
- Rust + wasm-bindgen + wgpu for the browser renderer
- Rust + axum + sqlx for the backend API
- Supabase for auth, Postgres, storage, and future realtime

## Repo Shape

- `apps/web`: React app shell, auth flow, editor UI, `ProjectDoc`, units, and feature-local TypeScript logic
- `apps/api`: thin axum API with health, version, and Supabase Bearer JWT verification
- `crates/render-wasm`: Rust wasm crate for WebGPU probing and renderer work
- `supabase`: local Supabase config and SQL migrations
- `setup`: helper scripts for local preview on port `3001`
- `tasks`: feature notes and bug notes

## Quick Start

Install dependencies:

```bash
pnpm install
```

Run the web app:

```bash
pnpm dev:web
```

Run the web app on port `3001`:

```bash
pnpm dev:3001
```

Run the API:

```bash
pnpm dev:api
```

Run web + API together:

```bash
pnpm dev
```

Build the wasm renderer package:

```bash
pnpm build:wasm
```

Run the smallest repo checks:

```bash
pnpm verify:web
pnpm test
```

## Environment Notes

Web env lives in `apps/web/.env.local` and starts from `apps/web/.env.example`.

Current browser-safe web vars:

- `VITE_SUPABASE_URL`
- `VITE_SUPABASE_PUBLISHABLE_KEY`
- `VITE_LOCAL_AUTH_BYPASS`
  - temporary browser-side bypass for opening `/editor` directly in local or deployed client builds

API env currently expects:

- `DATABASE_URL`
- `SUPABASE_URL`
- optional `API_HOST`
- optional `API_PORT`

## Persistence Direction

- TypeScript owns the canonical in-editor `ProjectDoc`.
- Durable persistence is stored in Supabase as versioned JSONB snapshots.
- Postgres tables stay thin: `projects`, `project_members`, and `project_snapshots`.
- Rust does not own a second BIM schema for the MVP.

## For Coding Agents

Read [AGENTS.md](./AGENTS.md) first.
