# FB003 3D Adapter Typed Message And pnpm esbuild Warning

## Status

Implemented on `2026-03-26`.

Result:

- `3D View` now tells the user what class of failure happened instead of only dumping the raw `wgpu` message
- no-adapter environments still land in the stable `WebGPU unavailable` state, but now include an explicit `Type` line
- the repo now explicitly allows the trusted `esbuild` install script, so the `pnpm` warning path has a repo-level fix instead of relying on per-machine approval

## Goal

Capture the current `3D View` failure report as `FB003`, separate the product bug from the install warning, land the smallest message-level and repo-level fixes, and record a prevention plan so future screenshots are faster to triage.

This stayed lean:

- no renderer rewrite
- no WebGL fallback
- no new dependencies
- no scene-model changes

## Final Root Cause

The two screenshots show two different problems and they should not be treated as one bug.

### 1. `3D View` screenshot

The `3D View` screenshot is not evidence of broken floor-plan data.

What it shows instead:

- the browser reached the wasm renderer startup path
- `navigator.gpu` was exposed strongly enough for the app to attempt WebGPU startup
- the device or browser session did not return a usable WebGPU adapter
- the UI still surfaced mostly raw `wgpu` text, so the user could see the low-level detail but not the error category quickly

That means the product gap was:

- the app knew the failure details
- but it did not label the failure type clearly enough for fast human triage

### 2. `pnpm` warning screenshot

The `pnpm` warning is a dependency-install policy issue, not a renderer crash.

What it shows:

- `pnpm` blocked the `esbuild` build script during install
- the repo did not yet declare `esbuild` in `pnpm.onlyBuiltDependencies`
- the result was a recurring warning asking a developer to run `pnpm approve-builds`

That means the repo gap was:

- a trusted build dependency was left to machine-local approval instead of being allowed explicitly in versioned config

## What Shipped

### 1. Typed 3D error messages

`apps/web/src/three-d-viewport.tsx`

- keeps the existing `unsupported` vs `error` phase split
- adds a typed issue model with a summary, type label, and detail line
- maps startup failures into clearer categories instead of only echoing raw exception text

Message types now include:

- `Browser missing WebGPU API`
- `WebGPU adapter unavailable`
- `Renderer package mismatch`
- `3D renderer startup failed`
- `3D renderer draw failed`

Why:

- the user can now tell whether this is a browser capability problem, an adapter problem, a stale wasm package, or a real renderer crash
- the raw detail is still preserved for debugging

### 2. UI support for an explicit error type line

`apps/web/src/styles.css`

- adds styling for the new `Type:` line and detail line in the `3D View` state card

Why:

- the message needed to stay readable without turning the overlay into a dense debug dump

### 3. Repo-level fix for the `esbuild` install warning

`package.json`

- adds `pnpm.onlyBuiltDependencies` with `esbuild`

Why:

- `esbuild` is a known trusted dependency in this repo through the Vite toolchain
- the warning should be solved once in repo config instead of repeated manually on every machine

## Fix Plan That Was Applied

### Bug fix plan

1. keep the existing `unsupported` state for no-adapter failures instead of reclassifying them as generic crashes
2. convert renderer failures into a typed message model
3. show a short human summary first
4. show `Type: ...` on the overlay so the screenshot itself reveals the failure category
5. keep the raw detail line underneath for deeper debugging

### Warning fix plan

1. confirm the warning source with `pnpm ignored-builds`
2. use the repo-level allowlist that `pnpm` itself recommends
3. rebuild the affected package once so the local install state matches repo policy
4. re-run `pnpm ignored-builds` to confirm the warning is gone

## Prevention Plan

Use this order the next time `3D View` fails on startup:

1. read the `Type:` line first
2. if the type is `Browser missing WebGPU API`, treat it as browser capability
3. if the type is `WebGPU adapter unavailable`, treat it as browser or device support
4. if the type is `Renderer package mismatch`, run `pnpm build:wasm` and restart the web app
5. if the type is `3D renderer startup failed` or `3D renderer draw failed`, treat it as a real renderer bug and inspect the detail line

Use this order the next time install output shows a build-script warning:

1. run `corepack pnpm ignored-builds`
2. if the blocked package is expected and trusted, add it to `pnpm.onlyBuiltDependencies`
3. run `corepack pnpm rebuild <package-name>`
4. re-run `corepack pnpm ignored-builds`

## Verification Plan

Run:

```bash
corepack pnpm rebuild esbuild
corepack pnpm ignored-builds
corepack pnpm --filter web test
corepack pnpm --filter web build
```

Manual smoke:

1. open `/editor`
2. switch to `3D View`
3. confirm the overlay now shows `Type: ...` for unsupported or error states
4. confirm the message still keeps the lower-level detail line for debugging
5. confirm the install flow no longer reports `Ignored build scripts: esbuild`

## Learnings

- raw low-level error text is not enough for bug triage if the screenshot does not also reveal the failure class
- a user-facing message should answer `what kind of failure is this?` before it answers `what exact subsystem string came back?`
- repo-trusted build tools should be approved in versioned config, not left to repeated machine-local approval prompts
