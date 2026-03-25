# KL

Glossary of repo and workflow terms.

## Workflow Terms

- `MP.md`: the single repo map and document index. Read this first for non-bug-fix tasks.
- `SP.md`: the architecture contract for data ownership and runtime boundaries.
- `QC.md`: repeated user-facing regression traps.
- `task note`: a feature note in `tasks/<number> <task name>.md`.
- `bug note`: a bug or failure note in `tasks/FB<number> <task name>.md`.

## Domain Terms

- `ProjectDoc`: the canonical TypeScript document for the MVP.
- `Level`: a vertical datum stored in the TypeScript document.
- `Space`: a level-owned rectangular space stored in the TypeScript document.
- `snapshot`: one versioned JSONB copy of the project document persisted to Supabase.

## Runtime Terms

- `activeView`: the current editor view mode, currently `plan` or `3d`.
- `activeTool`: the current editor tool mode, currently `select`, `space`, or `level`.
- `selection`: the current UI selection in the editor shell.
- `auth snapshot`: the small browser-side auth state managed by `apps/web/src/auth.ts`.
- `local auth bypass`: local-only development shortcut controlled by `VITE_LOCAL_AUTH_BYPASS` on `localhost`.

## Data Terms

- `canonical`: the source-of-truth representation for authored editor state.
- `derived`: data rebuilt from the canonical document, such as render payloads or summaries.
- `transient`: interaction-only or session-only state that should not become durable project truth.

## Units Terms

- `decimal feet`: the internal numeric unit used for geometry math in the MVP.
- `feet-inch UI`: the display and input layer for imperial lengths.
- `markerless shorthand`: unit input like `12 3 3/4` that is accepted only at the parsing boundary.

## Repo Seams

- `apps/web/src/`: React shell, routes, auth UI, document helpers, units, and editor state.
- `apps/api/src/`: axum router, config, auth middleware, and HTTP error handling.
- `crates/render-wasm/src/`: Rust wasm entry points for WebGPU work.
- `supabase/migrations/`: durable schema for projects, members, and snapshots.
- `setup/`: helper scripts for preview and local port-3001 control.
