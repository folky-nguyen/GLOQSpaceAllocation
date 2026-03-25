# QC

Behavioral regression log.

Use this file for repeated user-facing misbehavior, not for implementation details. Write each entry as: action, wrong behavior, expected behavior.

## Repeated Regressions

- Auth restore:
  - Action: sign in, land on `/editor`, then refresh or open `/editor` directly.
  - Wrong behavior: the app falls back to `/login`, loops forever, or stays stuck in loading.
  - Expected behavior: the session restores and the protected editor route opens.

- Logout isolation:
  - Action: change active tool, view, or selection, then log out and sign in again.
  - Wrong behavior: the new session inherits stale editor chrome state from the previous session.
  - Expected behavior: logout clears the auth session and resets editor session UI state.

- Imperial shorthand input:
  - Action: enter feet-inch input such as `12'6"`, `12 3 3/4`, or `7''`.
  - Wrong behavior: the parser rejects supported shorthand or normalizes to inconsistent output.
  - Expected behavior: supported shorthand parses successfully and normalizes to canonical ft-in output.

- View-state parity:
  - Action: switch views from the ribbon, workspace tabs, or project browser.
  - Wrong behavior: the active view, selection label, and status bar drift out of sync.
  - Expected behavior: all shell surfaces reflect the same `activeView` and selected view state.

- API auth surface:
  - Action: call `GET /api/me` with a valid Supabase Bearer token after the API is already running.
  - Wrong behavior: public routes work, but `/api/me` fails because header parsing or JWKS refresh drifted.
  - Expected behavior: `/api/me` accepts valid Bearer tokens and continues to work across JWKS cache refreshes.
