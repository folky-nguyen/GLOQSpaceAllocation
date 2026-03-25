# 009 3D Space View

## Goal

In `apps/web` and `crates/render-wasm`, replace the current 3D placeholder with a real WebGPU-backed 3D viewport that renders `Space` volumes from the canonical `ProjectDoc`.

For this MVP, "simulate spaces in 3D" means:

- take each rectangular `Space` footprint from TypeScript
- extrude it to the owning level height
- place it at the owning level elevation
- render it as a simple shaded prism in the 3D view

The result should be a lean, inspectable 3D massing view, not a second authored BIM model.

This task should bias toward the smallest shippable implementation:

- reuse current shell state and selection state
- keep new files to a minimum
- avoid general-purpose renderer abstractions
- accept full-scene rebuilds when the scene changes

## Current Repo State

`apps/web` currently has:

- `src/editor-shell.tsx`
  - one working plan viewport driven by TypeScript document data
  - one `3D View` tab that is still a static placeholder shell
  - no canvas mount, no wasm renderer lifecycle, and no camera controls
  - active level still inferred from selection in the current code path
- `src/ui-store.ts`
  - `activeView`, `activeTool`, and `selection`
  - no dedicated 3D viewport session state
- `src/project-doc.ts` in the mainline repo shape
  - canonical `ProjectDoc`, `Level`, and `Space`
  - enough level and space data to derive 3D prisms without adding a new model

`crates/render-wasm` currently has:

- `src/lib.rs`
  - `probe_webgpu()` only
  - no surface setup, no scene upload, no camera state, and no render loop

What is missing today:

- no real 3D canvas inside the editor
- no TypeScript-to-wasm render payload seam
- no derived 3D scene model for spaces
- no orbit/pan/zoom controls
- no renderer fallback state when WebGPU init fails

## Scope

In scope:

- one real 3D viewport canvas in the existing editor shell
- wasm renderer bootstrap from React
- one pure TypeScript scene builder derived from `ProjectDoc`
- one simple prism per space using level elevation plus level height
- basic camera interaction:
  - orbit
  - zoom
  - pan
- active-level and selection highlighting
- empty, loading, unsupported, and error states for the 3D panel
- tests only for pure TypeScript scene-derivation logic

Out of scope:

- walls, doors, windows, slabs, or any non-space BIM solids
- 3D editing tools
- GPU picking or direct 3D click-to-select
- saved cameras, section boxes, clipping planes, shadows, or lighting polish
- API changes
- Supabase schema changes
- a second domain model in Rust

## Lean Constraints

This task should explicitly optimize for code economy.

Apply these rules:

- reuse `editor-shell.tsx` and `ui-store.ts` instead of adding a new global store
- prefer one new pure helper file plus one new viewport component on the web side
- keep Rust changes in `crates/render-wasm/src/lib.rs` unless the file becomes unreasonably hard to read
- do not add a scene graph, ECS, or renderer plugin system
- do not add new npm packages or Rust crates unless blocked
- do not add incremental scene diffing; rebuild the derived scene payload on each document or selection change
- do not run a permanent animation loop; redraw only when needed

## Implementation Decisions

### 1. Treat each space as a level-height prism, not as a new authored 3D entity

Derive 3D geometry from existing document data:

- base `z` from `level.elevationFt`
- height from `level.heightFt`
- `x`, `y`, `width`, and `depth` from the existing `Space`

Do not add:

- `SpaceVolume`
- a separate 3D-authored schema
- persisted mesh data

Reason:

- the shared TypeScript document must stay canonical
- current level and space fields are already sufficient for an MVP massing view

### 2. Use one explicit world-coordinate convention

To remove ambiguity between plan coordinates and 3D coordinates, use:

- `x`: `Space.xFt`
- `y`: `Space.yFt`
- `z`: vertical elevation
- `+z` is up
- prism width along `x`
- prism depth along `y`
- prism height along `z`

For one space:

- `minX = space.xFt`
- `minY = space.yFt`
- `minZ = level.elevationFt`
- `sizeX = space.widthFt`
- `sizeY = space.depthFt`
- `sizeZ = level.heightFt`

Reason:

- this keeps the 3D payload aligned with the existing plan model
- it avoids introducing coordinate remapping logic unless the renderer truly needs it

### 3. Depend on the active-level seam from task `008`, or land the minimum subset inline

This task should consume one canonical `activeLevelId` if task `008` is already landed.

If `008` is not landed yet, add only the minimum local session state needed for:

- active-level highlight
- view focus
- consistent plan/3D context

Do not create a separate 3D-only active level concept.

Reason:

- plan and 3D should still read from the same editing context

### 4. TypeScript owns scene derivation; Rust wasm owns drawing only

Add one pure TypeScript builder that converts the canonical document into a flat render payload.

Suggested surface:

