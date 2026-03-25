# MP

Combined repo map and document index. This is the first lookup surface for non-bug-fix work.

## How To Use It

- Start here when you need the right folder, note, or doc.
- For every non-bug-fix task, read this file before any other repo discovery or text search.
- If the task is a bug fix, read the matching `tasks/FB*.md` note first and come back here only if discovery is still unclear.
- End every non-bug-fix task by reopening this file and updating any folder, doc, entry point, ownership note, or repeated discovery path clarified by the task.
- Do not treat a non-bug-fix task as complete until the `MP.md` closeout pass is done.

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
- `supabase/`: local Supabase config, SQL migrations, and checked-in sample editor data.
- `setup/`: local preview helpers for port `3001`.
- `tasks/`: feature notes and bug notes.
- `vercel.json`: root Vercel deploy config for the frontend preview build and SPA rewrites.
- `setup/ensure-render-wasm.mjs`: web-build helper that rebuilds or reuses `crates/render-wasm/pkg` depending on toolchain availability.

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
| Sample editor cases | `supabase/sample-data/` |
| Local preview on port `3001` | `setup/` |
| Task context | `tasks/` |

## Key Entry Files

- `apps/web/src/main.tsx`: React bootstrap.
- `apps/web/src/App.tsx`: route composition and protected editor entry.
- `apps/web/src/editor-shell.tsx`: main editor UI shell.
- `apps/web/src/auth.ts`: browser auth client and auth snapshot store.
- `apps/web/src/project-doc.ts`: current `ProjectDoc`, `Level`, and `Space` document helpers.
- `apps/web/src/space-scene.ts`: derived 3D scene payload and default camera helpers.
- `apps/web/src/test-cases.ts`: manifest and JSON loaders for level, space, and mixed validation cases.
- `apps/web/src/test-dashboard.tsx`: draggable in-workspace test dashboard for loading sample cases.
- `apps/web/src/three-d-viewport.tsx`: 3D viewport lifecycle, wasm mount, and camera interactions.
- `apps/web/src/units.ts`: feet-inch parsing, default-unit bare-number parsing, formatting, and conversion helpers.
- `apps/web/src/units-inspector.tsx`: manual diagnostic panel for unit parsing, formatting, and conversion checks.
- `apps/web/src/ui-store.ts`: small editor chrome store.
- `apps/api/src/main.rs`: axum router and API wiring.
- `apps/api/src/auth.rs`: Bearer JWT verification against Supabase JWKS.
- `apps/api/src/config.rs`: env loading for API host, port, DB, and Supabase URL.
- `apps/api/src/error.rs`: JSON API error envelope.
- `crates/render-wasm/src/lib.rs`: wasm WebGPU entry points.
- `supabase/migrations/20260324170000_init.sql`: current schema and storage bootstrap.
- `supabase/sample-data/README.md`: conventions for checked-in sample `ProjectDoc` fixtures used by manual validation.
- `supabase/sample-data/levels/*.json`: level-focused sample `ProjectDoc` fixtures.
- `supabase/sample-data/spaces/*.json`: space-layout sample `ProjectDoc` fixtures with polygon apartments.
- `supabase/sample-data/mixed/*.json`: combined level + apartment layout sample `ProjectDoc` fixtures.
- `vercel.json`: Vercel build/output config and SPA rewrite fallback for preview deployments.
- `setup/ensure-render-wasm.mjs`: build-time guard for `crates/render-wasm/pkg` used by web dev and production builds.

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
- `007.02 decimal-inch and default-unit parsing.md`: accept decimal-inch input and unit-aware bare numeric parsing with feet as the default bare unit.
- `008 level manager.md`: add level CRUD, reorder, elevation editing, and active-level-driven plan and 3D filtering.
- `009 3d space view.md`: replace the 3D placeholder with a real WebGPU-backed viewport and simulate spaces as simple 3D prisms.
- `009.01 3d view polish.md`: improve the 3D view presentation, naming, and scene readability without changing the ownership model.
- `010 selection dropdown and workspace cleanup.md`: replace the dead left tool stack with one working selection dropdown and remove old workspace chrome that wasted plan space.
- `011 overlay-safe editor shell.md`: harden the editor shell layout so ribbon, workspace chrome, and overlays do not overlap under zoom, DPI, or text growth.
- `012 test menu and sample case fixtures.md`: original test-menu note, now superseded by `012.01`.
- `012.01 draggable test dashboard and polygon apartment cases.md`: replace the flyout menu with a draggable test dashboard and require polygon apartment sample cases for level, space, and mixed validation.

### Bug Notes

- `FB001 editor unreachable on port 3001.md`: diagnose and stabilize the local preview flow when port `3001` is occupied.
- `FB002 vercel preview deployment not found.md`: document the Vercel `NOT_FOUND` preview failure and the explicit monorepo + SPA routing fix.

### Setup Docs

- `setup/README.md`: explain the helper scripts used for local preview and port-3001 control.
- `setup/ensure-render-wasm.mjs`: rebuild or reuse the generated wasm package before web dev/build commands.
- `apps/web/.env.example`: list the browser-safe Supabase and local auth env variables for the web app.

## Discovery Tips

- For non-bug-fix work, use this file before any text search or broad repo discovery.
- After non-bug-fix work, do a final `MP.md` pass before reporting completion.
- For bug-fix work, start from the matching `tasks/FB*.md` note.
- The repo is still small, so most subsystem entry points are one file deep from the folders listed above.
