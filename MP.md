# MP

Combined repo map and document index. This is the first lookup surface for non-bug-fix work.

## How To Use It

- Start here when you need the right folder, note, or doc.
- If the task is not a bug fix, read this file before searching text.
- If the task is a bug fix, read the matching `tasks/FB*.md` note first and come back here only if discovery is still unclear.
- If this file is missing a new folder, doc, or repeated entry point, update it after the task.

## Workflow Docs

- [AGENTS](./AGENTS.md)
- [SP](./SP.md)
- [KL](./KL.md)
- [QC](./QC.md)
- [README](./README.md)
- [Setup README](./setup/README.md)

## Top-Level Map

- `apps/web/`: React + Vite frontend shell and TypeScript editor logic.
- `apps/api/`: Rust + axum API.
- `crates/render-wasm/`: Rust wasm renderer crate.
- `supabase/`: local Supabase config and SQL migrations.
- `setup/`: local preview helpers for port `3001`.
- `tasks/`: feature notes and bug notes.

## Best Folder To Inspect First

| Subsystem | Start Folder |
| --- | --- |
| Web shell and routes | `apps/web/src/` |
| Browser auth | `apps/web/src/` |
| TypeScript document logic | `apps/web/src/` |
| Units and imperial parsing | `apps/web/src/` |
| API HTTP surface | `apps/api/src/` |
| API auth and JWT verification | `apps/api/src/` |
| WebGPU wasm renderer | `crates/render-wasm/src/` |
| Supabase persistence | `supabase/migrations/` |
| Local preview on port `3001` | `setup/` |
| Task context | `tasks/` |

## Key Entry Files

- `apps/web/src/main.tsx`: React bootstrap.
- `apps/web/src/App.tsx`: route composition and protected editor entry.
- `apps/web/src/editor-shell.tsx`: main editor UI shell.
- `apps/web/src/auth.ts`: browser auth client and auth snapshot store.
- `apps/web/src/project-doc.ts`: current `ProjectDoc`, `Level`, and `Space` document helpers.
- `apps/web/src/units.ts`: feet-inch parsing, formatting, and conversion helpers.
- `apps/web/src/ui-store.ts`: small editor chrome store.
- `apps/api/src/main.rs`: axum router and API wiring.
- `apps/api/src/auth.rs`: Bearer JWT verification against Supabase JWKS.
- `apps/api/src/config.rs`: env loading for API host, port, DB, and Supabase URL.
- `apps/api/src/error.rs`: JSON API error envelope.
- `crates/render-wasm/src/lib.rs`: wasm WebGPU entry points.
- `supabase/migrations/20260324170000_init.sql`: current schema and storage bootstrap.

## Document Index

### Feature Tasks

- `001 minimal editor shell.md`: build the first lean editor shell with ribbon, properties, workspace, browser, and status bar.
- `002 preview 3001.md`: standardize local preview and helper scripts for running the web app on port `3001`.
- `003 api server skeleton.md`: add the first axum API shell with config loading, CORS, health, and version routes.
- `004 supabase project persistence.md`: define thin project, member, and snapshot persistence in Supabase with JSONB snapshots.
- `005 supabase auth in web.md`: wire Supabase passwordless auth into the web app and protect the editor route.
- `006 supabase bearer jwt auth.md`: verify Supabase Bearer tokens in the API and expose authenticated user context through `/api/me`.
- `007 units module.md`: add core feet-inch parsing, formatting, conversion, and tests for the imperial units layer.
- `007.01 units parser hardening.md`: expand shorthand imperial parsing and tighten parser behavior with broader test coverage.
- `008 level manager.md`: add level CRUD, reorder, elevation editing, and active-level-driven plan and 3D filtering.
- `009 3d space view.md`: replace the 3D placeholder with a real WebGPU-backed viewport and simulate spaces as simple 3D prisms.

### Bug Notes

- `FB001 editor unreachable on port 3001.md`: diagnose and stabilize the local preview flow when port `3001` is occupied.

### Setup Docs

- `setup/README.md`: explain the helper scripts used for local preview and port-3001 control.
- `apps/web/.env.example`: list the browser-safe Supabase and local auth env variables for the web app.

## Discovery Tips

- For non-bug-fix work, use this file before text search.
- For bug-fix work, start from the matching `tasks/FB*.md` note.
- The repo is still small, so most subsystem entry points are one file deep from the folders listed above.
