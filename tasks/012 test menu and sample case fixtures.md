# 012 Test Menu And Sample Case Fixtures

Superseded by `012.01 draggable test dashboard and polygon apartment cases.md` for the current implementation direction.

## Goal

Add one in-app `Test` button that opens a cascading validation menu.

Instead of using feature-specific editor validation only through `project-doc.test.ts`, the editor should support loading repeatable sample cases directly from the UI.

The menu should support:

- one top-level `Test` button
- one first-level flyout with validation groups such as:
  - `Test Level`
  - `Test Space`
  - future groups as needed
- one second-level flyout to the right with `3-5` concrete cases per group
- clicking a case should immediately load that sample scenario into the local editor state

Sample case data must live under `supabase/sample-data/`.

This data is not connected to live Supabase reads or writes yet, but it must stay compatible with future Supabase-backed snapshot flows.

## Current Repo State

`apps/web` currently has:

- `src/editor-shell.tsx`
  - main editor shell
  - ribbon controls
  - local editor session state
- `src/project-doc.ts`
  - canonical `ProjectDoc` and pure document helpers
- `src/project-doc.test.ts`
  - pure logic tests for deterministic document behavior

`supabase/` currently has:

- local config
- SQL migrations
- no checked-in sample case folder

What is missing today:

- no in-app validation menu
- no reusable sample case catalog
- no standard folder for checked-in editor scenarios
- no workflow rule for how sample cases should be authored and stored

## Scope

In scope:

- one `Test` ribbon button in the web editor
- one cascading validation menu
- grouped case definitions such as `Test Level`
- `3-5` cases per group
- loading local sample `ProjectDoc` snapshots into the editor
- checked-in sample JSON under `supabase/sample-data/`
- workflow documentation for creating and maintaining sample cases

Out of scope:

- live Supabase fetch for test cases
- authenticated persistence of test cases
- replacing all existing unit tests
- a UI for authoring new sample cases in-browser
- automated e2e execution

## Implementation Decisions

### 1. Keep unit tests for pure logic, use the Test menu for interactive editor validation

Do not treat the new `Test` menu as a replacement for all `*.test.ts` files.

Rule:

- keep `*.test.ts` for pure logic and critical API behavior
- use the `Test` menu for interactive editor scenarios, visual checks, and reusable authored sample cases

Reason:

- these solve different failure modes
- unit tests are fast and deterministic
- UI sample cases are better for editor workflows that need visual inspection

### 2. Keep sample case files as snapshot-compatible JSON

Each sample case should be stored as a JSON document that can map directly to the future persisted `ProjectDoc` snapshot shape.

Do not store:

- ad-hoc partial fragments
- UI-only derived state
- a second schema unrelated to the editor document

Reason:

- future Supabase persistence will already use document snapshots
- one canonical shape keeps fixtures reusable

### 3. Make `supabase/sample-data/` the source of truth for sample cases

Use:

- `supabase/sample-data/<group>/<case-id>.json`

Examples:

- `supabase/sample-data/levels/basic-two-levels.json`
- `supabase/sample-data/levels/basement-stack.json`
- `supabase/sample-data/spaces/open-office-split.json`

Reason:

- the folder sits beside persistence infrastructure
- fixtures remain close to the future snapshot contract
- the repo gets one predictable place for reusable editor scenarios

### 4. Keep the UI catalog separate from the JSON content

Add one small TypeScript catalog file in `apps/web/src/` that maps:

- menu group label
- case label
- case id
- sample data path or loader

Reason:

- the UI needs labels and grouping
- the JSON files should remain content-only

### 5. Load a case by replacing the current local editor document

When a case is clicked:

- replace the current `ProjectDoc` in editor session state
- reset transient selection and active view state as needed
- choose a valid `activeLevelId` from the loaded document

Do not merge the case into the current project.

Reason:

- cases should be deterministic and reproducible
- merge logic adds unnecessary ambiguity

### 6. Keep menu depth fixed and simple

Use exactly:

- level 1: `Test`
- level 2: validation group
- level 3: case list

Do not add deeper nesting, search, or a tree browser in the first version.

Reason:

- the menu should stay fast and predictable
- `3-5` cases per group is small enough for a simple flyout

### 7. Keep future Supabase compatibility explicit

The sample files are not loaded from Supabase yet.

But they should already obey these rules:

- JSON only
- stable ids
- canonical numeric feet values
- document fields that match the editor document contract

If Vite cannot load files directly from `supabase/sample-data/`, add a thin loader or manifest step later, but keep `supabase/sample-data/` as the source of truth.

## File Plan

### 1. `supabase/sample-data/README.md`

Add a short convention note for:

- folder purpose
- naming
- JSON shape expectations
- future Supabase compatibility

### 2. `apps/web/src/editor-shell.tsx`

Add:

- `Test` ribbon button
- cascading validation menu mount
- case loading behavior into local editor state

### 3. `apps/web/src/test-cases.ts`

Add:

- grouped menu metadata
- case labels
- sample data loader mapping

### 4. `supabase/sample-data/<group>/*.json`

Add:

- checked-in sample `ProjectDoc` cases
- at least `3-5` cases for the first group implemented

### 5. `AGENTS.md`

Update the common workflow so sample test data creation is part of the repo operating rules.

## UI Behavior

### Ribbon

Add one `Test` button near the existing utility controls.

Behavior:

- click opens or closes the validation menu
- the menu does not mutate the project until a case is clicked

### Flyout Structure

Example:

- `Test`
  - `Test Level`
    - `Basic Two Levels`
    - `Basement + Ground`
    - `Rename + Reorder`
  - `Test Space`
    - `Single Open Office`
    - `Split Core + Rooms`
    - `Dense Small Rooms`

### Case Click

When a case is clicked:

1. load the sample document
2. replace the current editor document
3. repair `activeLevelId`
4. clear or repair selection
5. close the menu

## Data Conventions

Each sample file should:

- be valid JSON
- represent one whole `ProjectDoc`
- keep all geometry lengths in decimal feet
- keep ids stable once introduced
- use human-readable file names

Recommended file naming:

- lowercase kebab-case
- one scenario per file

Recommended group folders:

- `levels/`
- `spaces/`
- `views/`
- `mixed/`

## Verification Plan

When implemented, verify:

1. the `Test` button opens a flyout
2. hovering or clicking a group reveals the right-side case list
3. each case loads deterministic sample data
4. the loaded document respects `activeLevelId`
5. plan and 3D stay consistent after case load
6. sample JSON remains canonical and future snapshot-compatible

## Done Criteria

This task is complete when:

1. the editor has one working `Test` button
2. the menu supports grouped validation cases with a right-side flyout
3. clicking a case loads a deterministic sample editor scenario
4. sample case data lives under `supabase/sample-data/`
5. sample files are compatible with future Supabase snapshot use
6. `AGENTS.md` documents the shared workflow for sample test data
