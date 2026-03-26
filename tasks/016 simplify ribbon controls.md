# 016 Simplify Ribbon Controls

## Goal

In `apps/web`, remove the dead ribbon groups that are no longer needed for the current MVP shell and simplify the `Select` group label so the top ribbon matches the requested leaner chrome.

## Required Outcome

- remove the ribbon `File` group with `New` and `Save`
- remove the ribbon `Edit` group with `Undo` and `Redo`
- remove the ribbon `View` group with `Site`, `3D`, and `Plan`
- keep view switching available through the existing workspace tabs
- keep the `Select` menu working, but stop showing the extra `Pick Many` summary text under the `Select` button
- keep the `Inspect` tools unchanged

## Constraints

- keep the change local to the web shell
- prefer deleting dead UI over hiding it with CSS
- do not add a replacement ribbon group for removed controls
- do not change `ProjectDoc`, renderer contracts, API boundaries, or Supabase schema

## Current Repo State

### `apps/web/src/editor-shell.tsx`

Today this file renders:

- dead `File` buttons via the local `ribbonGroups` array
- dead `Edit` buttons via the same `ribbonGroups` array
- duplicate `View` buttons in the ribbon even though `header.view-tabs` already switches views inside the workspace
- the `Select` button plus an extra current-mode summary line that shows `Pick Many` or `Sweep Select`

### `apps/web/src/styles.css`

Today this file still styles the select-mode summary with:

- `.select-menu-summary`

If that summary is removed from the shell, the selector should be removed too.

## Scope

In scope:

- deleting the dead ribbon button groups
- deleting the duplicate ribbon `View` group
- deleting the extra select-mode summary row below the `Select` button
- removing dead shell CSS that only styled the deleted summary

Out of scope:

- changing the behavior inside the `Select` flyout
- redesigning the remaining ribbon styles
- changing the workspace tabs, properties panel, or status bar wording unless needed for build health

## Implementation Decisions

### 1. Remove dead ribbon groups at the JSX owner

Lean rule:

- delete the `ribbonGroups` constant
- delete the JSX that mapped `File` and `Edit`
- delete the duplicate `View` ribbon section

Reason:

- these controls are either dead or duplicated by surviving seams
- deleting the owner markup is smaller and clearer than hiding buttons individually

### 2. Keep view switching on workspace tabs only

Lean rule:

- do not replace the removed ribbon `View` buttons elsewhere
- rely on the already working `view-tabs` header inside the workspace

Reason:

- the repo already has one surviving seam for view switching
- this keeps the ribbon focused on controls that still matter

### 3. Simplify the `Select` group label

Lean rule:

- keep the `Select` trigger button
- remove the extra summary line that echoed the current selection mode below it

Reason:

- the requested UI only needs `Select`
- the flyout still exposes the current mode and mode choices when opened

## Acceptance

- the top ribbon no longer shows `New`, `Save`, `Undo`, `Redo`, `Site`, `3D`, or `Plan`
- the workspace tabs still switch views
- the `Select` group no longer shows `Pick Many` under the button
- the `Select` flyout still opens and keeps its current mode actions
- no dead `.select-menu-summary` styling remains

## Verification Plan

When implemented, run:

```bash
corepack pnpm --filter web build
```

Manual smoke:

1. open `/editor`
2. confirm the ribbon only keeps the surviving groups
3. confirm the workspace tabs still switch `Site Plan`, `3D View`, and `Floor Plan`
4. open the `Select` flyout and confirm mode choices plus actions still work

## Implementation Status

Implemented in:

- `apps/web/src/editor-shell.tsx`
- `apps/web/src/styles.css`
- `tasks/016 simplify ribbon controls.md`
- `MP.md`

What landed:

- removed the dead ribbon `File` and `Edit` button groups by deleting the local `ribbonGroups` owner
- removed the duplicate ribbon `View` group and kept view switching on the existing workspace tabs
- removed the extra select-mode summary row under the `Select` button so the ribbon group now reads as a simpler `Select`
- removed the unused `.select-menu-summary` CSS selector
- added this task note and indexed it in `MP.md`

## Verification Result

- `corepack pnpm --filter web build` passed
- the web build reused the checked-in `crates/render-wasm/pkg` artifacts because `wasm-pack` was not installed locally
- Vite reported the existing chunk-size warning for `assets/index-*.js`, but the build completed successfully

## Review Update

- second-pass review against the requested screenshot found no remaining `016` scope gaps in the current shell code
- `apps/web/src/editor-shell.tsx` now keeps only the surviving ribbon groups: `Select` and `Inspect`
- the duplicate ribbon-only `File`, `Edit`, and `View` groups are gone, while the workspace tabs remain the surviving view-switch seam
- the old `Pick Many` summary row under the ribbon `Select` button is gone and the dead `.select-menu-summary` CSS is gone with it
- automated visual smoke in this environment was not completed through Playwright because the browser launcher attached to an existing Chrome session, so the signed-in UI still benefits from one manual glance in a normal browser session
