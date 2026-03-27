# FB005 3D Startup Should Not Block On wasm Probe

## Status

Implemented on `2026-03-26`.

Result:

- `3D View` no longer blocks the real renderer startup on the separate wasm-side `probe_webgpu()` step
- the browser now uses a direct `navigator.gpu.requestAdapter()` check for capability triage before wasm startup
- machines or browser profiles that hit a false negative in the wasm probe can now continue into `create_renderer(...)`
- `ER.md` was reviewed and updated during closeout; the affected `WEB-3D-001` and `WEB-3D-002` rows kept the same canonical messages, so the registry impact was recorded in the `FB Review Log` instead of changing the rows

## Goal

Fix the current `3D View` startup bug where the app can stop at `WebGPU unavailable` before the actual renderer is even allowed to start.

Keep this lean:

- no renderer architecture change
- no WebGL fallback
- no scene-model change
- no Rust edit unless the browser-side fix proves insufficient

## Current Evidence

The screenshot shows:

- `WebGPU unavailable`
- `Type: WebGPU adapter unavailable`
- detail text beginning directly with `No suitable graphics adapter found ...`

That detail shape matters:

- `crates/render-wasm/src/lib.rs` prefixes the real renderer path with `Failed to request WebGPU adapter: ...`
- `probe_webgpu()` does not add that prefix

So the most likely current failure path is:

1. the app loads wasm
2. the app calls `probe_webgpu()`
3. the probe fails first
4. the app never reaches `create_renderer(...)`

That means the product bug is not only adapter availability. It is also the startup order:

- a brittle preflight probe is allowed to block the real renderer path

## Root Cause

`apps/web/src/three-d-viewport.tsx` treated the wasm probe as a required startup gate instead of using it only for capability detection.

Why that is risky:

- the browser already exposes `navigator.gpu`
- browser-side `requestAdapter()` is the most direct capability check available to the app shell
- `probe_webgpu()` is an extra wasm-specific preflight step
- if that step returns a false negative on a browser or profile, the app shows `WebGPU unavailable` without ever trying the real renderer creation path

## What Shipped

### 1. Browser-side capability probe

`apps/web/src/three-d-viewport.tsx`

- adds one small browser probe that calls `navigator.gpu.requestAdapter()` directly
- keeps the existing missing-API and no-adapter classifications
- uses the browser result only to decide whether startup should continue

### 2. Renderer startup no longer depends on `probe_webgpu()`

`apps/web/src/three-d-viewport.tsx`

- removes the blocking wasm preflight call from the init path
- goes straight from the browser capability check to `create_renderer(canvas)`
- preserves the existing typed error handling if real renderer creation still fails

## Verification

Run:

```bash
corepack pnpm --filter web test
corepack pnpm --filter web build
```

Observed results:

- `corepack pnpm --filter web test` passed with `36` tests green
- `corepack pnpm --filter web build` passed
- the web prebuild step still reused the checked-in `crates/render-wasm/pkg` artifacts because `wasm-pack` is not available on this machine
- manual smoke on `http://localhost:3001/editor` still reached the live `3D View` surface after switching tabs

Manual smoke:

1. open `/editor`
2. switch to `3D View`
3. confirm the page no longer depends on a separate wasm probe before trying renderer startup
4. on supported browsers, confirm the viewport still reaches the live 3D canvas
5. on unsupported browsers, confirm the UI still lands in the typed unsupported state

## Learnings

- a preflight probe should not be allowed to fail harder than the real runtime path it is supposedly checking
- browser capability checks belong in the browser shell when they do not require wasm-specific information
