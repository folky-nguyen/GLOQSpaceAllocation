# 018 Draggable Workspace Windows

## Goal

Make every floating workspace window movable by its header, matching the current `Test Dashboard` behavior, with the smallest safe code change.

## Required Outcome

- `Test Dashboard` keeps its current drag behavior
- `Level Manager` can be dragged inside the workspace
- `Units Inspector` can be dragged inside the workspace
- all three windows stay within visible workspace bounds while dragged
- existing editor state, data flow, and window content stay unchanged

## Constraints

- keep the change inside `apps/web/src/`
- prefer one shared drag seam over copy-pasted pointer logic
- do not add resize, docking, persistence, or z-order management
- preserve the current default open positions as much as possible

## Current Repo State

- `apps/web/src/test-dashboard.tsx`
  - owns the only draggable floating window today
  - keeps local pointer and clamping logic inside the component
- `apps/web/src/editor-shell.tsx`
  - keeps `LevelManager` inline as a fixed-position floating panel
- `apps/web/src/units-inspector.tsx`
  - renders `Units Inspector` as a fixed-position floating panel
- `apps/web/src/styles.css`
  - styles all three floating windows and their headers

## Minimal Implementation Plan

### 1. Reuse one shared drag seam

Move the pointer-drag and workspace-clamp behavior into one small helper under `apps/web/src/`.

Reason:

- this keeps `Test Dashboard`, `Level Manager`, and `Units Inspector` aligned
- it avoids repeating the same pointer listeners in multiple files

### 2. Keep each window owner in place

- keep `Test Dashboard` in `apps/web/src/test-dashboard.tsx`
- keep `LevelManager` inline inside `apps/web/src/editor-shell.tsx`
- keep `Units Inspector` in `apps/web/src/units-inspector.tsx`

Reason:

- the task is behavior reuse, not component extraction
- each file already owns its window content

### 3. Drag by header only

Rules:

- start drag from the header surface only
- ignore header buttons so `Close` and `Create Level` keep working normally
- clamp moved windows inside the visible workspace

### 4. Keep mobile fallback simple

Do not redesign the narrow-screen layout.

If needed, keep the existing small-screen pinned positioning rules so dragged desktop coordinates do not break the mobile view.

## Verification Plan

Run:

```bash
corepack pnpm --filter web build
```

Manual smoke:

1. open `Test`, `Levels`, and `Unit`
2. drag each window by its header
3. confirm each window stays inside the workspace bounds
4. confirm header buttons still click normally

## Implementation Status

Implemented in:

- `apps/web/src/draggable-panel.ts`
- `apps/web/src/test-dashboard.tsx`
- `apps/web/src/editor-shell.tsx`
- `apps/web/src/units-inspector.tsx`
- `apps/web/src/styles.css`
- `tasks/018 draggable workspace windows.md`
- `MP.md`

What landed:

- extracted the shared workspace drag and clamp behavior into `useDraggablePanel`
- kept `Test Dashboard` on the shared drag seam without changing its content or initial position
- made `Level Manager` draggable by its header inside the workspace
- made `Units Inspector` draggable by its header inside the workspace while preserving its default top-right open position until the first drag
- aligned close/reopen behavior by unmounting `Units Inspector` when closed, so all three dialogs reopen from their default positions
- added header drag affordances and small-screen pinned overrides so mobile layout stays stable after desktop dragging

## Verification Result

- `corepack pnpm --filter web build` passed
- `Select-String -Path apps/web/src/*.tsx -Pattern 'role="dialog"'` confirmed the draggable workspace-window scope is the current set of three dialogs:
  - `apps/web/src/editor-shell.tsx`
  - `apps/web/src/test-dashboard.tsx`
  - `apps/web/src/units-inspector.tsx`

## Review Notes

- the implementation stayed on the smallest seam by reusing one helper instead of copying pointer listeners into each window
- no window persistence, docking, z-order, or content refactor was added
- manual browser dragging was not exercised here, so one quick in-app smoke pass is still the best final confirmation

## Review Update

Code review originally found one behavior gap before final closeout:

- `Units Inspector` had been rendered as a mounted component that returned `null` when closed, so its local drag state survived close and reopen within the same session
- `Level Manager` and `Test Dashboard` already unmounted on close, so they reopened from their default positions instead
- that meant the three workspace dialogs did not yet have one consistent close/reopen behavior

## Follow-Up

Implemented the smallest follow-up:

1. `Units Inspector` now mounts conditionally like the other two dialogs

Result:

- all three current workspace dialogs drag by header
- all three unmount on close
- all three reopen from their default positions without adding persistence to the drag seam
