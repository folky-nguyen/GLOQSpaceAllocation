# FB004 Error Registry And Trapped Message Standardization

## Status

Implemented and review-updated on `2026-03-26`.

Result:

- the repo now has a root `ER.md` registry for trapped error messages, stable codes, and owning file paths across web, API, and setup helpers
- web trap messages now follow a consistent summary-first style instead of mixing generic copy and raw provider text
- the `Site Plan` setback input trap and the current API/setup trapped messages are now included in the registry after the review pass
- the workflow now requires `ER.md` updates after `FB...` tasks only, while ordinary tasks can skip it

## Goal

Create a fast lookup surface from a trapped error message to the owning code file, standardize the current web trap copy, and wire the new registry into the repo workflow with the smallest practical diff.

This stayed lean:

- no auth-flow redesign
- no renderer ownership change
- no persistence or schema change
- no new dependency

## Final Root Cause

The repo had a process gap and a copy gap at the same time:

- trapped error messages were scattered across auth, login validation, level-manager validation, site-plan geometry validation, and the 3D viewport
- some surfaces showed a curated summary, but auth often let raw provider text replace the primary message entirely
- there was no single document mapping a visible message back to the owning source file
- the workflow did not tell bug-fix tasks to keep that mapping current

The review pass on the first implementation exposed one more gap:

- the initial `ER.md` pass still missed the `Site Plan` setback input validation trap and did not yet index the current API/setup trapped messages

That made bug triage slower than it needed to be:

- the same class of issue could be described in slightly different ways
- screenshots were not enough to jump to the right file quickly
- message edits could ship without a registry update because no workflow rule required it

## Plan

1. create `ER.md` with stable codes, canonical messages, and owning file paths
2. normalize the current trapped web messages so the human summary always comes first
3. keep low-level provider detail behind a separate `Detail:` suffix when a surface only has one error string
4. sweep for any missed current traps outside the first web-only pass and add them to the registry
5. update `AGENTS.md`, `KL.md`, and `MP.md` so `ER.md` becomes part of the bug-fix workflow only
6. run focused verification and record the results

## What Shipped

### 1. Root error registry and message rules

`ER.md`

- adds one lookup table for the current trapped web, API, and setup-helper trap messages
- defines stable code format and source-file ownership
- allows placeholder tokens for dynamic messages so registry rows stay searchable and stable
- defines the copy rules for primary messages versus raw low-level detail
- documents that `ER.md` is refreshed after `FB...` tasks, not after ordinary tasks

### 2. Summary-first auth errors

`apps/web/src/auth.ts`

- replaces the generic `Unable to ...` copy with one consistent `Could not ...` style
- keeps the canonical summary first even when Supabase returns raw provider text
- appends raw provider text after `Detail:` instead of letting it become the whole message
- aligns sign-out wording with the rest of the auth flow

`apps/web/src/editor-shell.tsx`

- now reuses the auth-layer sign-out message directly instead of replacing it with a second shell-specific message

### 3. Normalized validation and geometry errors

`apps/web/src/editor-shell.tsx`

- normalizes the level-manager validation copy

`apps/web/src/project-doc.ts`

- normalizes the site-boundary and setback failure copy

`apps/web/src/editor-shell.tsx`

- normalizes the site-edge setback input validation copy and registers it in `ER.md`

`apps/web/src/three-d-viewport.tsx`

- tightens the generic renderer fallback detail copy

### 4. API and setup coverage

`apps/api/src/config.rs`

- normalizes startup config error messages to sentence case with periods

`ER.md`

- now indexes the current API auth errors and current setup-helper trap messages so local preview failures and API auth failures are also searchable by message

### 5. Workflow updates

`AGENTS.md`

- adds `ER.md` to the repo docs
- points trapped-message rules to `ER.md`
- requires `ER.md` updates for `FB...` work only

`KL.md`

- adds glossary entries for `ER.md` and `trapped error message`

`MP.md`

- indexes `ER.md`
- indexes this `FB004` note

## Verification

Run:

```bash
cargo test --manifest-path apps/api/Cargo.toml
corepack pnpm --filter web test
corepack pnpm --filter web build
```

Observed results:

- `cargo test --manifest-path apps/api/Cargo.toml` passed with `10` API tests green
- `corepack pnpm --filter web test` passed with `35` tests green across `units`, `space-scene`, and `project-doc`
- `corepack pnpm --filter web build` passed
- the web prebuild step reused the checked-in `crates/render-wasm/pkg` artifacts because `wasm-pack` was not present locally
- Vite still reports the existing chunk-size warning for the main frontend bundle after minification

## Learnings

- a stable human summary is much easier to search than raw provider text
- if a surface only has one error slot, `Summary. Detail: ...` is a practical compromise
- review the registry against real code after the first pass, because the first inventory can still miss one small validation branch
- bug-fix tasks are the right time to refresh the error registry because that is when message-to-file ownership usually changes
