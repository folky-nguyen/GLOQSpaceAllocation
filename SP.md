# SP

Architecture and implementation contract for future GLOQ Space Allocation code.

## 1. System Contract

- This repo is a lean Revit-like MVP, not a generic drawing app.
- The MVP scope is floor plan editing, 3D view, levels, spaces, US feet-inch UI, and WebGPU rendering.
- Within the editor runtime, TypeScript owns the canonical `ProjectDoc`.
- Durable project persistence stores that document as versioned JSONB snapshots in Supabase.
- If a shortcut creates a second authoritative model in Rust, SQL, or UI state, it is the wrong design.

## 2. Runtime Boundaries

### TypeScript

- TypeScript owns:
  - `ProjectDoc`
  - level and space editing logic
  - MVP geometry logic
  - unit parsing and formatting
  - route protection and browser auth flow
  - 2D and 3D view orchestration
- TypeScript may keep transient interaction and session state.
- TypeScript should prefer plain objects and pure functions over classes.

### Rust wasm + wgpu

- Rust wasm owns GPU rendering only.
- The wasm layer may probe adapters, manage WebGPU device setup, and draw derived render payloads.
- The wasm layer must not become a second owner of authored BIM or editing truth.
- New rendering work targets `wgpu` and WebGPU, not WebGL fallback paths.

### Rust API

- Rust API owns thin, auth-aware HTTP endpoints.
- Rust API owns persistence plumbing, snapshot versioning, and access checks.
- Rust API must not grow a second full domain model for levels, spaces, or floor-plan editing.
- API handlers should stay thin until repetition proves a new seam is needed.

## 3. Data Ownership And Persistence

- Durable storage is Supabase Postgres plus Supabase Storage.
- The persisted project document is a JSONB snapshot, not decomposed BIM tables.
- Relational metadata should stay small and explicit:
  - `projects`
  - `project_members`
  - `project_snapshots`
- Derived render data, caches, and view artifacts must be reconstructable from the TypeScript-owned document.
- Storage buckets may hold project assets, but must not replace the snapshot as the document source of truth.

## 4. Shared Domain Model

- One shared domain model must drive both 2D and 3D.
- `Level` and `Space` are first-class document concepts.
- A 3D view may derive its display from the same document used by the floor plan.
- Do not model separate 2D-only and 3D-only authored schemas for the same MVP entity.

## 5. Units And Geometry

- Internal length math stays in decimal feet.
- Feet-inch formatting and parsing happen only at the UI boundary.
- Canonical geometry data must stay numeric and unit-stable.
- Parser convenience must not leak shorthand formats into stored data.

## 6. Auth And Security Boundaries

- Browser auth uses Supabase with publishable credentials only.
- `apps/web` must never read or expose service-role credentials.
- API Bearer auth should verify Supabase access tokens against the project's JWKS.
- Auth context should stay small: user ID plus minimal metadata such as email or role when needed.

## 7. Performance And Rendering Rules

- Keep hot loops and GPU-facing work out of React render paths.
- Keep React responsible for shell, forms, orchestration, and derived display state.
- WebGPU is the target render path for the browser renderer.
- Do not add WebGL or `three.js` fallback architecture.

## 8. Architectural Guardrails

- Do not introduce `three.js`, `react-three-fiber`, `Babylon`, `Tailwind`, `MUI`, `Prisma`, `tRPC`, `Redux Toolkit`, or a custom ECS or scene graph unless absolutely required.
- Do not create a parallel Rust domain schema for authored model state.
- Do not persist ephemeral editor chrome state as durable project truth.
- When adding a field, decide whether it is canonical, derived, or transient before landing the change.
- When folder ownership or repo discovery changes, update `MP.md`.
