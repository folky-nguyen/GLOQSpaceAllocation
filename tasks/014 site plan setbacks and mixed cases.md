# 014 Site Plan Setbacks And Mixed Cases

This task refines the sample-case direction from `012.01 draggable test dashboard and polygon apartment cases.md`.

The repo should stop treating `Level`, `Space`, and `Mixed` as three separate validation tracks.

From this task onward, the checked-in validation surface should collapse into `3` mixed cases that each combine:

- level stack
- site polygon shape
- setback offsets
- building footprint
- space layout

## Goal

Add one lean `Site Plan` view derived from the `Level 1` floor plan context.

Each mixed case should define one site boundary as a simple polygon in decimal feet.

The user must be able to:

1. open `Site Plan`
2. click one site-boundary edge
3. edit that edge's setback offset in imperial UI input
4. start from a default-filled `5 ft`
5. see the derived building footprint update from the new offset
6. see the case spaces drawn inside that building footprint

Keep only `3` mixed sample cases, each with a different:

- level configuration
- site shape
- space arrangement

Delete the extra sample data and UI paths that only exist to separately validate levels or space layouts.

## Current Repo State

`apps/web` currently has:

- `src/ui-store.ts`
  - only `plan` and `3d` view modes
  - no site-plan state or site-edge selection
- `src/editor-shell.tsx`
  - renders the active level floor plan as one SVG of space polygons
  - mounts `TestDashboard`
  - has no site polygon, setback controls, or building-footprint overlay
- `src/test-dashboard.tsx`
  - still exposes separate `Level` and `Space` selectors plus `Mixed Cases`
- `src/test-cases.ts`
  - still imports and exports `LEVEL_CASES`, `SPACE_CASES`, and `MIXED_CASES`
- `src/project-doc.ts`
  - canonical `ProjectDoc` still contains only `levels` and `spaces`
  - has no site-boundary or setback ownership

`supabase/sample-data` currently has:

- `levels/*.json`
- `spaces/*.json`
- `mixed/*.json`

What is missing today:

- no canonical site polygon in `ProjectDoc`
- no derived building-footprint helper from per-edge setbacks
- no `Site Plan` view
- no site-edge selection model
- no setback editing UI
- no reduced mixed-only sample-case workflow

## Scope

In scope:

- one `Site Plan` view tied to one host level
- one canonical site boundary polygon in `ProjectDoc`
- one setback offset value per site edge
- one derived building-footprint polygon
- rendering the site boundary, setback result, and level-hosted spaces in `Site Plan`
- only `3` mixed cases under `supabase/sample-data/mixed/`
- deleting unused `levels` and `spaces` sample fixtures
- removing separate `Level` and `Space` selectors from the test dashboard
- pure TypeScript geometry tests for site/setback helpers

Out of scope:

- freehand site-boundary authoring tools
- dragging edges with gizmos
- multiple site plans per project
- persisted user preferences for the last selected site edge
- runtime validation wizards for level-only or layout-only scenarios
- Rust renderer changes unless later work proves a real 3D site need

## Implementation Decisions

### 1. Add one canonical site-plan object to `ProjectDoc`

Keep the document lean and TypeScript-owned.

Suggested shape:

```ts
type SitePlan = {
  levelId: string;
  boundary: Point2Ft[];
  edgeSetbacksFt: number[];
};

type ProjectDoc = {
  id: string;
  name: string;
  defaultStoryHeightFt: number;
  levels: Level[];
  spaces: Space[];
  sitePlan: SitePlan | null;
};
```

Rules:

- `levelId` points to the host level for the site plan, expected to be `Level 1` in the first pass
- `boundary` is one simple non-self-intersecting polygon
- `edgeSetbacksFt[index]` applies to the edge from `boundary[index]` to the next point
- all values stay in decimal feet

Reason:

- site geometry must stay in the same TypeScript-owned document as levels and spaces
- edge-index-based offsets are leaner than inventing a second edge-id model when the boundary itself is not being edited yet

### 2. Keep building footprint derived, not authored separately

Do not persist a second authored polygon for the building footprint.

Derive it from:

- normalized site boundary
- per-edge setback offsets

TypeScript should own:

- boundary normalization
- inward offset-line math
- edge intersection solving
- invalid-offset detection
- final footprint polygon generation

Reason:

- this avoids a duplicate source of truth
- it keeps geometry logic out of Rust and out of sample-data duplication

### 3. Keep the first site shapes simple and deterministic

The first implementation should support simple polygon sites chosen to keep inward offsets reliable.

Fixture rule:

