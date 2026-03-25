# 005 Supabase Auth In Web

## Goal

Add Supabase Auth to `apps/web` with browser-safe environment variables only.

The web app must support:

- session bootstrap on app load
- minimal login UI with email OTP or magic link
- logout
- a protected editor route
- auth state in one tiny dedicated module

Non-goal:

- never expose `service_role` in browser code, browser env files, or Vite client bundles

## Current Repo State

`apps/web` is currently a single-page Vite + React app with no routing and no auth wiring.

Relevant files today:

- `apps/web/src/main.tsx`
  - mounts `App` directly
- `apps/web/src/App.tsx`
  - contains the full editor shell UI
- `apps/web/src/ui-store.ts`
  - owns editor-only UI state through Zustand
- `apps/web/package.json`
  - has React, Vite, and Zustand only

There is no existing:

- `@supabase/supabase-js`
- `react-router-dom`
- auth module
- env example file for the web app
- route protection layer

Local web origins already used by the repo:

- `http://localhost:5173`
- `http://127.0.0.1:5173`
- `http://localhost:3001`
- `http://127.0.0.1:3001`

Those matter for Supabase redirect URL setup.

## Updated Context

- Repo is a `pnpm` workspace.
- The relevant frontend package is `apps/web`.
- `apps/web` uses Vite and already supports both local dev ports:
  - default Vite flow on `5173`
  - alternate flow on `3001` through the existing workspace scripts
- `apps/web/src/App.tsx` currently contains the full editor shell, so auth routing should be layered around that shell rather than woven into `ui-store.ts`.
- `apps/web/src/ui-store.ts` is already reserved for editor chrome state only.
- `apps/web/src/ui-store.ts` is module-scoped Zustand state, so it will survive route changes and sign-out unless the app resets it explicitly.
- The repo does not currently expose any Supabase browser config in source control, so auth setup must come from local env only.
- `supabase/config.toml` currently does not provide browser runtime keys, so the web app must not try to infer them from local Supabase config files.
- Because the app is already running in React Strict Mode, auth bootstrap and auth subscription setup must be idempotent.
- OTP completion cannot assume the user stays on the same `/login` page instance after requesting the email. Refresh, new-tab open, or coming back from the inbox are realistic flows.

## Scope

In scope:

- browser Supabase client setup
- passwordless email login flow
- session bootstrap and auth subscription
- protected `/editor` route
- public `/login` route
- logout affordance inside the editor shell
- one tiny auth state module outside Zustand
- minimal env and setup documentation for the web app

Out of scope:

- server-side auth rendering
- Rust API auth enforcement
- profile management
- invite flows
- social providers
- role-aware UI
- moving editor document state out of TypeScript

## Implementation Decisions

### 1. Browser-safe env vars

Use exactly these web env vars in `apps/web`:

- `VITE_SUPABASE_URL`
- `VITE_SUPABASE_PUBLISHABLE_KEY`

Why:

- Vite only exposes `VITE_`-prefixed vars to browser code
- Supabase publishable keys are intended for public clients
- this avoids training the frontend to accept multiple key names

Do not add:

- `SUPABASE_SERVICE_ROLE_KEY`
- `VITE_SUPABASE_SERVICE_ROLE_KEY`
- any fallback that silently reads server-only secrets

If a project is still using a legacy anon key, map that value into `VITE_SUPABASE_PUBLISHABLE_KEY` locally instead of teaching the app a second canonical env name.

### 2. One tiny dedicated auth module

Add one dedicated module at `apps/web/src/auth.ts`.

This module should own:

- Supabase browser client creation
- runtime env validation
- current auth snapshot
- subscribe/unsubscribe mechanism
- bootstrap logic
- passwordless sign-in function
- OTP verify function
- logout function
- a small `useAuth()` hook built on `useSyncExternalStore`

Keep auth state out of:

- `ui-store.ts`
- React context layers unless the module truly needs one

Reason:

- auth is cross-cutting app session state, not editor chrome state
- a small external store keeps file count and abstraction count low

### 3. Route shape

Introduce routing with the minimum useful surface:

- `/login`
- `/editor`
- `/`

Behavior:

- `/` redirects to `/editor` when authenticated
- `/` redirects to `/login` when signed out
- `/editor` is protected
- `/login` stays public but redirects away once a valid session exists

Use `react-router-dom` for this instead of a hand-rolled pathname switch.

