# FB001 Editor Unreachable On Port 3001

## Status

Implemented on `2026-03-25`.

Result:

- `pnpm dev:3001` now type-checks before opening Vite on port `3001`
- the repo now has an explicit web build gate and an explicit `/editor` smoke check
- the repo now has a one-command bring-up path via `pnpm up:web:3001`
- local preview on `http://127.0.0.1:3001/editor` was revalidated with HTTP `200`

## Goal

Restore the local web editor so `http://127.0.0.1:3001/editor` is reachable again, identify the direct cause of the outage, and add the smallest sensible safeguards so this class of outage is caught before it reaches a developer screenshot again.

This remained a fix-bug task only. No editor redesign, no API changes, and no new dependencies were introduced.

## Final Root Cause

The browser screenshot was a transport-level failure, not a React crash:

- Chrome showed `ERR_CONNECTION_REFUSED`
- no process was listening on `127.0.0.1:3001` at the time of triage

That means the immediate outage cause was simple:

- the local web dev server for port `3001` was not running when the page was opened

The process gap was the real product issue:

- the repo had no explicit one-command smoke check for `/editor`
- the `3001` startup flow did not force a TypeScript check before Vite booted
- a developer could therefore discover the problem only after opening the browser
- there was no single command to both start the web app and wait until `/editor` was actually reachable

During early triage I also saw a transient broken working-tree state around `editor-shell.tsx`, but by the time the fix was implemented the current workspace build was green again. The confirmed outage tied to the screenshot was still the missing local server listener on port `3001`.

## What Shipped

### 1. Fail-fast startup for `3001`

`apps/web/package.json`

- `dev:3001` now runs `tsc --noEmit && vite --port 3001 --strictPort`

Why:

- if the web package has a broken import graph or type error, the `3001` flow now stops immediately with a concrete terminal error instead of waiting for browser discovery
- `--strictPort` remains in place so Vite never silently moves off `3001`

### 2. Explicit verification command for the web package

`package.json`

- added `pnpm verify:web`

What it does:

- runs `corepack pnpm --filter web build`

Why:

- it gives one explicit checked-build command for handoff and pre-merge validation

### 3. Explicit reachability smoke check for `/editor`

`package.json`

- added `pnpm smoke:web:3001`

`setup/check-web-3001.mjs`

- makes a plain HTTP request to `http://127.0.0.1:3001/editor`
- verifies both HTTP success and the expected GLOQ HTML marker
- fails fast on connection errors, timeouts, non-200 responses, or the wrong app answering on port `3001`
- prints the exact next step: start `pnpm up:web:3001`, then rerun the smoke check

`setup/README.md`

- documents the one helper script in the setup folder

Why:

- this closes the exact gap from the incident: we now have a deterministic command that proves the browser URL is actually reachable

### 4. One-command bring-up and shutdown for local preview

`package.json`

- added `pnpm up:web:3001`
- added `pnpm down:web:3001`

`setup/up-web-3001.mjs`

- starts the guarded `dev:3001` flow in the background
- waits until `http://127.0.0.1:3001/editor` responds
- writes a pid file and log file in `setup/`
- clears stale tracked processes before startup
- refuses to proceed if port cleanup does not actually release `3001`

`setup/down-web-3001.mjs`

- stops the tracked process and the real process currently listening on `3001`
- waits until port `3001` is actually free before reporting success

`setup/web-3001-runtime.mjs`

- holds the shared health-check and port-owner helpers used by `check`, `up`, and `down`

Why:

- this addresses the concrete lesson from the first pass: detection alone is not enough if the user still has to manually orchestrate startup every time

## Prevention Flow

Use this order for local preview on port `3001`:

1. run `pnpm up:web:3001`
2. run `pnpm smoke:web:3001`
3. before handoff or merge, run `pnpm verify:web`
4. stop the background server with `pnpm down:web:3001` when done

This is intentionally small:

- one guarded startup command
- one smoke check
- one checked build gate

No E2E framework, no watcher wrapper, and no duplicate config were added.

## Verification Run

Commands run during implementation:

```bash
corepack pnpm run verify:web
corepack pnpm --filter web test
corepack pnpm run up:web:3001
corepack pnpm run smoke:web:3001
corepack pnpm run down:web:3001
```

Observed results:

- `verify:web` passed
- web unit tests passed: `11` tests
- `up:web:3001` brought the web app up on port `3001`
- `smoke:web:3001` passed with HTTP `200` on `http://127.0.0.1:3001/editor`
- `smoke:web:3001` also failed correctly with a clear remediation message when no server was listening on port `3001`
- `down:web:3001` was tightened so it does not rely only on the tracked pid file; it also resolves the actual listener on `3001`

## Done Criteria Check

1. `pnpm dev:3001` serves the web app on port `3001`: done
2. `/editor` reachability is validated by command, not by guesswork: done
3. broken web startup assumptions fail earlier in terminal: done
4. the local outage triage path is documented: done
5. a minimal prevention path exists for future similar failures: done

## Kinh Nghiem

- `ERR_CONNECTION_REFUSED` on `127.0.0.1:3001` should be treated as a server-availability problem first, not a frontend-rendering bug.
- For this repo, the shortest diagnosis path is: check listener, run `pnpm up:web:3001`, run `pnpm smoke:web:3001`, then run `pnpm verify:web` if startup is still suspect.
- The first fix was incomplete because it improved detection but not recovery. For this class of outage, prevention must include both a smoke check and a one-command bring-up path.
- On Windows, pid-tree shutdown alone is not reliable enough for detached Vite flows. Shutdown must verify the real process that owns the listening port.
- A health check on the port is not enough by itself. It must also confirm that port `3001` is serving the GLOQ app, not just any HTTP process.
- `--strictPort` is necessary because silent port hopping creates false confidence and wastes debugging time.
- A local preview command without a type-check gate is too loose for handoff. `tsc --noEmit` before the guarded `3001` flow is a cheap and effective barrier.
- A tiny HTTP smoke script is enough here. Adding a heavier browser test stack for this class of outage would be unnecessary.