- use `3` different site shapes
- keep every site polygon simple and non-self-intersecting
- prefer convex or only gently irregular outlines in v1 so per-edge inward offsets stay stable

Reason:

- the task is about a lean MVP site-plan workflow, not a full parcel-geometry engine

### 4. Add `Site Plan` as a third view mode

Extend the existing view seam instead of creating a second editor surface.

Target:

```ts
type ViewMode = "site-plan" | "plan" | "3d";
```

Behavior:

- `Site Plan` always renders the host level defined by `project.sitePlan.levelId`
- `Floor Plan` keeps the current active-level behavior
- `3D View` remains unchanged for this task

Reason:

- the current workspace already switches views cleanly
- this keeps file and abstraction count low

### 5. Add one lean site-edge selection state

When the user clicks a site boundary edge, selection should move to that edge and the property panel should expose one setback input.

Suggested selection extension:

```ts
type Selection =
  | { kind: "view"; id: "view-site-plan" | "view-plan" | "view-3d" }
  | { kind: "site-edge"; edgeIndex: number }
  | ...
```

Behavior:

- selecting a site edge pre-fills the setback field with that edge's current value
- new sample cases start every edge at `5 ft` unless the fixture needs a deliberate variation
- editing the input recomputes the derived footprint immediately

Reason:

- click-to-edit matches the requested interaction
- it reuses the existing selection + properties pattern instead of introducing a floating inspector

### 6. Keep site-plan rendering in the current SVG plan seam

Do not add a second canvas or graphics library.

`Site Plan` should reuse one SVG-based 2D rendering path for:

- the site boundary polygon
- highlighted selected edge
- the derived building footprint polygon
- the host-level spaces inside that footprint
- labels only where they stay readable

Reason:

- the repo already renders polygon spaces through SVG in `editor-shell.tsx`
- reusing that seam is the smallest local diff

### 7. Collapse test data to mixed cases only

Remove the level-only and space-only sample workflows.

Keep:

- `3` mixed cases

Delete:

- `supabase/sample-data/levels/*.json`
- `supabase/sample-data/spaces/*.json`
- the `LEVEL_CASES` manifest path
- the `SPACE_CASES` manifest path
- the separate `Level` and `Space` dropdowns in the dashboard

Reason:

- the user no longer wants validation split by dimension
- each scenario should now be evaluated as one complete site + level + layout package

### 8. Keep spaces authored directly and positioned within the derived footprint

Do not invent a generative packing engine.

For each mixed fixture:

- author the `Space` polygons directly
- make them fit inside the derived building footprint
- keep them attached to real levels as the canonical document already expects

Reason:

- the user asked to draw spaces inside the footprint, not to auto-solve layouts
- checked-in sample documents are enough for the first validation loop

### 9. Use existing imperial parsing at the UI boundary

The setback input should accept the same feet-inch entry style already used elsewhere in the editor.

Rules:

- display current setback with the existing feet-inch formatter
- parse edits with the existing feet-inch parser
- store only decimal feet in `ProjectDoc`

Reason:

- this preserves the repo unit boundary contract

### 10. Keep site-plan interaction narrower than floor-plan interaction

To keep the diff small, `Site Plan` should not try to support every existing plan-selection behavior.

Lean rule:

- `Floor Plan` remains the primary place for selecting spaces
- `Site Plan` only needs reliable site-edge selection for setback editing
- spaces rendered in `Site Plan` may stay read-only in the first pass
- do not add sweep-select behavior for site edges

Reason:

- the user request is centered on editing setbacks, not duplicating the whole plan-selection stack in another view
- reusing all floor-plan interactions in `Site Plan` would add state branching with little product value

### 11. Keep spaces static when setbacks change

Do not auto-regenerate, auto-pack, or auto-trim spaces after a setback edit.

Behavior:

- the building footprint is the derived geometry that updates live
- the sample-case spaces remain authored polygons
- fixtures should start with spaces already fitting inside the default `5 ft` footprint
- if a later manual setback edit makes the footprint smaller than some spaces, show that visually and do not attempt repair logic

Reason:

- this keeps TypeScript geometry ownership lean
- it avoids inventing a second layout engine just to respond to setback edits

### 12. Normalize polygon winding before any setback math

Per-edge inward offsets depend on consistent polygon direction.

Lean rule:

- normalize the site boundary before deriving edges
- if the boundary is clockwise, reverse it once before computation
- reverse `edgeSetbacksFt` in the same pass so each setback still matches its intended edge
- perform inward offset math only on the normalized orientation

Reason:

- inconsistent winding is the fastest path to “offset goes outward instead of inward” bugs
- one normalization step is cheaper than sprinkling orientation checks throughout the code