Reason:

- route protection and redirects become explicit
- the dependency is justified by the protected route requirement
- it keeps the solution smaller than custom navigation glue

### 4. Session bootstrap on app load

Bootstrap auth once during app startup.

Concrete flow:

1. create the Supabase browser client from validated `VITE_` env vars
2. call `supabase.auth.getSession()`
3. set auth snapshot to one of:
   - `loading`
   - `signed_out`
   - `signed_in`
4. register `supabase.auth.onAuthStateChange(...)`
5. update the snapshot whenever the session changes

The bootstrap function must be idempotent.

Reason:

- React Strict Mode can mount effects twice in development
- duplicate subscriptions would create confusing auth state races

Module-level guards are enough:

- one bootstrap promise
- one auth subscription

### 5. Passwordless login must cover OTP and magic link

Use Supabase passwordless email auth only:

- send email via `supabase.auth.signInWithOtp({ email, options: { emailRedirectTo } })`
- verify typed OTP via `supabase.auth.verifyOtp({ email, token, type: "email" })`

This supports both allowed UX modes:

- magic link when the Supabase email template uses the confirmation URL
- OTP when the Supabase email template uses the token

Minimal login UI shape:

- email field
- `Send link or code` button
- success notice after send
- optional OTP input + `Verify code` button
- one compact error message region

Important behavior:

- OTP verification must still work when `pendingEmail` in memory is empty
- the form must allow verification from the typed email after a refresh or a new tab

Reason:

- a single screen covers both Supabase email configurations
- no second route is needed for auth callback handling

### 6. Redirect target

Use `/editor` as the redirect target after successful passwordless auth.

`emailRedirectTo` should be computed from the current origin:

- `${window.location.origin}/editor`

Reason:

- it works for both default dev port `5173` and alternate port `3001`
- it lands the user directly on the protected app surface

### 7. Protected route behavior

The protected route should have only three states:

1. `loading`
   - render a bare loading screen
2. `signed_out`
   - redirect to `/login`
3. `signed_in`
   - render the editor shell

Do not hide auth checks deep inside the editor shell.

Reason:

- route gating should happen at the route boundary
- editor code stays focused on editor UI

### 8. Logout placement

Add logout as one minimal control in the existing shell chrome.

Recommended placement:

- right side of the top ribbon, beside the existing summary

UI content can stay small:

- current user email
- `Log out` button

Behavior requirement:

- logout must also reset editor session UI state so the next signed-in user does not inherit the prior session's active tool/view/selection

Do not add:

- account dropdown
- avatar system
- settings page

### 9. Keep editor shell mostly intact

The current editor shell in `App.tsx` is large enough that auth routing should not be mixed into it directly.

Smallest clean split:

- move the current shell markup into `apps/web/src/editor-shell.tsx`
- repurpose `apps/web/src/App.tsx` as the route composition entry

This preserves:

- existing editor UI behavior
- existing `ui-store.ts`
- one shared TypeScript document model for 2D and 3D

## File Plan

### 1. `apps/web/package.json`

Add only these dependencies:

- `@supabase/supabase-js`
- `react-router-dom`

No extra state, form, or UI libraries.

### 2. `apps/web/.env.example`

Add a minimal example file:

```env
VITE_SUPABASE_URL=
VITE_SUPABASE_PUBLISHABLE_KEY=
```

Do not include service role examples in this file.

### 3. `apps/web/src/auth.ts`

Add the small dedicated auth module.

Suggested surface:

```ts
export type AuthStatus = "loading" | "signed_out" | "signed_in";

export type AuthSnapshot = {
  status: AuthStatus;
  session: Session | null;
  user: User | null;
  error: string | null;
  pendingEmail: string;
};

export function bootstrapAuth(): Promise<void>;
export function useAuth(): AuthSnapshot;
export function subscribeAuth(listener: () => void): () => void;
export async function sendLoginEmail(email: string): Promise<{ error: string | null }>;
export async function verifyEmailOtp(email: string, token: string): Promise<{ error: string | null }>;
export async function logout(): Promise<{ error: string | null }>;
```

Implementation notes:

- validate env eagerly and fail with a clear browser-side error if missing
- keep snapshot updates centralized in one `setSnapshot()` helper
- do not export raw mutable state

### 4. `apps/web/src/main.tsx`

Update startup to:

- mount the router
- trigger `bootstrapAuth()` once

