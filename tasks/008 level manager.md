# 008 Level Manager

## Goal

In `apps/web`, add one `Levels` button that opens a small in-app level manager UI.

The manager must support:

- auto-generate levels from:
  - `Stories below grade`
  - `Stories on grade`
  - `Story height`
- create level
- delete level
- rename level
- reorder levels
- change elevation
- set default story height
- active level switching

The active level must:

- filter floor plan editing
- drive 3D visibility

All level math stays in internal decimal feet. Feet-inch parsing/formatting is UI-only.

## Current Repo State

`apps/web` currently has:

- `src/project-doc.ts`
  - canonical `ProjectDoc`, `Level`, and `Space` types
  - a starter document with one level and level-linked spaces
- `src/editor-shell.tsx`
  - one plan viewport and one 3D placeholder viewport
  - one `Unit` inspector button
  - selection-driven level display only
  - no editable level manager
- `src/units.ts`
  - feet-inch parsing and formatting helpers
  - internal math already aligned to decimal feet
- `src/units-inspector.tsx`
  - a small diagnostic panel proving the ft-in helpers already work

What is missing today:

- no canonical `activeLevelId`
- no level editing UI
- no level auto-generation flow
- no way to switch active level without piggybacking on selection
- no rule that plan and 3D are filtered by the same level state

## Scope

In scope:

- one `Levels` ribbon button
- one floating level manager panel
- one canonical active-level concept in the web editor
- pure TypeScript helpers for level mutations
- feet-inch UI inputs for elevation and story height
- active-level filtering for both plan and 3D placeholder visibility

Out of scope:

- API persistence
- Supabase schema changes
- realtime sync
- separate per-level saved view objects
- drag/drop reorder
- 3D geometry generation in Rust

## Implementation Decisions

### 1. Keep one plan viewport, not one saved floor-plan object per level

For the MVP, the app should keep one plan viewport whose visible content is driven by `activeLevelId`.

Do not introduce:

- `FloorPlan[]`
- saved per-level view settings
- a second view schema

Reason:

- the user requirement is level-filtered editing, not view authoring
- the leanest mapping is `plan viewport + activeLevelId`
- this avoids extra state and duplicated concepts

### 2. Add `activeLevelId` as editor session state, not persisted document structure

`ProjectDoc` should stay focused on model data:

- `levels`
- `spaces`
- `defaultStoryHeightFt`

`activeLevelId` should live in the editor shell state because it is session/UI state.

Reason:

- it affects visibility and editing context
- it does not belong in versioned document content for the MVP

### 3. Add only one new document field: `defaultStoryHeightFt`

Add `defaultStoryHeightFt: number` to `ProjectDoc`.

Use it for:

- new level creation
- auto-generate defaults
- manager editing

Do not add:

- per-level custom default settings
- below-grade and above-grade generation presets on the document

### 4. Keep all level mutations as pure functions in `project-doc.ts`

Add a small set of flat helpers in `apps/web/src/project-doc.ts`.

Suggested surface:

```ts
export type AutoGenerateLevelsInput = {
  storiesBelowGrade: number;
  storiesOnGrade: number;
  storyHeightFt: number;
};

export function getLevelById(doc: ProjectDoc, levelId: string): Level | null;
export function createLevel(doc: ProjectDoc, activeLevelId: string): { doc: ProjectDoc; activeLevelId: string };
export function deleteLevel(doc: ProjectDoc, levelId: string, activeLevelId: string): { doc: ProjectDoc; activeLevelId: string };
export function renameLevel(doc: ProjectDoc, levelId: string, name: string): ProjectDoc;
export function moveLevel(doc: ProjectDoc, levelId: string, direction: "up" | "down"): ProjectDoc;
export function setLevelElevation(doc: ProjectDoc, levelId: string, elevationFt: number): ProjectDoc;
export function setDefaultStoryHeight(doc: ProjectDoc, heightFt: number): ProjectDoc;
export function autoGenerateLevels(doc: ProjectDoc, input: AutoGenerateLevelsInput): { doc: ProjectDoc; activeLevelId: string };
```

Reason:

- TypeScript owns canonical geometry/editor logic
- these are deterministic mutations
- no class hierarchy or store abstraction is needed

### 5. Keep auto-generation deterministic and simple

Generation rules:

- `storiesBelowGrade`
  - create `Basement N ... Basement 1`
  - elevations are negative multiples of story height