### 13. Repair incomplete site data at load time instead of spreading guards everywhere

The first line of defense should be one small normalization helper in `project-doc.ts`.

Rules:

- if `sitePlan` is missing, treat the project as having no site plan
- if `edgeSetbacksFt.length` does not match the boundary edge count, repair it to a full array
- fill missing or invalid values with `5`
- clamp non-finite or negative setback values to `0`

Reason:

- one repair seam keeps the rest of the UI simple
- this is lower-code than adding repeated null and length checks throughout the editor

## Leanest Implementation Path

The implementation should favor the fewest file changes and the fewest new concepts.

Rules:

- keep all site/setback geometry helpers in `apps/web/src/project-doc.ts` first
- keep `Site Plan` rendering in `apps/web/src/editor-shell.tsx`
- keep the dashboard as the same component instead of introducing a second mixed-case launcher
- do not add a new store slice; extend the current `ui-store.ts` union types only
- do not touch `apps/web/src/three-d-viewport.tsx`, `apps/web/src/space-scene.ts`, or `crates/render-wasm/src/lib.rs` unless a regression is proven
- do not add a new sample-data manifest schema; extend the existing mixed-case manifest only

Preferred outcome:

- one new canonical field in `ProjectDoc`
- one new view mode
- one new selection kind
- one geometry-helper cluster
- one dashboard simplification

Anything beyond that needs evidence.

## Exact Data Rules

These rules close the main ambiguities in the current note.

### 1. `sitePlan` stays optional during the transition

Use:

```ts
type ProjectDoc = {
  ...
  sitePlan?: SitePlan | null;
};
```

Transition rule:

- `createStarterProjectDoc()` should continue to work with `sitePlan: null`
- old documents without `sitePlan` must not crash loading paths
- `Site Plan` UI should hide itself or show a clear empty state when `sitePlan` is absent

Reason:

- optionality makes the migration tolerant while the old fixtures are being replaced

### 2. Site coordinates and floor-plan coordinates are the same coordinate system

To avoid a second transform model:

- site boundary
- derived building footprint
- host-level spaces

must all use the same decimal-foot XY coordinates.

That means:

- `Site Plan` is not a transformed overview of a separate local floor-plan origin
- sample spaces for the host level should already be authored in site/world coordinates
- no site-to-building transform matrix is needed in v1

Reason:

- one shared coordinate system is the lowest-code option
- this keeps `Floor Plan` and `Site Plan` visually consistent by construction

### 3. Building-footprint failure should produce `null`, not partial garbage

If setback math fails, prefer one explicit invalid result:

```ts
type DerivedFootprintResult =
  | { footprint: Point2Ft[]; error: null }
  | { footprint: null; error: string };
```

Rules:

- do not emit NaN points
- do not emit a self-intersecting best guess
- keep the site boundary visible even when the footprint is invalid

Reason:

- one explicit failure path is easier to render and test than half-valid geometry

## Transition Order

To reduce breakage during implementation, land the change in this order.

### Step 1. Expand the document model first

Update `project-doc.ts` and its tests before any UI work:

- add `SitePlan`
- add load-time normalization and repair helpers
- add derived-footprint helpers
- keep `createStarterProjectDoc()` returning a valid document

This step should compile without touching the renderer.

### Step 2. Add the new mixed fixtures before deleting old imports

Create the new `mixed/*.json` files first.

Only after they exist:

- refactor `test-cases.ts` to import mixed-only fixtures
- remove `LEVEL_CASES`
- remove `SPACE_CASES`

Reason:

- deleting the old fixtures first would break current raw imports immediately

### Step 3. Extend `ui-store.ts`

After the new data shape exists:

- add `site-plan` to `ViewMode`
- add `view-site-plan` and `site-edge` to `Selection`
- keep the default session state on `plan`

Reason:

- the current app still needs to boot cleanly before any sample case is loaded

### Step 4. Update sample-case load behavior

`handleLoadSampleCase` in `editor-shell.tsx` should then be updated to:

- accept `preferredView: "site-plan" | "plan" | "3d"`
- keep `preferredActiveLevelId`
- reset local site-edge selection when a new case loads
- reset `threeDVisibilityMode` only on full document replacement, consistent with `013.01`

Reason:

- this is where view-id mismatches are most likely to surface during the transition

### Step 5. Add `Site Plan` rendering last

Once the data and selection seams are stable:

- render site boundary
- render selected edge
- render derived footprint
- render host-level spaces
- add the setback input

Reason:

- this keeps the visible UI change as the last step, after the underlying types are safe

