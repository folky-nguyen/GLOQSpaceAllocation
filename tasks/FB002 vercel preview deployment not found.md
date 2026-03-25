# FB002 Vercel Preview Deployment NOT_FOUND

## Status

Implemented on `2026-03-25`.

Result:

- preview deployment no longer relies on implicit Vercel monorepo detection
- the repo now has an explicit Vercel build command and output directory
- SPA routes like `/`, `/login`, and `/editor` can be served through the frontend shell instead of failing at the platform edge
- web builds no longer require `wasm-pack` to exist in the deploy environment when `crates/render-wasm/pkg` is already present

## Goal

Explain why the preview deployment returned Vercel `404: NOT_FOUND`, capture the direct root cause, and record the smallest stable fix so this class of deploy issue is faster to recognize next time.

This remained a deploy-fix task only. No editor redesign, no API changes, and no domain-model changes were introduced.

## Final Root Cause

The failure was a platform routing problem, not a React render crash:

- the deployed URL returned Vercel `404: NOT_FOUND`
- the web app uses `BrowserRouter` and client-side routes like `/login` and `/editor`
- the repo is a monorepo and the web build output lives under `apps/web/dist`
- the repo did not have an explicit `vercel.json` telling Vercel what to build, what directory to serve, or how to treat SPA routes

That means the immediate outage cause was:

- Vercel could not reliably resolve the correct deployment artifact and route behavior for this frontend from the repo layout alone

The routing gap was the real product issue:

- the project depended on implicit Vercel configuration in a monorepo
- the frontend used client-side routing but had no SPA rewrite fallback
- auth redirects and direct deep links expected `/editor` to load the SPA shell first
- Vercel resolves URLs before React runs, so missing platform config produced a 404 before the app could boot

Follow-up deploy failure after that first fix:

- Vercel started the correct frontend build
- `apps/web` still ran a `prebuild` step that called `pnpm build:wasm`
- that script requires the `wasm-pack` binary
- the Vercel build image did not provide `wasm-pack`

That made the next immediate outage cause:

- the deployment depended on a local Rust-to-wasm packaging tool that was not available in the hosting environment

## What Shipped

### 1. Explicit Vercel deploy config

`vercel.json`

- added `buildCommand: "corepack pnpm --filter web build"`
- added `outputDirectory: "apps/web/dist"`

Why:

- this removes ambiguity about which workspace Vercel should build
- this makes the served static output match the actual Vite production build location

### 2. SPA rewrite fallback for client routes

`vercel.json`

- added a rewrite from `/(.*)` to `/index.html`

Why:

- Vercel treats incoming URLs as platform-level paths first
- the web app needs `index.html` to load before React Router can resolve `/login`, `/editor`, or any future client-side path

### 3. Checked-in wasm package fallback for deploy builds

`apps/web/package.json`

- changed `predev`, `predev:3001`, and `prebuild` to call `node ../../setup/ensure-render-wasm.mjs`

`setup/ensure-render-wasm.mjs`

- rebuilds `crates/render-wasm/pkg` when `wasm-pack` is available
- reuses the checked-in wasm package when `wasm-pack` is missing but the generated artifacts already exist
- fails with one explicit remediation message when neither condition is true

`.gitignore`

- stopped ignoring `crates/render-wasm/pkg/`

`crates/render-wasm/pkg/.gitignore`

- removed the blanket ignore rule so the generated wasm package can be committed

Why:

- local development should still rebuild the wasm package when the toolchain is present
- Vercel only needs the generated package files to build and serve the frontend
- this keeps the deploy path deterministic instead of assuming Rust packaging tools exist in every build environment

## Prevention Flow

Use this order when a Vercel preview URL shows `404: NOT_FOUND`:

1. confirm whether the 404 is coming from Vercel itself or from the app
2. check the project root directory, build command, and output directory in Vercel
3. verify whether the app uses client-side routing and therefore needs an SPA rewrite
4. confirm that auth callback or deep-link paths are routable through `index.html`

Keep the diagnosis simple:

- platform 404 first
- build/output mapping second
- SPA rewrite third
- app code only after those are proven correct

## Verification Run

Commands run during implementation:

```bash
corepack pnpm install
corepack pnpm run verify:web
```

Observed results:

- the web package production build passed locally
- the emitted frontend output was present in `apps/web/dist`
- the repo now contains explicit Vercel routing and output config in `vercel.json`
- the web package now has a deploy-safe wasm bootstrap path through `setup/ensure-render-wasm.mjs`

## Done Criteria Check

1. the deploy note identifies why Vercel returned `NOT_FOUND`: done
2. the smallest repo-level Vercel fix is documented: done
3. the SPA routing requirement is documented for future deploys: done
4. the missing-`wasm-pack` deploy failure and fallback path are documented: done

## Learnings

- If a preview URL shows Vercel `404: NOT_FOUND`, treat it as a deployment or platform-routing problem first, not a React rendering bug.
- In a monorepo, it is not enough that the app builds "somewhere". Vercel must know exactly which workspace to build and which output folder to serve.
- When the app uses `BrowserRouter`, the server or hosting platform must route app paths back to `index.html`. Without that fallback, direct visits or refreshes on `/editor` and `/login` will fail before React starts.
- Auth redirect URLs are routing tests in disguise. If Supabase sends the browser to `/editor`, that path must work as a platform entry point, not only as an in-app navigation target after the app has already loaded.
- The shortest diagnosis path for this class of issue is: confirm whether the 404 is platform-generated, then check root directory, build command, output directory, and SPA rewrites in that order.
- Implicit hosting defaults are fragile in small monorepos. They often appear to work until folder layout or project settings drift. Explicit deploy configuration is cheaper than repeated triage.
- A local Vite dev server can hide this class of problem because it already behaves like an SPA server. Production hosting has a different responsibility boundary and must be configured separately.
- If a web build imports generated wasm artifacts directly, the deploy path must either commit those artifacts or provision the exact packaging toolchain that regenerates them.
