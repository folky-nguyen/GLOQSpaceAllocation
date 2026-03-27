# 017 Compact Level Manager Layout

## Goal

In `apps/web`, tighten the existing `Level Manager` window so it feels denser and wastes less space while keeping the current behavior and ownership model intact.

## Required Outcome

- reduce visible empty space in the panel header, sections, and level rows
- keep all current actions, validation, and data flow working as-is
- keep the change local to the existing `LevelManager` owner in `apps/web/src/editor-shell.tsx` plus its styles in `apps/web/src/styles.css`
- prefer CSS compaction and tiny copy changes over structural rewrites

## Constraints

- make the smallest local diff that materially improves the layout
- do not split `LevelManager` into a new file
- do not change `ProjectDoc`, editor state ownership, renderer contracts, API behavior, or Supabase schema
- do not add a new design system or new UI abstraction layer

## Current Repo State

### `apps/web/src/editor-shell.tsx`

Today this file keeps the entire `LevelManager` inline.

The current markup already groups the UI into:

- header actions
- `Auto-generate`
- `Defaults`
- `Levels`

Each level row currently uses:

- one wide activate button column
- one wide main content column
- one trailing action button group

The inactive activate label currently reads `Make Active`, which increases the space needed for that column.

### `apps/web/src/styles.css`

Today this file gives the manager a roomy desktop layout:

- panel width `min(38rem, calc(100% - 1.7rem))`
- generous section padding and row gaps
- wide row column sizing for the activate button, main fields, and action buttons

## Initial Review Update

Reviewed against the supplied screenshot and the current code.

The current density problem is layout-driven, not architecture-driven:

- the panel is wider than the content needs
- section padding and inter-control gaps stack up into visible empty space
- the level-row proportions make the button columns feel oversized
- the existing markup is already close enough that a CSS-first compaction is the smallest safe fix

Because of that, this task should avoid component extraction or field reordering and should instead tighten the existing layout seam.

## Scope

In scope:

- compact header action sizing and section spacing
- narrow the panel and level-row proportions
- reduce the visual footprint of per-row actions
- shorten the inactive activate label only if needed for density

Out of scope:

- changing level CRUD, validation, or auto-generate behavior
- changing active-level rules
- moving `LevelManager` into a separate component file
- adding UI automation just for this cosmetic compaction task

## Implementation Decisions

### 1. Keep the existing JSX owner

Lean rule:

- keep `LevelManager` inside `apps/web/src/editor-shell.tsx`

Reason:

- this is already the owner seam
- extracting it would add code without helping the compactness goal

### 2. Prefer CSS compaction over JSX rewrite

Lean rule:

- reduce panel width, padding, gaps, and row proportions in `styles.css`
- keep the existing sections and row structure unless one very small JSX tweak materially helps density

Reason:

- the current markup already supports the requested design direction
- CSS is the smallest lever for this request

### 3. Allow one tiny copy change for the activate button

Lean rule:

- if the row still feels too tall or wide, replace `Make Active` with a shorter label

Reason:

- the current label increases the minimum width of the first column
- one copy tweak is smaller than reshaping the row markup

### 4. Keep the responsive fallback intact

Lean rule:

- preserve the existing mobile stack behavior
- only tighten responsive rules if the desktop compaction requires a small matching adjustment

Reason:

- the repo already has a working narrow-screen fallback for this panel
- the task is about density, not a full responsive redesign

## Acceptance

- the `Level Manager` uses visibly less empty space than the current screenshot
- each level row is denser without losing any current controls
- all current level actions still behave the same
- the change stays local to the existing web shell seam

## Verification Plan

When implemented, run:

```bash
corepack pnpm --filter web build
```

Manual smoke if a normal signed-in session is available:

1. open the editor
2. open `Level Manager`
3. confirm the panel is narrower and denser than before
4. confirm `Create Level`, `Generate`, active switching, reorder, and delete still work
5. confirm the panel still stacks correctly on a narrow viewport

## Implementation Status

Implemented in:

- `apps/web/src/editor-shell.tsx`
- `apps/web/src/styles.css`
- `tasks/017 compact level manager layout.md`
- `MP.md`

What landed:

- tightened the `Level Manager` shell width, padding, section gaps, and field spacing to reduce empty space without changing the existing structure
- compacted the level rows by shrinking the activate column, row padding, field gaps, and action-button footprint
- reduced the input and button sizing inside the manager so the window reads denser with the same controls
- shortened the inactive row action label from `Make Active` to `Activate` to help the first column stay smaller
- added this task note and indexed it in `MP.md`

## Verification Result

- `corepack pnpm --filter web build` passed
- the web build reused the checked-in `crates/render-wasm/pkg` artifacts because `wasm-pack` was not installed locally
- Vite reported the existing chunk-size warning for `assets/index-*.js`, but the production build completed successfully

## Review Update

- second-pass review against the supplied screenshot and the final patch kept the task on the smallest safe seam: one tiny JSX copy change plus CSS-only compaction
- the final layout reduces empty space mainly by tightening width, padding, gaps, and row proportions instead of reordering fields or adding new abstractions
- responsive fallback behavior was left intact, so narrow viewports still switch to the existing single-column stack
- automated visual comparison was not completed in a signed-in browser session here, so one manual glance in the editor is still the best final confirmation of the new density
