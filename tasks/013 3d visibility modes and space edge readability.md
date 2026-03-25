# 013 3D Visibility Modes And Space Edge Readability

## Goal

In `apps/web` and `crates/render-wasm`, extend the current `3D View` with:

- `Active Floor Only`
- `All Levels`

`Active Floor Only` should stay as the default so the current behavior is preserved.

The 3D scene should also draw colored space edges so each space boundary reads clearly when polygon spaces touch or stack across levels.

This task should stay lean:

- TypeScript still owns visibility decisions and derived geometry
- Rust wasm still owns draw setup and GPU upload only
- reuse the current 3D viewport seam
- avoid global-state churn, scene graphs, and post-process outline systems

## Current Repo State

`apps/web` currently has:

- `src/space-scene.ts`
  - builds the 3D payload from the canonical `ProjectDoc`
  - filters spaces to `activeLevelId`
  - sends one `vertices` array for filled prism triangles
  - has no separate edge payload
- `src/three-d-viewport.tsx`
  - builds the 3D scene from `project`, `activeLevelId`, and `selection`
  - owns camera interactions and the `Fit` button
  - has no visibility-mode toggle
- `src/editor-shell.tsx`
  - mounts `ThreeDViewport` only when the `3D View` tab is active
  - already owns nearby transient editor session state
- `src/ui-store.ts`
  - owns cross-view state such as `activeView`, `selectMode`, and `selection`
  - does not need to change for this task unless later reuse proves it necessary

`crates/render-wasm` currently has:

- `src/lib.rs`
  - one triangle-list pipeline for filled geometry
  - one vertex buffer path
  - no edge/line pass

## Scope

In scope:

- one explicit 3D visibility control
- default `Active Floor Only` behavior
- `All Levels` visibility across the full building stack
- edge rendering for each visible space
- keep active-level and selected-space emphasis in both modes
- tests only for pure TypeScript scene building

Out of scope:

- `ProjectDoc` schema changes
- persisted user preferences
- plan-view visibility changes
- project-browser changes
- GPU picking
- post-process outlines or glow
- custom line-thickness systems
- API or Supabase changes

## Implementation Decisions

### 1. Keep visibility mode as local editor session state

Prefer one local `threeDVisibilityMode` state in `apps/web/src/editor-shell.tsx`.

Use:

```ts
type ThreeDVisibilityMode = "active-floor-only" | "all-levels";
```

Rules:

- default to `active-floor-only`
- keep it transient
- do not persist it in `ProjectDoc`
- do not add it to the API
- do not move it into `ui-store.ts` unless another surface later needs the same state

Reason:

- the state is specific to the mounted 3D surface
- `editor-shell.tsx` already owns nearby session-only UI state
- this avoids unnecessary global-store churn

### 2. TypeScript decides the visible scene

Update `buildSpaceScenePayload(...)` so visibility is decided before any payload reaches wasm.

Suggested input:

```ts
buildSpaceScenePayload(doc, {
  activeLevelId,
  selection,
  visibilityMode
});
```

Rules:

- `active-floor-only`: include only spaces on `activeLevelId`
- `all-levels`: include all spaces with a matching level
- scene extents must be computed from the same visible set that gets rendered

Reason:

- visibility is editor logic derived from the canonical TypeScript document
- Rust must not infer level-filtering rules on its own

### 3. Keep the existing fill payload key and add only one edge payload

Do not rename the current filled-geometry payload key.

Keep:

```ts
type SpaceScenePayload = {
  items: SpacePrismRenderItem[];
  vertices: SceneVertex[];
  edgeVertices: SceneVertex[];
  extents: SceneExtents;
  hasVisibleItems: boolean;
};
```

Reason:

- `crates/render-wasm/src/lib.rs` already consumes `vertices`
- keeping the existing fill key avoids unnecessary churn
- the only new renderer contract needed for this task is `edgeVertices`

### 4. Generate only perimeter and corner edges

Edge geometry should come from the same visible prism data already derived in TypeScript.

Draw only:

- top perimeter edges
- bottom perimeter edges
- vertical corner edges

Do not draw:

- triangulation diagonals
- internal top-face edges
- shared-edge deduping across neighboring spaces in the first pass

Reason:

- perimeter and corner lines are enough to clarify each space
- skipping dedupe and triangulation edges keeps the geometry path small and predictable

### 5. Keep one simple emphasis hierarchy in both modes

Visual priority stays:

1. selected space
2. other spaces on the active level
3. spaces on all other levels

Rules:

- the active level still comes from the existing editor context
- `All Levels` changes visibility, not the definition of the active level
- edge colors should be derived from the same emphasis state as face colors

Reason:

- this keeps the 3D scene readable without introducing another visibility matrix

### 6. Use one cheap edge pass in wasm

In `crates/render-wasm/src/lib.rs`, keep the renderer minimal:

