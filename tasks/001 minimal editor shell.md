# 001 Minimal Editor Shell

## Status

Implemented on `2026-03-24`.

Build status:

- `corepack pnpm --filter web build` passed

## Original Task

In `apps/web`, build a minimal editor shell with:

- Ribbon bar: top bar
- Properties bar: left side
- center area for workspace showing `3D View` and `Floor Plan View`
- Project Browser
- bottom status bar

Constraints:

- plain React + CSS only
- no Tailwind
- no component library
- use a very small Zustand store only for UI/editor session state
- mock data is acceptable
- resize behavior must be correct
- optimize for minimal code and low abstraction count

Reference layout direction:

- `C:\Users\folky\Documents\GitHub\GLOQ3D`

## Current Implementation Context

The task is now implemented as a lean single-page shell in `apps/web`.

Source of truth:

- `apps/web/src/App.tsx`
  - renders the full shell
  - derives all display data from `project-doc.ts`
- `apps/web/src/ui-store.ts`
  - owns the small UI/session store
- `apps/web/src/styles.css`
  - owns the desktop shell layout, panel chrome, overflow rules, and breakpoints
- `apps/web/src/project-doc.ts`
  - remains the mock domain data source
- `apps/web/package.json`
  - now includes `zustand`

## What Was Shipped

### Shell Regions

The page now renders these 5 required regions:

- top `Ribbon`
- left sidebar with `Tools` and `Properties`
- center `Workspace`
- right `Project Browser`
- bottom `Status Bar`

### Zustand Store

Only one new UI store was added.

Actual store shape:

```ts
type ViewMode = "3d" | "plan";
type ToolMode = "select" | "space" | "level";
type Selection =
  | { kind: "view"; id: "view-3d" | "view-plan" }
  | { kind: "level"; id: string }
  | { kind: "space"; id: string }
  | null;
```

Actual store fields:

- `activeView`
- `activeTool`
- `selection`

Actual actions:

- `setActiveView`
- `setActiveTool`
- `setSelection`

Not implemented in store:

- document data
- geometry state
- persistence
- async logic
- browser open/close state

## Actual UI Behavior

### Ribbon

Implemented groups:

- `File`: `New`, `Save`
- `Edit`: `Undo`, `Redo`
- `View`: `3D`, `Plan`

Behavior:

- `3D` and `Plan` switch the active workspace view
- `New`, `Save`, `Undo`, and `Redo` are presentational placeholders

### Left Sidebar

Implemented sections:

- `Tools`
  - `Select`
  - `Space`
  - `Level`
- `Properties`
  - `Session`
  - `Selection`

Behavior:

- tool buttons update `activeTool`
- properties content is derived from current selection and current view

### Center Workspace

Implemented tabs:

- `3D View`
- `<Active Level> Floor Plan`

Behavior:

- one active view at a time
- `3D View` is a styled placeholder viewport
- `Floor Plan` is a 2D mock plan based on current spaces from `project-doc.ts`
- clicking a plan space selects that space and keeps the active view on plan

### Project Browser

Implemented groups:

- `Views`
- `Levels`
- `Spaces`

Behavior:

- clicking a view row switches the active view
- clicking a level row selects the level
- clicking a space row selects the space
- selected rows get active styling

### Status Bar

The status bar currently shows:

- units
- active level
- active view
- active tool
- selection summary

## Mock Data And Derivation

Document source:

- `createStarterProjectDoc()` from `apps/web/src/project-doc.ts`

Derived in `App.tsx`:

- `activeLevel`
- `activeSpaces`
- `grossArea`
- `currentViewLabel`
- `selectionLabel`
- plan canvas size
- selection detail rows for the properties panel

This keeps TypeScript as the canonical UI/document layer and avoids duplicating a second schema.

## Layout And Resize Context

### Desktop Layout

Implemented as CSS Grid:

- app shell rows: `48px minmax(0, 1fr) 28px`
- main shell columns: `240px minmax(0, 1fr) 280px`
- workspace rows: `40px minmax(0, 1fr)`

### Responsive Breakpoints

Implemented breakpoints in `apps/web/src/styles.css`:

- `max-width: 1200px`
  - main columns become `220px minmax(0, 1fr) 240px`
- `max-width: 900px`
  - right browser moves below the left + center layout
- `max-width: 720px`
  - layout becomes one column
  - left tool buttons become a 3-column horizontal row

### Overflow Rules Implemented

- `html`, `body`, and `#root` are full height
- `body` uses `overflow: hidden`
- shell panels use `min-width: 0` and `min-height: 0` where needed
- internal scroll is allowed inside:
  - properties panel
  - project browser
  - plan canvas area
  - ribbon/status rows when content is tight

## Visual Direction That Landed

The shipped look is intentionally simplified but aligned with the reference direction:

- dense desktop-app chrome
- gray panel surfaces
- compact button treatments
- center viewport emphasis
- no marketing-page styling remains

## Files Changed For This Task

- `apps/web/package.json`
- `apps/web/src/App.tsx`
- `apps/web/src/styles.css`
- `apps/web/src/ui-store.ts`
- `pnpm-lock.yaml`

## Verification Run

Command run:

```bash
corepack pnpm --filter web build
```

Result:

- passed

## Known Limits

This task intentionally does not implement:

- real WebGPU canvas
- real editing commands
- drag-resizable panes
- browser search
- browser tree nesting
- persistent UI preferences
- undo/redo behavior
- Supabase or API wiring

## Remaining TODOs

- manual browser QA for resize behavior at `1280px`, `1024px`, `900px`, and `720px`
- replace the 3D placeholder with the Rust WebGPU/WASM renderer in a later task
- evolve the floor plan from mock clickable blocks into real authoring behavior in a later task