```ts
export type SpacePrismRenderItem = {
  id: string;
  levelId: string;
  name: string;
  minXFt: number;
  minYFt: number;
  minZFt: number;
  sizeXFt: number;
  sizeYFt: number;
  sizeZFt: number;
  emphasis: "normal" | "active-level" | "selected";
};

export type SpaceScenePayload = {
  items: SpacePrismRenderItem[];
  extents: {
    minXFt: number;
    minYFt: number;
    minZFt: number;
    maxXFt: number;
    maxYFt: number;
    maxZFt: number;
  };
  hasVisibleItems: boolean;
};

export function buildSpaceScenePayload(
  doc: ProjectDoc,
  input: { activeLevelId: string | null; selection: Selection }
): SpaceScenePayload;
```

Rust should receive only derived scene data plus camera input.

Reason:

- TypeScript owns document and geometry logic
- Rust should not become a second owner of authored model state

### 5. Rebuild the whole derived scene when the model context changes

When any of these inputs change:

- `project`
- `activeLevelId`
- `selection`

recompute the full `SpaceScenePayload` in TypeScript and upload it again.

Do not implement:

- per-space patching
- partial GPU updates by semantic diff
- a second cached scene model in React state

Reason:

- the MVP scene is small
- full rebuilds keep the implementation understandable
- this is the lowest-risk way to keep TypeScript authoritative

### 6. Real 3D should render the stacked model and highlight the active level

Render spaces across all levels in the MVP 3D view.

Visibility behavior:

- active level: fully emphasized
- selected space: strongest emphasis
- other levels: still visible but dimmed

Do not hard-filter the entire 3D viewport to one level by default.

Reason:

- once 3D geometry exists, hiding every non-active level removes most of the spatial value
- highlight is enough for context without introducing a second saved-view system

### 7. Use the smallest wasm API that still keeps React out of GPU details

Expose a minimal renderer handle from Rust.

Suggested surface:

```rust
#[wasm_bindgen]
pub async fn create_renderer(canvas: HtmlCanvasElement) -> Result<RendererHandle, JsValue>;

#[wasm_bindgen]
impl RendererHandle {
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), JsValue>;
    pub fn set_scene(&mut self, scene: JsValue) -> Result<(), JsValue>;
    pub fn set_camera(&mut self, camera: JsValue) -> Result<(), JsValue>;
    pub fn render(&mut self) -> Result<(), JsValue>;
}
```

Reason:

- React should not know GPU details
- wasm should not know editor semantics beyond the derived payload
- this is small enough to implement without creating a second abstraction layer

Implementation note:

- `set_scene(...)` may accept the full scene every time it changes
- `set_camera(...)` may accept the full camera every time the user interacts
- `render()` should draw one frame only

### 8. Keep geometry upload simple even if it is not the most optimized path

For the MVP, Rust may rebuild a flat triangle buffer from the full scene payload whenever `set_scene(...)` runs.

One prism can be expanded to:

- 8 corners
- 12 triangles
- solid color per prism based on emphasis

Do not implement yet:

- instancing
- shared mesh plus per-instance transform buffers
- indirect draw
- GPU-side picking IDs

Reason:

- the number of spaces in the starter MVP is small
- this avoids a large amount of pipeline and buffer-management code

### 9. Do not run a continuous render loop

Render only when one of these events happens:

- renderer initialized
- canvas resized
- scene payload changed
- camera changed from user input
- fit/reset requested

Do not run `requestAnimationFrame` continuously while the scene is idle.

Reason:

- lower code volume
- lower browser overhead
- easier lifecycle management from React

### 10. Use one explicit camera interaction contract

Use only:

- left drag: orbit
- `Shift` + left drag: pan
- mouse wheel: zoom
- one small `Fit` overlay button: reset the camera to the current scene extents

Do not add:

- keyboard navigation
- touch gestures
- saved cameras
- orthographic mode

Reason:

- this is enough to inspect the model
- the interaction contract is unambiguous
- one button is simpler than managing extra shortcuts

### 11. Fit the camera from scene extents in TypeScript, not in Rust

Add one pure helper that computes the initial camera from `SpaceScenePayload.extents`.

Rules:

- target = extents center
- orbit yaw/pitch can be fixed defaults
- distance is based on the largest scene dimension plus a small margin
- empty scene returns a safe default camera

Reason:

- camera-fit logic is derived view logic, not GPU logic
- this keeps Rust limited to drawing

### 12. Use a dedicated viewport component and keep failure states explicit

Add one small React component for the 3D viewport lifecycle.

That component should own:

- canvas ref
- wasm init state
- resize observer
- transient camera/session state
- fallback overlays

`editor-shell.tsx` should keep owning:

- project data
- active view
- selection
- active level context

Reason:

- this keeps React orchestration local without pushing renderer details into the shell
- it also prevents wasm lifecycle code from leaking across the editor shell

### 13. Prefer a direct package import before touching Vite config

First try importing the generated wasm package directly from the existing generated output path.

Only change `apps/web/vite.config.ts` if that import is actually blocked.

Reason:

- code economy
- avoid config churn for a seam that may already work

## File Plan

### 1. `apps/web/src/project-doc.ts`

Keep the canonical document types unchanged unless a tiny helper is needed for cleaner level lookup reuse.

Do not add 3D-only authored fields to `ProjectDoc`.

### 2. `apps/web/src/space-scene.ts`

