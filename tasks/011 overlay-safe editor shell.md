# 011 Overlay-Safe Editor Shell

## Status

Implemented on `2026-03-25`.

## Goal

In `apps/web`, stop editor chrome from overlapping adjacent regions and harden the shell against browser zoom, Windows display scale, and longer text with the smallest code diff that still solves the problem.

This task is specifically about layout ownership and chrome sizing. It is not a visual redesign task.

## Problem Snapshot

- the top ribbon can bleed into the next layout row and cover the panel title bars plus the workspace tabs
- the same risk exists anywhere the shell assumes a fixed chrome height but the content can grow
- the failure is user-facing in `/editor` and breaks the desktop-shell contract

## Current Repo Shape

### `apps/web/src/editor-shell.tsx`

Today this file owns all relevant shell surfaces in one place:

- `ribbon`
- `main-shell`
- `workspace-shell`
- `status-bar`
- `LevelManager`
- `UnitsInspector`

This is good for the fix because the task does not need a new component seam.

### `apps/web/src/styles.css`

Today this file owns all shell sizing and chrome layout behavior.

The current brittle points are:

- `.app-shell` uses `grid-template-rows: 48px minmax(0, 1fr) 28px`
- `.workspace-shell` uses `grid-template-rows: 40px minmax(0, 1fr)`
- `.units-inspector` and `.level-manager` use `top: 3rem` plus a height calculation that assumes the tab row always stays near that size
- the mobile breakpoint at `720px` reintroduces the fixed `48px` and `28px` shell rows

### What Does Not Need A New Fix Path

The select dropdown is already anchored to its local owner:

- `.select-menu` is `position: relative`
- `.select-menu-panel` is `position: absolute`

That means this task should not broaden into a general overlay rewrite unless the select menu still reproduces the bug after the shell-height fix.

## Root Cause To Fix

### 1. Outer shell rows are fixed while their content is not

The ribbon and status bar are placed in fixed-height grid tracks, but their children include:

- borders
- padding
- text labels
- buttons
- auth text
- summary text

Those can all grow under zoom, DPI scaling, or longer content.

When that happens, the content does not stay inside its own row. It spills into the next one.

### 2. The workspace tab row has the same structural weakness

`workspace-shell` also assumes a fixed `40px` top row for `view-tabs`.

If the tabs wrap or their line height grows, the viewport and overlays are no longer aligned to the real tab height.

### 3. The floating inspectors rely on a magic offset

`UnitsInspector` and `LevelManager` are rendered under `workspace-shell`, then offset downward with `top: 3rem`.

That only works while the tab row height remains close to the original assumption.

Even if the ribbon bug disappears after row sizing is fixed, this remains a latent overlap bug.

## Minimum Viable Fix

The minimum acceptable implementation is:

1. make the shell rows content-aware
2. let ribbon and tabs wrap inside their own rows
3. anchor the two floating inspectors to the viewport area instead of to a guessed offset

Everything else is optional and should be avoided unless the minimal fix fails.

## Implementation Plan

### 1. Make only the shell rows content-aware

In `apps/web/src/styles.css`:

- change `.app-shell` from fixed tracks to `auto minmax(0, 1fr) auto`
- change `.workspace-shell` from `40px minmax(0, 1fr)` to `auto minmax(0, 1fr)`
- keep `height: 100vh`
- keep `min-height: 0` on the main scroll containers so the workspace still owns the remaining height

Do not:

- add JavaScript measurement for shell heights
- add a new layout hook
- add a new store

Reason:

- CSS grid already solves this if the rows are allowed to size to content
- this is the smallest fix for the actual failure mode

### 2. Let chrome wrap inside itself instead of outside itself

In `apps/web/src/styles.css`:

- allow `.ribbon-groups` to wrap
- allow `.ribbon-side` and `.ribbon-auth` to wrap when needed
- allow `.view-tabs` to wrap
- give the ribbon, tabs, and status bar enough vertical padding or minimum block size so the current visual density still looks intentional after row sizing becomes automatic

Do not:

- redesign the ribbon
- change the breakpoint strategy unless a current breakpoint directly reintroduces the fixed-height bug
- create alternate mobile markup

Reason:

- the shell already has the right structure
- the bug is containment, not information architecture

### 3. Move only the two brittle floating panels

In `apps/web/src/editor-shell.tsx`:

- move `LevelManager` and `UnitsInspector` so they render inside `.viewport-shell`, not as siblings under `.workspace-shell`

In `apps/web/src/styles.css`:

- change their positioning to local viewport insets such as top/left/right spacing inside the viewport region
- replace `max-height: calc(100% - 3.85rem)` with a calculation based only on viewport-local padding

