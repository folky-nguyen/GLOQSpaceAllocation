# 006 Supabase Bearer JWT Auth

## Goal

In `apps/api`, add minimal Bearer JWT authentication for Supabase access tokens:

- verify JWT signatures against the project's JWKS endpoint
- cache JWKS briefly in memory
- extract the authenticated user ID from `sub`
- expose `email` and `role` when present
- attach auth context to request extensions
- add `GET /api/me`

This should stay as thin API plumbing. Do not introduce a full auth framework, session layer, or a second user/domain model.

## Current Repo State

`apps/api` currently has:

- `src/config.rs` with `API_HOST`, `API_PORT`, and `DATABASE_URL`
- `src/error.rs` with JSON error responses for `not_found` and `internal_error`
- `src/main.rs` with:
  - `AppState { pool: PgPool }`
  - `GET /api/health`
  - `GET /api/version`
  - CORS and trace middleware

What is missing today:

- no auth-related config
- no auth middleware
- no request extensions carrying user context
- no `GET /api/me`
- no HTTP client or JWT verification dependency

## Updated Context

- Repo is a Rust workspace with `apps/api` and `crates/render-wasm`, and the current API crate already follows the shared workspace dependency pattern.
- `apps/api` already has a thin shape:
  - config loading in `src/config.rs`
  - JSON HTTP errors in `src/error.rs`
  - a single `src/main.rs` that wires router, tracing, CORS, and Postgres
- `GET /api/health` and `GET /api/version` are already public under `/api`, so auth should be added as a narrow extension to that surface, not as a whole-app framework rewrite.
- `AppState` currently carries a live `PgPool`, and the health route already proves the API is willing to keep state small and concrete rather than introducing service containers.
- `supabase/config.toml` currently exposes only `project_id = "gloq-space-allocation"`. It does not provide runtime URLs, JWT keys, or server env defaults, so the API must read its Supabase runtime origin from environment instead of inferring it from repo files.
- Plan `005` already moves browser auth into `apps/web` with Supabase Auth. That means the API-side task here is specifically about verifying the access token the web app will already have, not about recreating login flows on the server.
- The persistence direction in earlier tasks is still JSONB snapshots plus thin metadata. Auth in `apps/api` should stay equally thin: verify the caller, attach identity, and let future endpoints use that context.
- The repo currently has no generic auth abstraction, no shared user model, and no claim mapping layer. The smallest sensible path is to keep all auth code local to `apps/api`.

## Scope

In scope:

- Bearer token parsing from `Authorization`
- Supabase JWT verification via JWKS
- small in-memory JWKS cache
- request-scoped auth context
- protected `GET /api/me`
- route tests for critical auth behavior

Out of scope:

- refresh tokens
- login/signup/logout flows
- session cookies
- RBAC policy engine
- a reusable auth framework
- database-backed token storage
- client-side auth changes

## Decisions

### 1. Keep auth in one small module

Add a single `apps/api/src/auth.rs` module that owns:

- auth context types
- JWT claims parsing
- JWKS fetch/cache logic
- Bearer middleware

Do not create `auth/`, `middleware/`, `services/`, or `extractors/` folders for this task.

### 2. Add one Supabase env input

Add required env config:

- `SUPABASE_URL`

Expected shape:

```text
https://<project-ref>.supabase.co
```

Derive these values from it instead of adding more env surface:

- issuer: `SUPABASE_URL + "/auth/v1"`
- JWKS URL: `SUPABASE_URL + "/auth/v1/.well-known/jwks.json"`

This keeps config small while still letting the API validate the token issuer and fetch the correct JWKS document.

### 3. Use a short in-memory JWKS cache

Use a process-local cache inside the auth verifier:

- cache keyed by `kid`
- cache TTL fixed in code at `60` seconds
- no background refresh thread

Refresh behavior:

1. read JWT header to get `kid`
2. try cached key
3. if cache is stale, empty, or does not contain that `kid`, fetch JWKS once
4. retry key lookup after refresh

This is enough for the MVP and also handles key rotation without adding infrastructure.

### 4. Verify only what the API actually needs

Token validation should require:

- valid `Authorization: Bearer <token>` header
- matching signature from the Supabase JWKS
- valid `iss`
- valid `exp` and other standard time checks provided by the JWT library
- present `sub`

