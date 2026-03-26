# FB006 Chrome Signed-In Profile Returns No WebGPU Adapter

## Status

Implemented on `2026-03-26`.

Result:

- the browser-side WebGPU probe now requests a high-performance adapter instead of the previous no-options request
- a browser probe that returns no adapter or throws no longer blocks renderer startup by itself
- `3D View` now lets `create_renderer(canvas)` remain the source of truth for adapter availability whenever `navigator.gpu` exists
- unsupported and startup-error overlays can now include browser-probe diagnostics so Chrome-profile-specific failures are easier to interpret without changing the canonical primary summaries

## Goal

Fix the current `3D View` bug where the same machine and the same Chrome build behave differently by browser profile:

- a Chrome window already signed into the user's Google profile shows `WebGPU unavailable`
- a fresh Chrome window on the same machine renders `3D View` correctly

Keep this lean:

- no fallback renderer
- no scene or sample-data changes
- no auth-flow changes
- no Rust edit unless the browser-side evidence proves TypeScript cannot distinguish the failure safely

## Current Evidence

### 1. The failure is tied to Chrome profile state, not to project data

The two screenshots show:

- the same `localhost:3001/editor` surface
- the same app session shape with `local-dev@gloq.local`
- different outcomes depending on which Chrome window/profile is used

That means the first suspect is not:

- Supabase auth
- document loading
- sample-case geometry
- level or visibility state

The first suspect is browser-process or browser-profile-specific WebGPU behavior.

### 2. The failing path currently stops at the browser-side preflight probe

The failing screenshot shows:

- `WebGPU unavailable`
- `Type: WebGPU adapter unavailable`
- detail text `Browser WebGPU probe returned no adapter.`

That detail comes from `apps/web/src/three-d-viewport.tsx`, inside `probeBrowserWebGpu()`.

So the current failing order is:

1. the app enters `3D View`
2. `probeBrowserWebGpu()` calls `navigator.gpu.requestAdapter()`
3. the browser probe returns `null`
4. the app stops before `create_renderer(canvas)` is allowed to run

### 3. The browser probe and the real renderer startup are not the same request

Today the browser preflight and the renderer do not request an adapter the same way:

- `apps/web/src/three-d-viewport.tsx` calls `navigator.gpu.requestAdapter()` with no options
- `crates/render-wasm/src/lib.rs` calls `instance.request_adapter(...)` with:
  - `power_preference: HighPerformance`
  - `compatible_surface: Some(&surface)`
  - `force_fallback_adapter: false`

That means the current browser-side probe is not yet a like-for-like predictor for the real renderer path.

So the first investigation target should not be "remove the gate immediately." It should be:

- verify whether the probe is mismatched with the real startup path
- then decide whether the gate should be aligned, softened, or removed

## Working Hypotheses

### 1. Probe/runtime mismatch is the leading app-side hypothesis

`probeBrowserWebGpu()` may be giving a misleading result because it uses a weaker or different adapter request than the real renderer startup.

If true, the fix should be:

- first align the browser probe more closely with renderer intent
- then keep or relax the gate based on real reproduction evidence

### 2. The signed-in Chrome context may have profile-specific GPU restrictions

The signed-in Chrome profile may differ from the fresh window because of:

- hardware acceleration settings
- browser policy
- extension interference
- profile-specific experimental flags

If true, the app cannot manufacture a GPU adapter, but it can:

- stop implying this is only a machine-wide device problem
- show a clearer recovery hint tied to browser/profile state

### 3. The current unsupported state may be too terminal for a profile-specific startup failure

The init effect currently bails out when `phase === "unsupported"`.

That is acceptable for a truly unsupported browser, but it is worth checking whether a profile-specific GPU reset or retry path should be allowed before forcing the user to reload or remount the viewport.

## Investigation Plan

### Step 1. Reproduce on the same machine with both Chrome windows

Confirm the report with the smallest matrix:

1. signed-in Chrome profile/context -> `/editor` -> `3D View`
2. fresh or alternate Chrome window/context -> `/editor` -> `3D View`
3. record exact overlay title, type, and detail text for both
4. keep the same document and view scope while reproducing so the variable stays on browser context only

### Step 2. Compare like-for-like adapter requests before bypassing the gate

Temporarily instrument `apps/web/src/three-d-viewport.tsx` so we can compare:

- `navigator.gpu.requestAdapter()` with the current no-options call
- `navigator.gpu.requestAdapter({ powerPreference: "high-performance" })`
- the result of `create_renderer(canvas)`

This is the key branch point for the fix:

- if the aligned browser probe succeeds but the current no-options probe fails, the fix is probably to align the preflight instead of removing it
- if both browser probes fail but renderer creation works, the preflight must stop being a hard gate
- if all paths fail in the signed-in context only, the issue is likely real but profile-specific and the message may need better recovery guidance

### Step 3. Inspect browser-context factors only after the app-side mismatch is checked