### Step 6. Delete old sample folders only after the mixed-only dashboard builds

Do the cleanup last:

- remove `supabase/sample-data/levels/*.json`
- remove `supabase/sample-data/spaces/*.json`
- update `supabase/sample-data/README.md`

Reason:

- this prevents ending up in a half-migrated state where the dashboard still references deleted assets

## Risk Register

These are the main failure modes to watch during the conversion.

### 1. View-id mismatch bugs

Risk:

- `ui-store.ts`, `editor-shell.tsx`, and `test-cases.ts` can drift if `site-plan` is added in one place but not the others

Likely symptom:

- load case works, but selection or active view lands in an impossible state

Lean mitigation:

- centralize view ids in the existing unions first
- update `getViewSelection(...)` before wiring new buttons

### 2. Outward-offset or mirrored-footprint bugs

Risk:

- polygon winding differs between fixtures, so the same setback value pushes one case inward and another outward

Lean mitigation:

- normalize winding once in `project-doc.ts`
- never do ad-hoc orientation logic in the component layer

### 3. Collapsed-footprint bugs from large setbacks

Risk:

- an offset value larger than the narrow part of the lot creates invalid intersections

Likely symptom:

- SVG renders a twisted polygon or nothing meaningful

Lean mitigation:

- return `footprint: null` plus one message
- keep rendering the site boundary and current spaces

### 4. Broken imports during sample-data cleanup

Risk:

- deleting `levels/` and `spaces/` before `test-cases.ts` is migrated breaks Vite raw imports immediately

Lean mitigation:

- migrate `test-cases.ts` first, delete old files last

### 5. Host-level mismatch bugs

Risk:

- `sitePlan.levelId` points to a level that does not exist in the loaded case

Lean mitigation:

- repair through `getValidActiveLevelId(...)` style fallback logic
- if repair fails, keep `Site Plan` disabled and show an inline message

### 6. Thin SVG hit-targets on edges

Risk:

- clicking a 1px boundary line is frustrating and feels broken

Lean mitigation:

- render one invisible or transparent wider stroke for hit testing
- keep the visible stroke thin

This is lower-code than adding handles or drag gizmos.

### 7. Space-selection regressions in `Site Plan`

Risk:

- trying to keep both edge selection and full space selection active in the same view creates branching bugs

Lean mitigation:

- make `Site Plan` edge-editing-first
- keep interactive multi-space selection in `Floor Plan`

## Minimal Verification Additions

To keep the code lean, verification should focus on the seams most likely to regress.

Add checks for:

- loading a document with `sitePlan: null`
- loading a document with a short `edgeSetbacksFt` array and confirming repair to full length
- reversing a clockwise boundary and preserving edge-to-setback alignment
- selecting one mixed case whose preferred view is `site-plan`
- loading a mixed case, switching to `3D View`, and confirming no sample-case load regression

## File Plan

### 1. `apps/web/src/project-doc.ts`

Add:

- `SitePlan` type
- `ProjectDoc.sitePlan`
- helpers for normalized site edges
- helpers for default setback arrays
- helpers for derived building-footprint polygons
- helpers for safe fallback when offsets collapse the footprint

### 2. `apps/web/src/project-doc.test.ts`

Add pure-logic coverage for:

- default `5 ft` setback seeding
- per-edge offset application
- derived footprint generation for simple polygons
- invalid or over-large offsets returning a safe empty or null footprint

### 3. `apps/web/src/ui-store.ts`

Update:

- `ViewMode` to include `site-plan`
- `Selection` to include `site-edge`
- default session state for the new view ids

### 4. `apps/web/src/editor-shell.tsx`

Update:

- the view tabs and ribbon to expose `Site Plan`
- selection handling for site edges
- properties panel setback input
- SVG rendering for site boundary, selected edge, building footprint, and host-level spaces
- case loading so `Site Plan` can become the preferred initial view for the mixed fixtures that need it

### 5. `apps/web/src/test-dashboard.tsx`

Reduce the dashboard to one mixed-case list only.

Keep it draggable, but remove:

- `Level` selector
- `Space` selector
- any copy that suggests split validation tracks

### 6. `apps/web/src/test-cases.ts`

Refactor to:

- import only the `3` mixed JSON files
- export only one mixed-case manifest list
- carry preferred view and preferred active level for each full scenario

### 7. `apps/web/src/styles.css`

Add only the styles needed for:

- site boundary
- selected edge emphasis
- building-footprint outline/fill
- mixed-case-only dashboard layout

### 8. `supabase/sample-data/mixed/*.json`

Replace the current mixed fixtures with `3` full scenarios that each include:

