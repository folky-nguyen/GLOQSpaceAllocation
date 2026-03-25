# 010 Selection Dropdown And Workspace Cleanup

## Goal

In `apps/web`, remove dead editor chrome that users cannot act on, then replace the old left-side tool stack with one real `Select` dropdown that exposes working selection behaviors for the MVP.

## Required Outcome

- remove the left `Tools` panel
- remove the plan title/status overlay inside the floor-plan viewport
- delete the now-dead `ToolMode` UI pipeline that existed only to light up inactive buttons
- add one `Select` dropdown in the ribbon
- support these selection flows in the current plan/browser shell:
  - pick many
  - sweep select
  - select all visible
  - clear selection
- every select flow must have a clear deselect path
- show short usage guidance in the dropdown/status UI

## Constraints

- keep `ProjectDoc` canonical in TypeScript
- keep file count low
- do not add placeholder UI that the user cannot trigger
- do not introduce a second domain model for selection state outside the web shell

## Acceptance

- no `Space` or `Level` tool buttons remain in the old side panel
- plan viewport content starts immediately in the usable workspace
- multi-selection highlights consistently in plan, browser, properties, and 3D emphasis
- build passes with the cleaned-up code path