Add only the pure helpers needed for:

- converting levels and spaces into prism render items
- deriving scene extents
- assigning emphasis state from active level and selection
- computing an initial fit-to-scene camera target

Suggested surface:

```ts
export type OrbitCamera = {
  targetXFt: number;
  targetYFt: number;
  targetZFt: number;
  distanceFt: number;
  yawDeg: number;
  pitchDeg: number;
};

export function buildSpaceScenePayload(
  doc: ProjectDoc,
  input: { activeLevelId: string | null; selection: Selection }
): SpaceScenePayload;

export function getDefaultOrbitCamera(scene: SpaceScenePayload): OrbitCamera;
```

### 3. `apps/web/src/space-scene.test.ts`

Add tests for:

- prism height from level height
- prism elevation from level elevation
- negative level elevation
- active-level highlighting
- selected-space highlighting
- scene extents across multiple levels
- empty-scene camera fallback

### 4. `apps/web/src/three-d-viewport.tsx`

Add one React wrapper that:

- mounts the canvas
- initializes wasm on demand
- uploads scene payload changes
- translates pointer input into camera updates
- handles resize and fallback UI

Keep this component lean:

- no custom hooks unless repetition forces one
- no global store writes for camera state
- no child component tree for overlays if plain JSX in one file is enough

### 5. `apps/web/src/editor-shell.tsx`

Replace the placeholder 3D panel with `ThreeDViewport`.

Keep:

- existing shell labels and browser coordination
- selection-driven emphasis updates
- status and property text aligned with the 3D state

### 6. `apps/web/src/styles.css`

Add only the styles needed for:

- 3D canvas sizing
- loading and fallback overlays
- camera cursor states
- compact renderer status text

### 7. `apps/web/vite.config.ts`

Only if needed, add the smallest config change required to import the generated wasm package cleanly from the web app.

Possible needs:

- alias for the generated package path
- `server.fs.allow` for the workspace seam

Default preference:

- no config change at all

### 8. `crates/render-wasm/src/lib.rs`

Expand the wasm crate to support:

- canvas-backed renderer creation
- scene upload
- camera update
- resize handling
- one draw path for simple space prisms
- optional simple ground plane only if it fits into the same draw path cheaply

Keep the renderer focused on draw concerns only.

Keep the first pass small:

- one render pipeline
- one depth buffer
- one solid-color fragment path
- no text
- no outlines unless they are nearly free

## UI Behavior

### 3D Viewport States

The 3D panel should show one of:

1. loading renderer
2. unsupported WebGPU/browser state
3. renderer init error
4. empty model state
5. live 3D scene

### Scene Presentation

The live scene should include:

- simple colored or tinted space prisms
- active-level emphasis
- selected-space emphasis when selection comes from the browser or plan view

Optional:

- one simple ground plane if it does not require a second major rendering system

### Interaction

When the user switches to `3D View`:

- the viewport initializes lazily
- the camera fits the current model once
- later document or selection updates do not wipe the current camera unless the scene becomes empty

### Selection And View Parity

For the MVP:

- selecting a space from plan or browser updates 3D highlight
- selecting a level updates level emphasis
- direct picking inside 3D is not required

If a level is active but no specific space is selected:

- all spaces on the active level use `active-level` emphasis
- spaces on other levels use `normal` emphasis

If a space is selected:

- that space uses `selected` emphasis
- its level still counts as active for the other spaces on that level

## Testing Plan

Add tests only for pure TypeScript scene derivation.

Must cover:

- one space on one level becomes one prism with the correct base and height
- multiple levels produce stacked `z` extents
- active level changes emphasis state without changing geometry
- selected space gets stronger emphasis than unselected spaces
- empty-space input yields an empty scene safely
- default camera fit returns a usable fallback for an empty scene

No Rust renderer tests are required for this task.

## Verification Plan

When implemented, run:

```bash
corepack pnpm --filter web test
corepack pnpm --filter web build
corepack pnpm run build:wasm
```

Manual smoke:

1. open the editor
2. switch to `3D View`
3. confirm the placeholder is replaced by a live canvas or a clear unsupported-state message
4. orbit, pan, and zoom the scene
5. confirm each space appears as one box
6. confirm boxes use the owning level elevation and height
7. select a space from plan or browser and confirm 3D emphasis updates
8. switch active level and confirm level emphasis updates
9. resize the window and confirm the canvas redraws correctly
10. confirm the app still returns to plan view without view-state drift
11. leave the scene idle and confirm there is no visible continuous redraw behavior or runaway CPU use

## Done Criteria

This task is complete when:

1. the `3D View` tab mounts a real wasm/WebGPU-backed canvas
2. spaces are visible in 3D as simple level-height prisms
3. the scene is derived from the canonical TypeScript document only
4. Rust remains a thin renderer, not a second domain model
5. active level and selection affect 3D emphasis consistently
6. the viewport handles loading, unsupported, empty, and error states explicitly
7. only pure TypeScript scene logic gets tests
8. the implementation does not introduce a scene graph, global 3D store, or continuous idle render loop
9. the web test/build and wasm build checks pass
