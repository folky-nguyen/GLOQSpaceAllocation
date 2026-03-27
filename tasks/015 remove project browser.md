# 015 Remove Project Browser

## Goal

In `apps/web`, remove the right-side `Project Browser` panel and every button or picker that only exists inside that panel, so the editor shell keeps only working chrome with the smallest local diff.

## Required Outcome

- remove the full `Project Browser` sidebar from the editor shell
- remove the `Views`, `Levels`, and `Spaces` button stacks that currently live inside that panel
- keep users able to switch views without the browser by using the existing workspace tabs
- keep users able to work without the browser-only level picker by leaning on the existing shell seams instead of introducing a replacement panel
- reclaim the freed width for the main workspace instead of leaving an empty right column
- remove dead browser-specific state and styles if nothing else still uses them

## Constraints

- keep `ProjectDoc` canonical in TypeScript
- do not add a replacement browser, flyout, or placeholder panel in this task
- do not move selection or document ownership into Rust, Supabase, or a new store
- keep file count and abstraction count low
- prefer deleting dead UI and dead state over hiding it with CSS

## Current Repo State

### `apps/web/src/editor-shell.tsx`

Today this file renders the full right sidebar:

- one `aside.sidebar.sidebar-right`
- one `section.project-browser`
- browser `Views` buttons that duplicate the existing workspace view tabs
- one browser `Levels` picker that exists only inside the panel
- browser `Spaces` buttons that mirror current plan or site-plan spaces
- browser-only local state and refs:
  - `showLevelValidationMenu`
  - `levelValidationMenuRef`
- browser-only handler and close logic:
  - `handleBrowserSpaceSelection(...)`
  - the outside-click effect that closes both the select menu and the browser level picker
- one browser-named derived variable, `browserSpaces`, that still feeds the status bar even outside the removed panel

The same file also already owns the shell seams that can survive the removal:

- top ribbon
- left `Properties` panel
- center workspace
- workspace view tabs
- status bar

This is good for the change because the panel can be removed at the JSX owner instead of being hidden indirectly.

### `apps/web/src/styles.css`

Today this file still reserves layout space for the removed surface:

- `.main-shell` uses `grid-template-columns: 240px minmax(0, 1fr) 280px`
- `.sidebar-right`
- `.project-browser`
- `.browser-group`
- `.browser-list`
- `.browser-row`
- `.browser-row-kind`
- shared selectors that still include browser classes:
  - `.ribbon-button, .view-tab, .browser-row`
  - hover and active variants for `.browser-row`

The responsive breakpoints at `1200px`, `900px`, and `720px` also still account for the right sidebar.

### `apps/web/src/ui-store.ts`

The browser removal should not require a new store seam.

The existing store owns:

- `activeView`
- `selectMode`
- `selection`

This task should only remove browser-driven usage if the right panel was the only consumer.

## Scope

In scope:

- deleting the right sidebar JSX
- deleting browser-only buttons and picker UI
- deleting browser-only local state, refs, and handlers if they become unused
- updating shell layout CSS so the center workspace absorbs the removed column
- updating responsive layout rules that assumed a right sidebar
- small copy updates in status or properties only if needed to avoid stale browser wording

Out of scope:

- redesigning the ribbon
- adding a new level-switcher surface unless implementation proves the current shell becomes unusable without one
- changing `ProjectDoc`, geometry logic, renderer contracts, or Supabase schema
- changing the draggable `Test` dashboard beyond what is needed to keep the shell clean
- adding resizable panes or a docking system

## Implementation Decisions

### 1. Delete the panel instead of visually hiding it

Lean rule:

- remove `aside.sidebar.sidebar-right` from `editor-shell.tsx`
- remove the nested `project-browser` markup with it

Do not:

- keep the DOM mounted and hide it with `display: none`
- keep empty placeholder chrome on the right

Reason:

- this task is explicitly about removing the browser, not collapsing it temporarily
- deleting the owner markup is the smallest way to prevent stale focus targets and duplicate controls

### 2. Treat workspace tabs as the surviving view switcher

The current shell already has a working view-navigation seam:

- `header.view-tabs`

Lean rule:

- browser `Views` buttons should be deleted without replacement
- view switching should continue through the existing tabs only

Reason:

- the browser view list duplicates an existing control surface
- removing duplicate navigation is lower-risk than inventing another substitute

### 3. Remove the browser-only level picker in the same task

Current state:

- the `Levels` section inside the browser hosts the only visible level-validation picker

Lean rule:

- remove that picker together with the browser
- delete its local state, refs, and open/close plumbing if no other surface needs them
- remove the browser-only close path from the global outside-click effect
- stop `setActiveLevel(...)` from resetting browser menu state that no longer exists

Escalation point:

- if implementation proves active-level switching becomes blocked for normal floor-plan use, add the smallest replacement to an existing surviving surface such as `Properties` or the ribbon
- do not add a new sidebar or floating panel

Reason:

- the user asked to remove the browser and the buttons inside it
- the repo should not keep browser-specific state alive after the browser is gone

### 4. Keep the space list out instead of relocating it

Lean rule:

- remove the browser `Spaces` buttons
- keep space selection in the actual plan or site-plan canvas
- keep selection feedback in the existing `Properties` panel and status bar

Reason:

- the browser list is a secondary duplicate of the real drawing surface
- moving the same list elsewhere would keep the same chrome weight without proving value

### 5. Let the center shell absorb the freed width

In `styles.css`:

- change `.main-shell` from three columns to two columns
- remove layout rules that reserve or stack `.sidebar-right`
- keep the left `Properties` panel and center workspace intact

Reason:

- the product value of this task is more usable workspace width
- layout should change structurally, not just cosmetically

### 6. Remove dead browser styling after JSX deletion

