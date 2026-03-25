# 009 3D Space View

## Goal

In `apps/web` and `crates/render-wasm`, replace the current 3D placeholder with a real WebGPU-backed 3D viewport that renders `Space` volumes from the canonical `ProjectDoc`.

For this MVP, "simulate spaces in 3D" means:

- take each rectangular `Space` footprint from TypeScript
- extrude it to the owning level height
- place it at the owning level elevation
- render it as a simple shaded prism in the 3D view

The result should be a lean, inspectable 3D massing view, not a second authored BIM model.

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

### 2. Depend on the active-level seam from task `008`, or land the minimum subset inline

This task should consume one canonical `activeLevelId` if task `008` is already landed.

If `008` is not landed yet, add only the minimum local session state needed for:

- active-level highlight
- view focus
- consistent plan/3D context

Do not create a separate 3D-only active level concept.

Reason:

- plan and 3D should still read from the same editing context

### 3. TypeScript owns scene derivation; Rust wasm owns drawing only

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

### 4. Real 3D should render the stacked model and highlight the active level

Render spaces across all levels in the MVP 3D view.

Visibility behavior:

- active level: fully emphasized
- selected space: strongest emphasis
- other levels: still visible but dimmed

Do not hard-filter the entire 3D viewport to one level by default.

Reason:

- once 3D geometry exists, hiding every non-active level removes most of the spatial value
- highlight is enough for context without introducing a second saved-view system

### 5. Keep the wasm API flat and serializable

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

### 6. Start with one lean camera model

Use one perspective camera with transient session state.

Support only:

- left drag to orbit
- mouse wheel to zoom
- modified drag for pan
- reset-to-fit on first scene load

Do not add:

- saved camera presets
- orthographic mode switching
- animation timelines

Reason:

- this is enough to inspect space volumes without bloating shell state

### 7. Use a dedicated viewport component and keep failure states explicit

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

## File Plan

### 1. `apps/web/src/project-doc.ts`

Keep the canonical document types unchanged unless a tiny helper is needed for cleaner level lookup reuse.

Do not add 3D-only authored fields to `ProjectDoc`.

### 2. `apps/web/src/space-scene.ts`

Add pure helpers for:

- converting levels and spaces into prism render items
- deriving scene extents
- assigning emphasis state from active level and selection
- computing an initial fit-to-scene camera target

### 3. `apps/web/src/space-scene.test.ts`

Add tests for:

- prism height from level height
- prism elevation from level elevation
- active-level highlighting
- selected-space highlighting
- scene extents across multiple levels

### 4. `apps/web/src/three-d-viewport.tsx`

Add one React wrapper that:

- mounts the canvas
- initializes wasm on demand
- uploads scene payload changes
- translates pointer input into camera updates
- handles resize and fallback UI

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

If needed, add the smallest config change required to import the generated wasm package cleanly from the web app.

Possible needs:

- alias for the generated package path
- `server.fs.allow` for the workspace seam

### 8. `crates/render-wasm/src/lib.rs`

Expand the wasm crate to support:

- canvas-backed renderer creation
- scene upload
- camera update
- resize handling
- one draw path for simple space prisms plus a ground/grid reference

Keep the renderer focused on draw concerns only.

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
- a ground or grid reference
- active-level emphasis
- selected-space emphasis when selection comes from the browser or plan view

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

## Testing Plan

Add tests only for pure TypeScript scene derivation.

Must cover:

- one space on one level becomes one prism with the correct base and height
- multiple levels produce stacked `z` extents
- active level changes emphasis state without changing geometry
- selected space gets stronger emphasis than unselected spaces
- empty-space input yields an empty scene safely

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

## Done Criteria

This task is complete when:

1. the `3D View` tab mounts a real wasm/WebGPU-backed canvas
2. spaces are visible in 3D as simple level-height prisms
3. the scene is derived from the canonical TypeScript document only
4. Rust remains a thin renderer, not a second domain model
5. active level and selection affect 3D emphasis consistently
6. the viewport handles loading, unsupported, empty, and error states explicitly
7. only pure TypeScript scene logic gets tests
8. the web test/build and wasm build checks pass
