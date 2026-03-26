# AGENTS.md

## Product Contract

GLOQ Space Allocation is a lean Revit-like MVP for:

- floor plan editing
- 3D view
- levels
- spaces
- US feet-inch UI
- WebGPU-based rendering

Mandatory stack:

- frontend shell: React + Vite + TypeScript
- 3D renderer: Rust compiled to WebAssembly with wasm-bindgen + wgpu
- backend API: Rust + axum + sqlx
- auth/db/realtime/storage: Supabase

Hard architecture rules:

- TypeScript owns the canonical `ProjectDoc` and all MVP editing and geometry logic.
- Rust wasm owns GPU rendering only.
- Rust API owns auth-aware persistence, versioning, and thin server endpoints.
- Do not create a second full BIM or domain schema in Rust.
- Persist the editor document as versioned JSONB snapshots plus small relational metadata.
- One shared domain model must drive both 2D and 3D.
- Prefer plain objects and pure functions over class hierarchies.
- One function = one responsibility.
- Keep file count and abstraction count low.
- Do not add `three.js`, `react-three-fiber`, `Babylon`, `Tailwind`, `MUI`, `Prisma`, `tRPC`, `Redux Toolkit`, or a custom ECS or scene graph unless absolutely required.

## Workflow

1. Read the user prompt.
2. If the task is not a bug fix, read `MP.md` before any other repo discovery or text search.
3. Read `SP.md` before making architecture or boundary changes.
4. Read `KL.md` when repo terms, workflow terms, or specialized calculation/runtime concepts are unclear.
5. Read the relevant task note in `tasks/`.
6. For bug-fix work, read the matching `tasks/FB<number> <task name>.md` first, then read `MP.md` only if the fix still needs repo discovery.
7. If `MP.md` does not answer where to look, then search text.
8. Make the smallest change that fits the task.
9. After every non-bug-fix task, reopen `MP.md` and update it for any file discovery, folder ownership, entry point, or document index changes clarified by the task.
10. If the task introduces a new glossary-worthy concept, changes the meaning of one, or renames a recurring repo term, update `KL.md` in the same task.
11. Keep the active task note current when the task note is part of the workflow.

## Sample Test Data Workflow

- Keep `*.test.ts` for pure logic or critical API behavior.
- For interactive editor validation cases, prefer checked-in sample data plus an in-app `Test` menu over hardcoded demo state in components.
- Put sample-case metadata and JSON loading in `apps/web/src/test-cases.ts`, and keep the in-app loader UI in `apps/web/src/test-dashboard.tsx`.
- Store reusable sample cases under `supabase/sample-data/<group>/<case-id>.json`.
- Keep each sample case as one whole snapshot-compatible `ProjectDoc` JSON document.
- Keep all sample geometry values in internal decimal feet and keep transient UI state out of sample files.
- When adding a new validation group, target `3-5` concrete cases before expanding the menu structure.
- When sample case conventions, locations, or groups change, update the active task note and `MP.md`.

## What Each File Does

- `SP.md`: architecture and implementation contract.
- `KL.md`: selective glossary of specialized repo, workflow, calculation, and runtime concepts using `AI` and `HM` entries.
- `QC.md`: repeated user-facing regression traps.
- `MP.md`: single repo map and document index. Start here for non-bug-fix tasks.
- `README.md`: human-facing project intro and quick-start.
- `tasks/<number> <task name>.md`: feature task context, plan, and verification notes.
- `tasks/FB<number> <task name>.md`: bug and failure notes. This repo does not use a root `FB.md`.

## Where To Put Things

- `apps/web/src/`: app shell, auth UI, routing, editor UI, TypeScript document logic, and unit helpers.
- `apps/api/src/`: axum HTTP surface, config loading, auth middleware, and API error shapes.
- `crates/render-wasm/src/`: Rust wasm renderer entry points and WebGPU-facing code.
- `supabase/migrations/`: schema and storage bootstrap for durable project persistence.
- `supabase/sample-data/`: checked-in sample `ProjectDoc` fixtures for manual validation and future snapshot-compatible test cases.
- `setup/`: local preview helpers and port-3001 runtime scripts.
- `tasks/`: task notes and bug notes.

## Commands

- install: `pnpm install`
- web dev: `pnpm dev:web`
- web dev on `3001`: `pnpm dev:3001`
- background preview on `3001`: `pnpm up:web:3001`
- preview smoke check on `3001`: `pnpm smoke:web:3001`
- stop background preview on `3001`: `pnpm down:web:3001`
- api dev: `pnpm dev:api`
- full dev: `pnpm dev`
- web verify: `pnpm verify:web`
- wasm build: `pnpm build:wasm`
- lint: `pnpm lint`
- tests: `pnpm test`

## Rules

- Prefer the smallest local diff.
- Reuse an existing seam before adding a new one.
- Do not search text first if `MP.md` already points to the right folder or file.
- Every non-bug-fix task starts with `MP.md` and ends with an `MP.md` review before reporting completion.
- If `MP.md` is missing the needed path, update it during task closeout instead of leaving discovery to search-only.
- Update `KL.md` in the same task when you introduce a new specialized repo concept, change the meaning of one, or rename a recurring term contributors should keep using.
- Do not update `KL.md` for ordinary local variables, generic framework terms, or trivial refactors with no terminology change.
- Keep editor-only session state out of the Rust API and out of Supabase schema unless it must persist.
- Keep browser auth config browser-safe. Never expose service-role credentials in `apps/web`.
- Keep internal length math in decimal feet. Feet-inch parsing and formatting are UI-only.
- Add tests only for pure logic or critical API behavior.
- Do not invent a second sample-data schema for editor validation; keep sample files aligned with canonical `ProjectDoc` snapshots.
- If you change the wasm renderer contract or `crates/render-wasm/src/lib.rs`, rebuild `crates/render-wasm/pkg` in the same task before claiming web verification is complete.
- After each task, run the smallest relevant build or test, fix obvious breakages, then report:
  1. changed files
  2. commands run
  3. remaining TODOs