- `storiesOnGrade`
  - create `Level 1 ... Level N`
  - `Level 1` elevation is `0`
  - upper levels stack upward by story height

Example for `1 below`, `3 on grade`, `10 ft`:

- `Basement 1` at `-10`
- `Level 1` at `0`
- `Level 2` at `10`
- `Level 3` at `20`

All stored values remain numeric feet.

### 6. Keep rename/delete behavior explicit

Rules:

- level names must be trimmed and non-empty
- deleting the last remaining level is not allowed
- deleting a level removes spaces on that level for now
- if the active level is deleted, switch to an adjacent surviving level

Reason:

- this avoids orphaned spaces in the MVP
- there is no reassignment workflow in scope

### 7. Use feet-inch parsing only at the manager boundary

Level manager text inputs for:

- elevation
- default story height
- auto-generate story height

should parse through `parseFeetAndInches(...)` from `units.ts`.

Internally:

- `ProjectDoc` stores only decimal feet
- level math uses only numeric feet

### 8. Keep the manager as one floating shell panel

Add one small `LevelManager` component similar in weight to `UnitsInspector`.

State split:

- `editor-shell.tsx`
  - owns `project`
  - owns `activeLevelId`
  - owns `showLevelManager`
- `LevelManager`
  - owns only transient form text for its fields

Do not put this in Zustand yet.

Reason:

- this is still local shell/editing state
- there is no cross-route or cross-page reuse
- a local component keeps abstraction count low

## File Plan

### 1. `apps/web/src/project-doc.ts`

Update the document model and add pure level mutation helpers.

Changes:

- add `defaultStoryHeightFt`
- keep `levels` and `spaces` flat
- add pure helpers for create/delete/rename/reorder/elevation/default height/auto-generate

### 2. `apps/web/src/editor-shell.tsx`

Add:

- local `project` state
- local `activeLevelId` state
- local `showLevelManager` state
- `Levels` ribbon button
- floating `LevelManager` mount

Update:

- plan viewport to show only spaces on `activeLevelId`
- 3D placeholder copy/stats to reflect `activeLevelId`
- project browser level rows to switch active level

### 3. `apps/web/src/styles.css`

Add only the styles needed for:

- `Levels` ribbon group/button
- floating manager panel
- compact level rows
- small action buttons

Reuse existing shell chrome instead of inventing a new design system.

### 4. `apps/web/src/project-doc.test.ts`

Add pure tests for:

- create level
- delete level and dependent spaces
- reorder
- auto-generate elevations/order
- active level fallback after destructive mutations

No UI tests are needed for this task.

## UI Behavior

### Ribbon

Add one `Levels` button beside the existing utility controls.

Behavior:

- click toggles the manager
- opening it does not change current tool, view, or selection

### Manager Sections

Keep three compact sections:

1. Auto-generate
2. Defaults
3. Levels list

### Levels List Row

Each row should expose:

- active toggle
- editable name
- editable elevation
- read-only height
- reorder up/down
- delete

### Validation

Validation rules:

- story counts must be whole numbers `>= 0`
- total generated story count must be `>= 1`
- story heights must parse and be `> 0`
- elevations must parse as feet-inch input
- level names cannot be empty after trim

## Testing Plan

Add tests only for pure document logic.

Must cover:

- `createLevel(...)` inserts above the active level and uses `defaultStoryHeightFt`
- `deleteLevel(...)` removes dependent spaces and picks the next active level
- `moveLevel(...)` changes order without changing ids
- `autoGenerateLevels(...)` creates the expected names and elevations
- `autoGenerateLevels(...)` preserves or discards spaces according to the chosen rule

## Verification Plan

When implemented, run:

```bash
corepack pnpm --filter web test
corepack pnpm --filter web build
```

Manual smoke:

1. open the editor
2. click `Levels`
3. create a level
4. rename it
5. change its elevation with ft-in input
6. reorder levels
7. switch active level
8. confirm plan content is filtered to that level
9. switch to 3D and confirm the placeholder reflects the same active level
10. run auto-generate with below-grade and on-grade inputs and confirm elevations are correct

## Done Criteria

This task is complete when:

1. the editor has one working `Levels` button
2. the manager supports all requested CRUD/reorder/elevation/default-height actions
3. auto-generate produces deterministic level names and elevations
4. `activeLevelId` canonically drives both plan filtering and 3D visibility
5. all level math stays in internal feet
6. ft-in strings are parsed only at the UI edge
7. only pure document logic gets tests
8. the web test/build checks pass
