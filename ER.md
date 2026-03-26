# ER

Registry for trapped error messages that are intentionally surfaced in the product. Use this file to jump from a screenshot or copied message to the owning code path quickly.

## How To Use It

- Update this file after every `FB...` task.
- If an `FB...` task adds, removes, renames, or reroutes a trapped error message, update the registry rows.
- If an `FB...` task does not change canonical registry rows, add one short note in `FB Review Log` so the ER review is still explicit.
- Do not update this file for ordinary non-bug-fix tasks unless the user explicitly asks for it.
- One row should represent one canonical primary message that a human can search for quickly.
- `Code` should stay stable once introduced. Prefer `WEB-<surface>-<nnn>`, `API-<surface>-<nnn>`, or `SETUP-<surface>-<nnn>`.
- Placeholder-based messages may use `<NAME>`, `<VALUE>`, `<URL>`, `<STATUS>`, and `<TIMEOUT_MS>` in the registry when the real message is formatted dynamically in code.
- Primary trapped messages should be sentence case and end with a period.
- Put the human summary first. Do not let raw vendor, browser, or library text replace the primary message.
- If the UI has a separate detail slot, keep low-level detail there.
- If the UI only has one error string, append the low-level detail after `Detail:` so the canonical summary still stays first.
- For typed overlays that already split `Title`, `Type`, and `Detail`, register the summary sentence here and keep the type label short and title case in code.
- `Path` should point to the source file that traps or emits the message.

## Registry