- levels
- one site boundary polygon
- one per-edge setback array
- spaces placed inside the resulting footprint

Suggested cases:

- `case-1-single-story-angled-lot.json`
- `case-2-one-basement-three-stories-tapered-lot.json`
- `case-3-three-basements-twelve-stories-wide-frontage-lot.json`

### 9. `supabase/sample-data/README.md`

Update the conventions to describe:

- mixed-case-only fixtures
- canonical site-plan fields
- setback-array rules

### 10. `supabase/sample-data/levels/` and `supabase/sample-data/spaces/`

Delete these fixtures once the mixed replacements are wired.

## Data Layout

Each mixed fixture should stay as one whole snapshot-compatible `ProjectDoc` JSON document.

Recommended minimum additions per case:

```json
{
  "sitePlan": {
    "levelId": "level-1",
    "boundary": [
      { "xFt": 0, "yFt": 0 },
      { "xFt": 120, "yFt": 0 },
      { "xFt": 132, "yFt": 84 },
      { "xFt": 16, "yFt": 96 }
    ],
    "edgeSetbacksFt": [5, 5, 5, 5]
  }
}
```

Fixture rules:

- keep the boundary point order consistent around the polygon
- keep `edgeSetbacksFt.length === boundary.length`
- use explicit `5` values in JSON instead of relying on hidden defaults
- keep all host-level spaces inside the derived footprint
- use clearly different site shapes across the `3` cases

## UI Behavior

### Open

- the `Test` dashboard shows only `3` mixed cases
- loading a case replaces the local `ProjectDoc` as it does today
- the case may choose `Site Plan`, `Floor Plan`, or `3D View` as its preferred landing view

### Site Plan

`Site Plan` should render:

- the full site boundary
- the derived building footprint
- the spaces on `sitePlan.levelId`

The floor plan for `Level 1` and the site plan should stay consistent because they are driven by the same canonical spaces.

### Select Edge

When the user clicks a site edge:

1. highlight that edge
2. show its setback field in the properties area
3. prefill the field with the current value, default `5 ft`
4. recompute the building footprint after a valid edit

### Invalid Offset

If the chosen setbacks collapse or invert the footprint:

- show a clear inline error
- keep the site boundary visible
- do not crash the site-plan view

## Testing Plan

Keep tests focused on pure TypeScript logic only.

Must cover:

- default setback seeding
- site-edge indexing order
- derived footprint generation for simple polygon cases
- stable behavior when setbacks are too large
- sample-case parsing with `sitePlan` present

No browser e2e suite is required for this task.

## Verification Plan

When implemented, run:

```bash
corepack pnpm --filter web test
corepack pnpm --filter web build
```

Manual smoke:

1. open the editor and load each of the `3` mixed cases
2. confirm the dashboard shows no standalone `Level` or `Space` selectors
3. switch to `Site Plan` and confirm the site boundary, footprint, and spaces render
4. click different site edges and confirm the setback field updates
5. edit a setback and confirm the building footprint redraws immediately
6. confirm the `Level 1` floor plan still shows a consistent space layout
7. confirm the `3D View` still loads the same mixed documents without renderer errors

## Implementation Status

Implemented in:

- `apps/web/src/project-doc.ts`
- `apps/web/src/project-doc.test.ts`
- `apps/web/src/ui-store.ts`
- `apps/web/src/editor-shell.tsx`
- `apps/web/src/test-cases.ts`
- `apps/web/src/test-dashboard.tsx`
- `apps/web/src/styles.css`
- `apps/web/src/three-d-viewport.tsx`
- `supabase/sample-data/README.md`
- `supabase/sample-data/mixed/case-1-single-story-angled-lot.json`
- `supabase/sample-data/mixed/case-2-one-basement-three-stories-tapered-lot.json`
- `supabase/sample-data/mixed/case-3-three-basements-twelve-stories-wide-frontage-lot.json`

Validation-only sample data removed from:

- `supabase/sample-data/levels/*.json`
- `supabase/sample-data/spaces/*.json`
- previous mixed fixtures under `supabase/sample-data/mixed/`

## Done Criteria

This task is complete when:

1. the editor exposes `Site Plan`, `Floor Plan`, and `3D View`
2. `ProjectDoc` canonically owns one site polygon plus per-edge setbacks
3. building footprint geometry is derived in TypeScript rather than stored as a second authored polygon
4. the user can click a site edge and edit its setback with an imperial input
5. each of the `3` mixed cases uses a different site shape and layout
6. standalone level-only and space-only sample flows are removed
7. unused sample fixtures are deleted