Claims handling:

- `sub` is required and parsed into a UUID user ID
- `email` is optional
- `role` is optional

Do not build a broader claim model than this.

### 5. Attach auth context to request extensions

On successful verification, middleware should insert:

```rust
AuthContext {
    user_id,
    email,
    role,
}
```

into `request.extensions_mut()`.

Handlers can then read it through `Extension<AuthContext>` or directly from extensions. This satisfies the requirement without introducing a custom auth extractor framework.

### 6. Protect only the route that needs auth

Keep:

- `GET /api/health` public
- `GET /api/version` public

Add:

- `GET /api/me` protected by the Bearer middleware

Do not put global auth middleware around the whole `/api` router yet. Route-level protection is the smallest change and avoids future exceptions for public endpoints.

### 7. Extend the existing error shape instead of replacing it

Add one more helper to `ApiError`:

- `unauthorized`

Use the existing JSON envelope:

```json
{
  "error": {
    "code": "unauthorized",
    "message": "Authentication required."
  }
}
```

Return `401` for:

- missing `Authorization` header
- non-Bearer scheme
- malformed JWT
- unknown `kid`
- failed signature verification
- missing or invalid `sub`

Keep the message generic so the API does not leak token validation details.

## Dependency Plan

Add only the minimum new Rust dependencies:

- `reqwest` for fetching JWKS over HTTPS
- `jsonwebtoken` for JWT parsing and signature validation

Keep everything else on the standard library or existing crates:

- use `std::sync::{Arc, Mutex}` or `RwLock` for cache state
- use existing `serde`
- use existing `sqlx` UUID support for `sub` parsing

Do not add:

- OAuth/OpenID client frameworks
- session/auth middleware suites
- a custom crypto stack if `jsonwebtoken` can consume the JWKS data directly

## File-by-File Plan

### 1. Root workspace manifest

Update `Cargo.toml`:

- add `reqwest` to `[workspace.dependencies]`
- add `jsonwebtoken` to `[workspace.dependencies]`

Recommended `reqwest` shape:

- `default-features = false`
- features:
  - `json`
  - `rustls-tls`

### 2. API crate manifest

Update `apps/api/Cargo.toml`:

- opt into `reqwest.workspace = true`
- opt into `jsonwebtoken.workspace = true`

No other crate additions are needed for this task.

### 3. Config module

Update `apps/api/src/config.rs`:

- add `supabase_url: String` to `AppConfig`
- require `SUPABASE_URL`
- normalize by trimming a trailing slash once at load time

Add tiny helpers on `AppConfig`:

- `supabase_issuer(&self) -> String`
- `supabase_jwks_url(&self) -> String`

Do not add a generic URL config system.

### 4. Error module

Update `apps/api/src/error.rs`:

- add `ApiError::unauthorized(...)`

Keep the existing response structure and `IntoResponse` implementation.

### 5. New auth module

Create `apps/api/src/auth.rs` with the minimum surface:

- `AuthContext`
- `JwtClaims`
- `AuthVerifier`
- `require_bearer_auth` middleware function
- small helpers for:
  - parsing the Bearer header
  - reading the JWT header `kid`
  - reading/updating the JWKS cache
  - turning verified claims into `AuthContext`

Suggested internal shape:

```rust
#[derive(Clone)]
pub struct AuthVerifier {
    issuer: String,
    jwks_url: String,
    client: reqwest::Client,
    cache: Arc<Mutex<JwksCache>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct AuthContext {
    pub user_id: sqlx::types::Uuid,
    pub email: Option<String>,
    pub role: Option<String>,
}
```

`JwksCache` only needs:

- `fetched_at`
- map of `kid -> decoding key`

Do not persist JWKS in Redis, Postgres, or files.

### 6. Main module

Update `apps/api/src/main.rs`:

- add `mod auth;`
- extend `AppState` to include the auth verifier
- construct the verifier during startup from `SUPABASE_URL`
- keep startup fail-fast if config is invalid

Router shape after the change:

```text
/
`-- /api
    |-- /health
    |-- /version
    `-- /me
```

Protect `/api/me` with route-level middleware that:

- verifies the Bearer token
- inserts `AuthContext` into request extensions
- calls the handler on success