Delete only the CSS selectors that become unused:

- `.project-browser`
- `.browser-group`
- `.browser-list`
- `.browser-row`
- `.browser-row-kind`
- any responsive rules that only existed for `.sidebar-right`
- the browser pieces of any shared button hover and active selectors

Keep shared button styles only if another owner still uses them.

Reason:

- dead CSS will otherwise mislead future discovery
- this repo relies on `MP.md` and task notes for low-friction file discovery, so stale selectors are real maintenance cost

## Leanest Implementation Path

1. remove the browser JSX in `editor-shell.tsx`
2. delete now-unused browser state, refs, helpers, and click handlers in the same file
3. rename any surviving browser-named derived values so status and properties describe visible view data instead of deleted UI
4. collapse `.main-shell` to a two-column layout in `styles.css`
5. remove browser-only CSS, shared selector fragments, and breakpoint branches
6. smoke-check that view switching, active selection, and overlays still fit the widened workspace

This task should stay inside the existing shell files unless implementation proves a small surviving level-switcher is required.

## Risk Register

### 1. Active-level switching may become stranded

Risk:

- after deleting the browser `Levels` picker, there may be no remaining lightweight way to change the active level during normal editing

Lean mitigation:

- verify whether existing flows such as `Levels` manager are sufficient
- only if they are not, add one minimal replacement inside an existing surviving surface

### 2. Status text may still reference browser-driven data

Risk:

- the status bar currently uses `browserSpaces.length`
- naming can become misleading once the browser list is gone

Lean mitigation:

- keep the derived data if still useful, but rename local variables so they describe view-visible spaces instead of browser-owned spaces

### 3. Menu-dismiss logic can keep dead references

Risk:

- the window-level pointerdown effect currently knows about both the select menu and the browser level picker
- deleting only the panel JSX can leave dead refs or unnecessary close logic behind

Lean mitigation:

- simplify the effect in the same edit pass so it only manages the surviving select menu

### 4. Responsive layout can leave stale empty regions

Risk:

- current breakpoints still move or size `.sidebar-right`
- deleting only the JSX may leave mobile and tablet layout assumptions behind

Lean mitigation:

- update all three `main-shell` breakpoint definitions in the same task

### 5. Hidden browser handlers can leave dead code behind

Risk:

- unused refs, state, and handlers around browser controls may survive compile-time if they are still weakly referenced

Lean mitigation:

- clean `editor-shell.tsx` immediately after JSX deletion
- rely on TypeScript and build output to catch remaining dead references

## File Plan

### 1. `apps/web/src/editor-shell.tsx`

Remove:

- the right sidebar `aside`
- the `Project Browser` panel markup
- browser view buttons
- browser level picker
- browser space buttons
- browser-only local state and handlers that become unused
- browser-only outside-click and active-level menu plumbing

Rename:

- `browserSpaces` to a surviving view-oriented name if the derived count remains useful in the status bar

Keep:

- view tabs
- properties panel
- workspace rendering
- status bar

### 2. `apps/web/src/styles.css`

Update:

- `.main-shell` column layout
- responsive layout rules that assumed the right sidebar
- shared button selectors that currently include browser classes

Delete:

- browser-only panel styles

### 3. `tasks/015 remove project browser.md`

Keep this note current during implementation if the task later moves from plan to code.

## Acceptance

- no `Project Browser` title or panel remains in the shell
- no `Views`, `Levels`, or `Spaces` buttons remain from the removed panel
- the main workspace becomes wider after the panel is removed
- view tabs still switch between the supported workspace views
- the shell still boots and the main editor layout remains stable across current breakpoints
- no dead browser-only state or styles remain unless a surviving control still uses them

## Verification Plan

When implemented, run:

```bash
corepack pnpm --filter web build
```

Manual smoke:

1. open `/editor`
2. confirm the right `Project Browser` panel is gone
3. confirm the workspace uses the freed width
4. switch between `Floor Plan`, `Site Plan`, and `3D View` through the view tabs
5. select spaces in plan view and confirm properties plus status still update
6. open `Levels`, `Test`, and `Unit` overlays and confirm the wider workspace still contains them correctly
7. repeat at the current responsive breakpoints to ensure no stale empty right column remains

## Done Criteria

1. the right-side `Project Browser` panel is removed
2. every button or picker that lived only inside that panel is removed too
3. the shell layout is updated so the center workspace absorbs the freed space
4. the change stays local to existing shell files unless a minimal surviving level switcher is proven necessary
5. `corepack pnpm --filter web build` passes once the implementation lands

## Review Update

The implementation review added three concrete cleanup requirements that were easy to miss from the first draft:

- remove the browser level-picker state, ref, and outside-click plumbing along with the panel
- clean shared CSS selectors that still listed browser button classes even after the panel is gone
- rename the browser-owned space-count variable so the status bar no longer reads from deleted UI concepts

## Implementation Status

Implemented in:

- `apps/web/src/editor-shell.tsx`
- `apps/web/src/styles.css`
- `tasks/015 remove project browser.md`

What landed:

- removed the full right `Project Browser` sidebar markup
- removed browser-only view, level, and space controls without adding a replacement panel
- removed browser-only state, refs, handlers, and the extra outside-click close path
- renamed the surviving space-count seam to `currentViewSpaces` and kept the status bar count as a neutral `Spaces`
- collapsed the shell to a two-column layout and deleted browser-only styles plus breakpoint rules

What stayed intentionally unchanged:

- `LevelManager` remains the surviving level-management surface
- no new store, overlay, or replacement browser was introduced

## Verification Result

- `corepack pnpm --filter web build` passed
- no remaining `Project Browser`, `browser-row`, `browser-group`, or browser level-picker references remain in `editor-shell.tsx` or `styles.css`