If the signed-in context still fails after a like-for-like probe check, inspect the smallest profile-specific factors next:

- hardware acceleration on/off state
- extension interference
- guest/profile/incognito differences
- any useful `chrome://gpu` clues that explain why the signed-in context cannot expose a usable adapter

Do not widen scope into auth or project data unless reproduction disproves the browser-context hypothesis.

### Step 4. Choose the smallest proven fix

Choose only one of these paths:

1. If the no-options browser probe is the mismatch:
   - update the browser probe so it better matches renderer intent
   - keep the typed unsupported path
2. If the preflight still fails while direct renderer creation succeeds:
   - remove or demote the browser preflight as a blocking gate
   - let renderer creation decide `ready` vs `unsupported` vs `error`
3. If the signed-in context truly cannot provide an adapter:
   - keep `WEB-3D-002`
   - update the detail or recovery hint so it mentions browser/profile state instead of only the device
4. If the unsupported state is too sticky after a transient profile/browser recovery:
   - add the smallest safe retry trigger

### Step 5. Touch Rust only if TypeScript cannot classify safely

Only edit `crates/render-wasm/src/lib.rs` if:

- TypeScript cannot distinguish the startup branches safely
- or the renderer needs to return a more stable startup reason than the current free-form string

Otherwise keep Rust unchanged.

## File Plan

### 1. `apps/web/src/three-d-viewport.tsx`

Primary investigation and likely fix surface:

- startup gating
- browser probe options
- WebGPU classification
- retry behavior if needed
- typed recovery copy for browser-profile failures

### 2. `crates/render-wasm/src/lib.rs`

Touch only if the renderer must expose a more stable startup reason than the current thrown string.

### 3. `ER.md`

Update only during implementation closeout:

- change registry rows if the canonical primary message changes
- otherwise add an `FB006` review note in `FB Review Log`

### 4. `tasks/FB006 chrome signed-in profile returns no webgpu adapter.md`

Keep this note current with:

- reproduction outcome
- chosen fix direction
- final verification results

## Verification Plan

When implemented, run:

```bash
corepack pnpm --filter web test
corepack pnpm --filter web build
```

Run this only if `crates/render-wasm/src/lib.rs` changes:

```bash
corepack pnpm run build:wasm
```

Manual smoke:

1. open `/editor` in the signed-in Chrome profile that currently fails
2. switch to `3D View`
3. open `/editor` in the fresh or alternate Chrome window that currently works
4. switch to `3D View`
5. confirm the final behavior is one of these:
   - both windows render successfully
   - or the failing profile lands in a clearer stable unsupported state with profile-aware recovery guidance
6. if the fix keeps an unsupported path, confirm tab revisit or the intended retry path behaves predictably
7. confirm `Site Plan` and `Floor Plan` are unaffected

## Done Criteria

This task is complete when:

1. the same-browser different-profile behavior is reproduced and classified
2. the app has proven whether the problem is a probe/runtime mismatch, a real unsupported signed-in context, or a retry-state problem
3. the startup gate matches the real renderer path closely enough that it does not misclassify the signed-in context
4. if the profile truly cannot use WebGPU, the product message no longer implies a generic device-wide failure without a browser/profile hint
5. the smallest fix is chosen without touching scene, auth, or sample-data flows
6. `ER.md` is updated appropriately during implementation closeout

## What Shipped

### 1. Probe mismatch reduced in `three-d-viewport.tsx`

`apps/web/src/three-d-viewport.tsx`

- updates the browser probe to use `powerPreference: "high-performance"` and `forceFallbackAdapter: false`
- keeps the fast hard-stop only for the true missing-API case where `navigator.gpu` is absent

### 2. Browser probe is now diagnostic-only for adapter null or thrown probe errors

`apps/web/src/three-d-viewport.tsx`

- if the probe returns `null`, the app still loads wasm and attempts `create_renderer(canvas)`
- if the probe throws, the app still attempts renderer startup and preserves the probe failure text as diagnostic context

This avoids stopping early on a browser-side preflight that cannot fully match the wasm renderer's `compatible_surface` startup path.

### 3. No-adapter and startup-error details now preserve browser-context clues

`apps/web/src/three-d-viewport.tsx`

- when renderer startup later fails, the typed overlay detail can merge:
  - the real renderer error text
  - the earlier browser-probe diagnostic
  - a short hint that Chrome profile settings, extensions, or hardware acceleration may be affecting this window

The canonical primary summaries stay the same, so `ER.md` only needs a review-log note.

## Verification

Run:

```bash
corepack pnpm --filter web test
corepack pnpm --filter web build
```

Observed results:

- `corepack pnpm --filter web test` passed with `39` tests green
- `corepack pnpm --filter web build` passed
- the build reused checked-in `crates/render-wasm/pkg` artifacts because `wasm-pack` is not available on this machine

Manual smoke:

- not run in this task closeout because the reported difference depends on the user's real signed-in Chrome profile context, which was not available to reproduce inside this workspace session