- filled faces still draw from `vertices` with `TriangleList`
- edges draw from `edgeVertices` with `LineList`
- keep fixed 1px WebGPU lines
- reuse the current shader unless a second shader becomes clearly necessary

Do not add:

- post-process outlines
- silhouettes
- line-width controls
- a material system

Reason:

- the task only needs clearer space separation
- a second lightweight draw pass is enough

### 7. Refit the camera only when the mode changes or the user asks for it

Changing the visibility mode can change scene extents a lot.

Behavior:

- fit once on first 3D load
- fit again when the user changes `Active Floor Only` vs `All Levels`
- fit when the user clicks `Fit`
- do not auto-fit on every selection change
- do not auto-fit on every active-level change unless that change is part of an explicit mode switch

Reason:

- the user keeps manual camera control during ordinary browsing
- the viewport still recenters when the scene scope changes materially

### 8. Keep the UI change inside the 3D overlay

Add one compact control near the existing 3D overlay actions, for example:

- `Scope: Active Floor Only | All Levels`

Keep this task focused on the 3D surface.

Do not redesign:

- the plan viewport
- the browser lists
- the broader shell layout

One small footer/status-text fix is acceptable only if the current wording becomes clearly misleading in `All Levels`.

### 9. Make the empty state actionable

If `active-floor-only` shows no spaces but `all-levels` would show some, the empty state should say that clearly and point the user to `All Levels`.

Reason:

- otherwise the new control is easy to miss
- the user gets a product explanation instead of a renderer-looking failure

## File Plan

### 1. `apps/web/src/editor-shell.tsx`

Add only:

- local `threeDVisibilityMode` state
- prop wiring into `ThreeDViewport`

No broader shell refactor.

### 2. `apps/web/src/three-d-viewport.tsx`

Add:

- the visibility-mode control
- scene building with `visibilityMode`
- camera refit only on explicit scope changes
- updated empty-state copy

### 3. `apps/web/src/space-scene.ts`

Update:

- filtering rules for the two visibility modes
- scene extents from the visible set
- `edgeVertices` generation
- edge colors derived from emphasis

### 4. `apps/web/src/space-scene.test.ts`

Add or update tests for:

- active-floor-only filtering
- all-levels filtering
- multi-level extents
- selected-space emphasis precedence
- edge generation for visible spaces
- empty-scene safety

### 5. `apps/web/src/styles.css`

Add only the styles needed for:

- the compact scope toggle
- any small overlay spacing updates

### 6. `crates/render-wasm/src/lib.rs`

Extend the renderer to:

- parse `edgeVertices`
- upload a second vertex buffer
- run a second edge draw pass after filled faces

No renderer-framework refactor.

## UI Behavior

### Active Floor Only

- matches today’s default 3D scope
- renders only spaces on the active level
- still draws edges for those visible spaces

### All Levels

- renders spaces from every level
- keeps active-level spaces more emphasized
- keeps the selected space as the strongest highlight
- uses the same edge treatment across the full visible set

### Camera

When the user changes the scope:

1. rebuild the scene
2. recompute extents
3. reset to a fit view once

Ordinary selection and browsing should not keep resetting the camera.

## Testing Plan

Keep tests focused on pure TypeScript logic only.

Must cover:

- `active-floor-only` returns only active-level spaces
- `all-levels` returns spaces from multiple levels
- extents expand correctly in `all-levels`
- selected-space emphasis still wins over active-level emphasis
- `edgeVertices` are generated for visible spaces
- empty visible sets stay safe

No Rust renderer tests are required for this task.

## Verification Plan

When implemented, run:

```bash
corepack pnpm --filter web test
corepack pnpm --filter web build
corepack pnpm run build:wasm
```

Manual smoke:

1. open the editor and switch to `3D View`
2. confirm the default scope is `Active Floor Only`
3. switch to `All Levels` and confirm spaces from other levels appear
4. confirm selected-space and active-level emphasis still read correctly
5. confirm edges make neighboring spaces easier to distinguish
6. confirm changing scope refits the camera once
7. confirm ordinary selection changes do not keep resetting the camera
8. confirm the empty state suggests `All Levels` when only the current floor is empty

## Implementation Status

Implemented in:

- `apps/web/src/editor-shell.tsx`
- `apps/web/src/space-scene.ts`
- `apps/web/src/three-d-viewport.tsx`
- `apps/web/src/styles.css`
- `crates/render-wasm/src/lib.rs`

Pure logic coverage updated in:

- `apps/web/src/space-scene.test.ts`

## Done Criteria

This task is complete when:

1. the 3D view exposes the two requested visibility modes
2. `Active Floor Only` remains the default behavior
3. `All Levels` shows the full visible stack across levels
4. colored edges make each visible space easier to read
5. the TypeScript-to-wasm contract stays lean by keeping `vertices` and adding only `edgeVertices`
6. Rust remains a thin renderer, not a second domain model
7. the web test/build and wasm build checks pass