The bootstrap can happen:

- immediately before render, or
- in a tiny startup component with one effect

Either is acceptable as long as it is idempotent and does not duplicate subscriptions.

### 5. `apps/web/src/App.tsx`

Refactor into route composition:

- public login route
- protected editor route
- root redirect

Keep `ProtectedRoute` local to this file unless it grows beyond a few lines.

### 6. `apps/web/src/editor-shell.tsx`

Move the current editor shell here with the smallest possible diff.

Only auth-specific additions should be:

- user email display
- logout button

The rest of the shell should remain unchanged.

### 7. `apps/web/src/styles.css`

Add only the minimal styles needed for:

- centered login card
- loading screen
- small auth action row in the ribbon

Do not restyle the editor shell broadly.

## Experience Notes

- Put the browser auth config in Vite env only. Do not add parallel config paths in TypeScript constants, root scripts, or Supabase config files.
- Keep the canonical browser key name as `VITE_SUPABASE_PUBLISHABLE_KEY`. Do not normalize multiple public key names in app code, or future setup drifts become harder to diagnose.
- Never expose `service_role` in any browser-facing file. If a value is accessible through `import.meta.env`, treat it as public.
- Compute `emailRedirectTo` from `window.location.origin` and append `/editor`. This avoids hardcoding separate redirect URLs for `5173` and `3001`.
- Use one login screen for both magic link and OTP. Splitting them into separate routes adds state and edge cases without helping the MVP.
- Do not couple OTP verification availability to in-memory `pendingEmail` alone. Users must still be able to paste a code after a refresh or from a different tab by re-entering their email.
- Gate `/editor` at the router boundary, not deep inside the editor shell. This keeps auth concerns from leaking into the editor module.
- Keep auth state out of Zustand. The existing Zustand store is scoped to editor interaction state, and mixing auth into it would blur responsibilities.
- Reset the editor UI store on logout. Module-scoped Zustand state otherwise leaks view/tool/selection across auth sessions in the same browser process.
- Guard `bootstrapAuth()` against duplicate execution. React Strict Mode can otherwise create duplicate `onAuthStateChange` subscriptions during development.
- When env is missing or invalid, fail loudly with a visible UI error state. Silent fallback behavior will look like a broken redirect loop.
- After changing allowed dev ports or local hostnames, update the Supabase redirect allow list immediately or passwordless auth will appear flaky.

## Supabase Project Setup Notes

This task depends on one small Supabase dashboard setup.

### Redirect URLs

Allow the existing local origins to redirect into `/editor`:

- `http://localhost:5173/editor`
- `http://127.0.0.1:5173/editor`
- `http://localhost:3001/editor`
- `http://127.0.0.1:3001/editor`

Also add the eventual production `/editor` URL when it exists.

### Email template mode

Supabase passwordless email behavior depends on the email template:

- magic link mode uses the confirmation URL
- OTP mode uses the token

This plan deliberately supports both without branching the app into separate auth screens.

## Verification Plan

### Build

Run the smallest relevant web check:

```bash
corepack pnpm --filter web build
```

### Manual smoke test

1. Set `apps/web/.env` from `.env.example`
2. Run either:
   - `corepack pnpm --filter web dev`
   - `corepack pnpm dev:3001`
3. Open `/login`
4. Submit an email address
5. Confirm the app shows a sent state
6. Complete auth by either:
   - clicking the magic link, or
   - entering the OTP code
7. Refresh `/login` after the email is sent, then verify that OTP can still be submitted by re-entering the email if needed
8. Confirm the app lands on `/editor`
9. Refresh the page and confirm the session is restored
10. Change the active tool/view/selection, then hit `Log out`
11. Confirm the app returns to `/login`
12. Sign in again and confirm editor UI state resets to its default session values
13. Visit `/editor` while signed out and confirm redirect protection still works

## Done Criteria

This task is complete when:

1. the browser app uses only `VITE_SUPABASE_URL` and `VITE_SUPABASE_PUBLISHABLE_KEY`
2. no service-role key is referenced anywhere in `apps/web`
3. auth state lives in one tiny dedicated module, not in Zustand
4. app startup restores an existing Supabase session
5. `/login` can send a passwordless email
6. the same login screen can complete either magic-link or OTP sign-in
7. `/editor` is route-protected
8. logout clears the session and returns the user to `/login`
9. `corepack pnpm --filter web build` passes