Do not:

- move the select menu
- introduce portals
- create a new overlay manager

Reason:

- these two panels are the only overlays currently tied to a guessed tab height
- moving them one level deeper in the existing JSX is a small change that removes the magic offset

### 4. Prevent recurrence with a layout contract, not runtime code

For this task, prevention should come from an explicit layout rule:

- no shell row may depend on a neighboring fixed height when its own content can grow
- any floating panel must be positioned relative to the surface it visually belongs to

Do not add in this task:

- `ResizeObserver`
- overlap-detection code
- dev-only DOM measurement helpers
- screenshot automation

If the problem still reappears after the minimal CSS plus JSX fix, that is the point where a stronger guard can be justified in a later task.

## Explicit Non-Goals

- no new layout store
- no new helper modules
- no new component files
- no design-system abstraction
- no pane resizing
- no portal system
- no visual regression harness
- no changes to selection, document, or geometry logic

## Exact File Plan

### 1. `apps/web/src/styles.css`

Required edits:

- make `.app-shell` rows content-aware
- make `.workspace-shell` rows content-aware
- remove the fixed-row reset at the `720px` breakpoint
- allow ribbon and view tabs to wrap within their own row
- update `level-manager` and `units-inspector` positioning so it depends only on viewport-local insets

Preferred outcome:

- this file carries almost all of the implementation weight

### 2. `apps/web/src/editor-shell.tsx`

Required edits:

- move the rendered position of `LevelManager`
- move the rendered position of `UnitsInspector`

Explicitly avoid:

- state changes
- prop changes
- new hooks
- new components

### 3. `tasks/011 overlay-safe editor shell.md`

- keep this note current during implementation
- record any deviation from the minimum-diff plan here

### 4. `QC.md`

- update only if the issue proves to be a repeated regression after implementation review

## Acceptance

- the ribbon may become taller, but the main shell starts only after the real ribbon height
- the status bar may become taller, but the main shell ends above it
- the workspace tabs may wrap, but they do not cover the viewport
- the level manager and units inspector stay fully inside the viewport region and do not depend on a guessed tab height
- the select dropdown remains anchored to its ribbon group and does not require structural changes
- the fix stays local to `styles.css` plus a small JSX move in `editor-shell.tsx`, unless verification proves that something else is still blocking the goal

## Verification Plan

When implemented, run:

```bash
corepack pnpm run verify:web
```

Manual smoke:

1. open `/editor`
2. verify at widths `1600`, `1366`, `1280`, `1024`, `900`, and `720`
3. repeat at browser zoom `100%`, `125%`, and `150%`
4. confirm the ribbon never covers panel headers, workspace tabs, or viewport content
5. confirm the status bar never overlaps the main shell
6. open `Levels` and `Unit` and confirm both panels stay within the viewport area
7. open `Select` and confirm the dropdown still anchors correctly to the ribbon
8. sign in with a long email and confirm the auth chrome stays contained inside the ribbon row
9. switch between `Plan` and `3D` and confirm the center region still absorbs the remaining height without page-level overlap

## Done Criteria

1. no fixed shell row remains where the content can grow past it
2. `LevelManager` and `UnitsInspector` no longer depend on `top: 3rem` or an equivalent magic tab-height offset
3. the implementation stays minimal: primarily CSS, plus a small JSX move in `editor-shell.tsx`
4. `corepack pnpm run verify:web` passes
5. manual smoke confirms no overlap at the defined width and zoom matrix

## What Landed

- `apps/web/src/styles.css`
  - `.app-shell` now uses content-aware outer rows
  - `.workspace-shell` now uses a content-aware tab row
  - ribbon groups, ribbon-side, ribbon-auth, and view tabs can wrap inside their own row
  - the `720px` breakpoint no longer reintroduces fixed shell rows
  - `level-manager` and `units-inspector` now use viewport-local insets instead of a guessed `top: 3rem`
- `apps/web/src/editor-shell.tsx`
  - `LevelManager` and `UnitsInspector` now render inside `.viewport-shell`
- intentionally unchanged
  - no new helper file
  - no new layout store
  - no runtime overlap detector
  - no select-menu structural rewrite

## Verification Result

- `corepack pnpm run verify:web` passed
- manual screenshots passed at:
  - `1600x900`
  - `1280x900`
  - `720x1100`
  - CSS zoom simulation at `125%` and `150%`
- overlay screenshots confirmed:
  - ribbon no longer overlaps the workspace row
  - workspace tabs stay above the viewport
  - `LevelManager` and `UnitsInspector` remain anchored inside the viewport area
- observed console error during smoke:
  - `favicon.ico` returned `404`
  - unrelated to this layout task