### 7. `/api/me` handler

Add a small authenticated handler that returns the current auth context:

```json
{
  "user_id": "00000000-0000-0000-0000-000000000000",
  "email": "user@example.com",
  "role": "authenticated"
}
```

Notes:

- `email` may be `null`
- `role` may be `null`
- the handler should not hit Postgres

This endpoint is only a proof that auth middleware and request extensions are wired correctly.

## Request Flow

Expected auth flow for `GET /api/me`:

1. read `Authorization`
2. require `Bearer <token>`
3. decode unverified JWT header to read `kid`
4. load matching key from the in-memory cache
5. if needed, refresh JWKS from Supabase and retry key lookup
6. verify signature and standard claims
7. parse `sub` into UUID
8. build `AuthContext`
9. attach `AuthContext` to request extensions
10. return `/api/me` response

## Test Plan

Add tests only for critical behavior.

### Pure/unit tests

1. missing `Authorization` header is rejected
2. non-Bearer scheme is rejected
3. invalid `sub` is rejected

### Route/integration tests

Use a local test JWKS server instead of a real Supabase project.

Test cases:

1. `GET /api/me` without auth returns `401`
2. `GET /api/me` with a valid JWT returns `200`
3. `/api/me` response includes `user_id`, `email`, and `role`
4. first valid request fetches JWKS, second valid request with same `kid` uses the cached key
5. stale cache or unknown `kid` triggers one refresh and still verifies the token

Implementation note:

- use a static test signing key pair in tests
- serve the matching public JWKS from a tiny local HTTP app
- keep tests deterministic and offline

## Experience Notes

- Keep the canonical server input as `SUPABASE_URL`. Do not add parallel env names like `SUPABASE_JWKS_URL`, `SUPABASE_ISSUER`, or `SUPABASE_PROJECT_REF` unless the repo later proves a real need for them.
- JWT verification for Supabase user tokens does not need the `service_role` key. Use the project's public JWKS and keep signing secrets out of the API auth path.
- Treat `sub` as the only required identity field for MVP authorization context. `email` and `role` are useful metadata, but they should stay optional and non-authoritative.
- Keep `AuthContext` intentionally small. Do not mirror the full JWT payload into Rust structs or extensions, or the API will start growing a second auth/domain schema.
- Attach auth context to request extensions and stop there. Avoid building a custom extractor framework, policy engine, or middleware stack registry for a single protected route.
- Prefer route-level protection for now. Applying auth globally to `/api` would immediately create exception handling for `/health` and `/version` and would add complexity before the API has enough protected routes to justify it.
- Cache JWKS briefly, but always recover cleanly on an unknown `kid`. Supabase key rotation is the real reason to have the refresh path, not just raw performance.
- Return generic `401` responses. Detailed token parsing and verification errors are helpful in logs, but they should not leak through the public JSON API surface.
- `/api/me` should not touch Postgres. If identity verification already depends on JWT validation, adding a DB read to prove the user exists would only add latency and new failure modes.
- Tests should not depend on a real Supabase project. A local JWKS fixture gives deterministic coverage for signature verification, cache refresh, and invalid-token paths without introducing network flakiness.

## Verification Commands

When this plan is implemented, run:

```bash
cargo fmt --all
cargo test -p gloq-api
```

Manual smoke:

```powershell
$env:DATABASE_URL = "postgres://postgres:postgres@127.0.0.1:54322/postgres"
$env:SUPABASE_URL = "https://<project-ref>.supabase.co"
cargo run -p gloq-api
```

Then verify:

- `GET http://127.0.0.1:4000/api/health`
- `GET http://127.0.0.1:4000/api/version`
- `GET http://127.0.0.1:4000/api/me` without a token returns `401`
- `GET http://127.0.0.1:4000/api/me` with a valid Supabase access token returns the current user payload

## Done Criteria

This task is complete when all of the following are true:

1. `apps/api` verifies Supabase Bearer tokens using the project's JWKS endpoint
2. JWKS is cached in memory for a short period and refreshed when needed
3. the API extracts `user_id` from `sub`
4. the API exposes `email` and `role` when present
5. auth context is attached to request extensions
6. `GET /api/me` returns the authenticated user info
7. the implementation stays dependency-light and does not introduce a full auth framework