| Code | Error message | Path file |
| --- | --- | --- |
| `WEB-LOGIN-001` | `Email is required.` | [`apps/web/src/App.tsx`](./apps/web/src/App.tsx) |
| `WEB-LOGIN-002` | `Password is required.` | [`apps/web/src/App.tsx`](./apps/web/src/App.tsx) |
| `WEB-LOGIN-003` | `OTP code is required.` | [`apps/web/src/App.tsx`](./apps/web/src/App.tsx) |
| `WEB-LOGIN-004` | `New password is required.` | [`apps/web/src/App.tsx`](./apps/web/src/App.tsx) |
| `WEB-AUTH-001` | `Supabase browser auth is not configured. Add VITE_SUPABASE_URL and VITE_SUPABASE_PUBLISHABLE_KEY.` | [`apps/web/src/auth.ts`](./apps/web/src/auth.ts) |
| `WEB-AUTH-002` | `Could not restore the current session.` | [`apps/web/src/auth.ts`](./apps/web/src/auth.ts) |
| `WEB-AUTH-003` | `Could not sign in with email and password.` | [`apps/web/src/auth.ts`](./apps/web/src/auth.ts) |
| `WEB-AUTH-004` | `Could not create the account.` | [`apps/web/src/auth.ts`](./apps/web/src/auth.ts) |
| `WEB-AUTH-005` | `Could not send the recovery email.` | [`apps/web/src/auth.ts`](./apps/web/src/auth.ts) |
| `WEB-AUTH-006` | `Could not verify the recovery code.` | [`apps/web/src/auth.ts`](./apps/web/src/auth.ts) |
| `WEB-AUTH-007` | `Could not verify the email code.` | [`apps/web/src/auth.ts`](./apps/web/src/auth.ts) |
| `WEB-AUTH-008` | `Could not open the password reset session.` | [`apps/web/src/auth.ts`](./apps/web/src/auth.ts) |
| `WEB-AUTH-009` | `Could not update the password.` | [`apps/web/src/auth.ts`](./apps/web/src/auth.ts) |
| `WEB-AUTH-010` | `Password updated, but could not sign out.` | [`apps/web/src/auth.ts`](./apps/web/src/auth.ts) |
| `WEB-AUTH-011` | `Could not sign out.` | [`apps/web/src/auth.ts`](./apps/web/src/auth.ts) |
| `WEB-LEVEL-001` | `Stories below grade must be a whole number greater than or equal to 0.` | [`apps/web/src/editor-shell.tsx`](./apps/web/src/editor-shell.tsx) |
| `WEB-LEVEL-002` | `Stories on grade must be a whole number greater than or equal to 0.` | [`apps/web/src/editor-shell.tsx`](./apps/web/src/editor-shell.tsx) |
| `WEB-LEVEL-003` | `Auto-generate requires at least one story.` | [`apps/web/src/editor-shell.tsx`](./apps/web/src/editor-shell.tsx) |
| `WEB-LEVEL-004` | `Story height must be a positive feet-inch value.` | [`apps/web/src/editor-shell.tsx`](./apps/web/src/editor-shell.tsx) |
| `WEB-SITE-001` | `Site boundary must include at least 3 valid points.` | [`apps/web/src/project-doc.ts`](./apps/web/src/project-doc.ts) |
| `WEB-SITE-002` | `Site boundary must enclose a valid area.` | [`apps/web/src/project-doc.ts`](./apps/web/src/project-doc.ts) |
| `WEB-SITE-003` | `Site boundary cannot contain a zero-length edge.` | [`apps/web/src/project-doc.ts`](./apps/web/src/project-doc.ts) |
| `WEB-SITE-004` | `Setbacks must resolve to a valid building footprint.` | [`apps/web/src/project-doc.ts`](./apps/web/src/project-doc.ts) |
| `WEB-SITE-005` | `Setbacks collapse the building footprint.` | [`apps/web/src/project-doc.ts`](./apps/web/src/project-doc.ts) |
| `WEB-SITE-006` | `Setbacks exceed the available site depth.` | [`apps/web/src/project-doc.ts`](./apps/web/src/project-doc.ts) |
| `WEB-SITE-007` | `Setback must be greater than or equal to 0.` | [`apps/web/src/editor-shell.tsx`](./apps/web/src/editor-shell.tsx) |
| `WEB-3D-001` | `This browser does not expose \`navigator.gpu\`, so the wasm renderer cannot start.` | [`apps/web/src/three-d-viewport.tsx`](./apps/web/src/three-d-viewport.tsx) |
| `WEB-3D-002` | `The browser exposed WebGPU, but this device did not return a usable graphics adapter.` | [`apps/web/src/three-d-viewport.tsx`](./apps/web/src/three-d-viewport.tsx) |
| `WEB-3D-003` | `The web app and the checked-in wasm renderer package are out of sync.` | [`apps/web/src/three-d-viewport.tsx`](./apps/web/src/three-d-viewport.tsx) |
| `WEB-3D-004` | `The wasm renderer threw before the first frame could be drawn.` | [`apps/web/src/three-d-viewport.tsx`](./apps/web/src/three-d-viewport.tsx) |
| `WEB-3D-005` | `The renderer started, but failed while sending the current scene to WebGPU.` | [`apps/web/src/three-d-viewport.tsx`](./apps/web/src/three-d-viewport.tsx) |
| `API-AUTH-001` | `Authentication required.` | [`apps/api/src/auth.rs`](./apps/api/src/auth.rs) |
| `API-AUTH-002` | `Authentication is temporarily unavailable.` | [`apps/api/src/auth.rs`](./apps/api/src/auth.rs) |
| `API-CONFIG-001` | `Missing required environment variable <NAME>.` | [`apps/api/src/config.rs`](./apps/api/src/config.rs) |
| `API-CONFIG-002` | `Environment variable <NAME> must be valid Unicode.` | [`apps/api/src/config.rs`](./apps/api/src/config.rs) |
| `API-CONFIG-003` | `API_PORT must be a valid u16, got <VALUE>.` | [`apps/api/src/config.rs`](./apps/api/src/config.rs) |
| `SETUP-WEB-001` | `Port 3001 is still occupied after cleanup.` | [`setup/up-web-3001.mjs`](./setup/up-web-3001.mjs) |
| `SETUP-WEB-002` | `Inspect the existing listener before retrying <URL>.` | [`setup/up-web-3001.mjs`](./setup/up-web-3001.mjs) |
| `SETUP-WEB-003` | `Web failed to start on <URL> within <TIMEOUT_MS>ms.` | [`setup/up-web-3001.mjs`](./setup/up-web-3001.mjs) |
| `SETUP-WEB-004` | `Inspect the startup log at <PATH>.` | [`setup/up-web-3001.mjs`](./setup/up-web-3001.mjs) |
| `SETUP-WEB-005` | `Port 3001 is still occupied after shutdown.` | [`setup/down-web-3001.mjs`](./setup/down-web-3001.mjs) |
| `SETUP-WEB-006` | `Smoke check failed: <URL> returned <STATUS>.` | [`setup/check-web-3001.mjs`](./setup/check-web-3001.mjs) |
| `SETUP-WEB-007` | `The endpoint must serve the GLOQ web app, not just any process on port 3001.` | [`setup/check-web-3001.mjs`](./setup/check-web-3001.mjs) |
| `SETUP-WEB-008` | `Smoke check failed: could not reach <URL>.` | [`setup/check-web-3001.mjs`](./setup/check-web-3001.mjs) |
| `SETUP-WEB-009` | `Start or restore the local preview with \`pnpm up:web:3001\`, then rerun \`pnpm smoke:web:3001\`.` | [`setup/check-web-3001.mjs`](./setup/check-web-3001.mjs) |
| `SETUP-WASM-001` | `wasm-pack is not available and crates/render-wasm/pkg is missing required artifacts.` | [`setup/ensure-render-wasm.mjs`](./setup/ensure-render-wasm.mjs) |
| `SETUP-WASM-002` | `Run \`pnpm build:wasm\` on a machine with wasm-pack, then commit crates/render-wasm/pkg.` | [`setup/ensure-render-wasm.mjs`](./setup/ensure-render-wasm.mjs) |

## FB Review Log

- `2026-03-26` `FB005`: reviewed `WEB-3D-001` and `WEB-3D-002` in [`apps/web/src/three-d-viewport.tsx`](./apps/web/src/three-d-viewport.tsx). The startup order changed from a blocking wasm probe to a browser-side WebGPU probe, but the canonical trapped messages and owning file stayed the same, so the registry rows did not change.
