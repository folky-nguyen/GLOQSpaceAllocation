# KL
Selective glossary for specialized repo and workflow concepts. Use this file for terms that affect repeated repo understanding, calculations, ownership boundaries, persistence shape, or runtime behavior. Do not use this file to list every common variable, helper, or generic framework term.
## How To Use It
- Add a term when the meaning is specialized, reused across files/tasks, or easy to misunderstand without repo context.
- Skip one-off local variables, obvious names, and generic terms like `state`, `props`, or `component`.
- `AI` describes how the current repo code behaves. Treat it as authoritative for current implementation behavior.
- `HM` is a human-authored real-world or professional definition. Use `null` when no human definition has been provided yet.
- If `AI` and `HM` differ, keep both. The repo may intentionally simplify a broader domain concept.
- Update this file in the same task when a glossary-worthy concept is introduced, renamed, or changes meaning.
## 1. Workflow Terms
### `MP.md`
- AI: The repo map and document index that non-bug-fix tasks read first before broader discovery or text search.
- HM: null
### `ER.md`
- AI: The trapped error registry that assigns stable codes, canonical message text, and owning source-file paths for intentionally surfaced errors. Bug-fix tasks update it when those messages change.
- HM: null
### `SP.md`
- AI: The architecture contract that states ownership boundaries between the TypeScript editor, the Rust wasm renderer, the Rust API, and persistence.
- HM: null
### `task note`
- AI: A feature note in `tasks/<number> <task name>.md` that records scope, plan, verification, and implementation status for a tracked repo change.
- HM: null
### `bug note`
- AI: A bug or failure note in `tasks/FB<number> <task name>.md` that records evidence, fix scope, and verification for a bug-focused task.
- HM: null
### `trapped error message`
- AI: A deliberately surfaced validation, capability, recovery, or failure message that stops a flow early and helps a human recover or triage, instead of only exposing a raw exception string.
- HM: null
## 2. Document And Domain Terms
### `ProjectDoc`
- AI: The canonical TypeScript project document defined in `apps/web/src/project-doc.ts`. It stores project identity, default story height, levels, spaces, and the optional site plan. Editor helpers mutate this shape and persistence stores snapshots of it.
- HM: null
### `Level`
- AI: A named vertical datum stored in `ProjectDoc` with an `elevationFt` and `heightFt`. Spaces and the optional site plan reference levels by id.
- HM: null
### `Space`
- AI: A level-owned named polygon footprint stored in `ProjectDoc`. Area, bounds, label points, and render geometry are derived from its footprint points.
- HM: null
### `SitePlan`
- AI: An optional project-level site boundary stored in `ProjectDoc`. It carries a host `levelId`, a boundary polygon, and one setback value per edge, then gets repaired and normalized before derived-footprint calculations run.
- HM: null
### `site edge setback`
- AI: The per-edge inset distance applied to a `SitePlan` boundary. The repo uses these values to offset each boundary edge inward and derive the buildable footprint polygon.
- HM: null
### `road frontage flag`
- AI: A boolean aligned to one `SitePlan` boundary edge that marks whether that parcel edge touches a public road frontage. The planned site-layout algorithm uses these flags to form frontage chains, choose the primary public face, and bias building, parking, and walkway placement.
- HM: null
### `frontage chain`
- AI: One contiguous run of parcel edges whose `road frontage flag` is `true`. The future layout algorithm uses the longest chain as the primary frontage and treats the others as secondary public faces.
- HM: null
## 3. Data Semantics
### `canonical`
- AI: The authored source of truth owned by the TypeScript editor, currently the persisted `ProjectDoc` shape and its directly edited fields.
- HM: null
### `derived`
- AI: Data rebuilt from canonical state instead of authored directly, such as polygon bounds, centroids, triangulations, site-derived footprints, and scene payloads for rendering.
- HM: null
### `transient`
- AI: Session-only or interaction-only browser state that supports the editor but should not become durable project truth, such as the current view mode, selection, or auth bootstrap flags.
- HM: null
## 4. Geometry And Units Terms
### `decimal feet`
- AI: The internal numeric length unit used for geometry, level elevations, space footprints, site setbacks, and most TypeScript document calculations.
- HM: null
### `feet-inch UI`
- AI: The input and display boundary where lengths are parsed from and formatted to imperial text while the underlying authored data remains in decimal feet.
- HM: null
### `markerless shorthand`
- AI: The parser-only imperial input style accepted by `parseFeetAndInches(...)` without explicit `'` or `"` markers, such as `12 3 3/4` or `3 1/2`, with meaning resolved by the repo's current parser rules.
- HM: null
### `derived footprint`
- AI: The buildable polygon computed from a repaired `SitePlan` boundary and its edge setbacks. It is recalculated from site data and is not stored directly in `ProjectDoc`.
- HM: null
## 5. Runtime And Session Terms
### `activeView`
- AI: The session UI mode stored in the zustand UI store. It currently switches the workspace between `plan`, `site-plan`, and `3d`.
- HM: null
### `selection`
- AI: The current editor selection stored in the zustand UI store. It can point at a view, site edge, level, one selected element ref, or a multi-element set. Today the only supported selection element kind is `space`.
- HM: null
### `selection element ref`
- AI: The transient discriminated ref used inside `selection` to identify one selectable editor element. It currently supports `{ kind: "space"; id }` and exists so future selectable 3D element kinds can plug into the same shell seam without inventing a second selection shape.
- HM: null
### `auth snapshot`
- AI: The browser-side auth store state in `apps/web/src/auth.ts`. It tracks sign-in status, current session, user, error state, pending OTP context, and password-recovery readiness.
- HM: null
### `local auth bypass`
- AI: A local-development auth shortcut in `apps/web/src/auth.ts` that marks the browser as signed in when `VITE_LOCAL_AUTH_BYPASS` is `true` during local dev on `localhost` or `127.0.0.1`.
- HM: null
## 6. Persistence Terms
### `snapshot`
- AI: One versioned JSONB copy of the project document stored in `public.project_snapshots`, keyed by project id and `version_number`.
- HM: null
