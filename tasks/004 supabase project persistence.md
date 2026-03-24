# 004 Supabase Project Persistence

## Status

Implemented on `2026-03-24`.

Verification status:

- `supabase db reset` could not run in this environment because Docker Desktop was not available on Windows

## Goal

Create the Supabase SQL migration plan for:

- `projects`
- `project_members`
- `project_snapshots`
- private storage bucket `project-assets`

The editor document stays in JSONB snapshots so TypeScript remains the canonical domain layer. Rust persists project metadata plus snapshots only. No second BIM schema is introduced in Postgres or Rust.

## Original Starting Point

`supabase/migrations/20260324170000_init.sql` currently creates:

- `projects` with `owner_id`
- `project_versions` with `version_number`

That migration is close, but it does not match the requested shape:

- access/listing needs `project_members`
- snapshot naming should be `project_snapshots`
- a private storage bucket is still missing

## Current Implementation Context

`supabase/migrations/20260324170000_init.sql` now creates:

- `public.projects`
- `public.project_members`
- `public.project_snapshots`
- private storage bucket `project-assets`

Actual shipped SQL shape:

- `projects`
  - `id uuid primary key default gen_random_uuid()`
  - `name text not null`
  - `created_at timestamptz not null default now()`
  - `updated_at timestamptz not null default now()`
- `project_members`
  - `project_id`
  - `user_id`
  - `created_at`
  - `primary key (project_id, user_id)`
- `project_snapshots`
  - `project_id`
  - `version_number`
  - `snapshot jsonb`
  - `created_at`
  - `primary key (project_id, version_number)`

Only one secondary index was added:

- `project_members_user_id_project_id_idx`

No RLS policies, storage object policies, triggers, or extra audit columns were added in this task.

## Scope

In scope:

- thin project metadata
- member-to-project join rows
- append-only JSONB snapshots
- only the minimum indexes and constraints needed for:
  - load by project
  - list projects by member
  - fetch latest snapshot quickly
- private bucket bootstrap

Out of scope:

- BIM/domain tables
- roles or ACL matrices
- storage object policies
- RLS policy design
- triggers
- SQL helper functions
- denormalized `latest_snapshot_id`
- extra audit columns not required by the task

## Core Decisions

### 1. Keep `projects` minimal

Use `public.projects` for project metadata only:

- `id uuid primary key default gen_random_uuid()`
- `name text not null`
- `created_at timestamptz not null default now()`
- `updated_at timestamptz not null default now()`

Do not add:

- `owner_id`
- `created_by`
- geometry columns
- project settings that belong in the editor document

Reason:

- ownership is not part of this task
- membership already gives the list-by-user path
- extra ownership/audit fields would be unused schema today

### 2. Make `project_members` the only relational access list

Use `public.project_members` as a pure join table:

- `project_id uuid not null references public.projects (id) on delete cascade`
- `user_id uuid not null references auth.users (id) on delete cascade`
- `created_at timestamptz not null default now()`
- `primary key (project_id, user_id)`

Do not add:

- `role`
- invitation state
- permissions flags
- surrogate member ID

Reason:

- the task only needs project membership for filtering and joins
- the composite primary key is both the data model and the needed uniqueness rule

### 3. Make snapshots naturally versioned

Use `public.project_snapshots` with a project-local version key:

- `project_id uuid not null references public.projects (id) on delete cascade`
- `version_number integer not null`
- `snapshot jsonb not null`
- `created_at timestamptz not null default now()`
- `primary key (project_id, version_number)`

Do not add:

- snapshot UUID
- `created_by`
- redundant latest-snapshot pointer
- decomposed snapshot tables

Reason:

- the task explicitly needs versioned snapshots
- `(project_id, version_number)` is the natural key for this table
- this avoids a redundant surrogate ID and avoids a second unique index
- latest-snapshot queries become deterministic without relying on timestamp ties or UUID ordering

## Minimum Index And Constraint Plan

### `projects`

Required:

- primary key on `id`

Nothing else is needed for the stated query paths.

### `project_members`

Required:

- primary key `(project_id, user_id)`
- secondary index `project_members_user_id_project_id_idx` on `(user_id, project_id)`

Why:

- load a project's members by `project_id` uses the primary key prefix
- list projects by member uses the reverse lookup index on `user_id`

### `project_snapshots`

Required:

- primary key `(project_id, version_number)`

Why:

- load all snapshots for a project uses the primary key prefix on `project_id`
- fetch latest snapshot uses the same primary key with a backward index scan:

```sql
select version_number, snapshot, created_at
from public.project_snapshots
where project_id = $1
order by version_number desc
limit 1;
```

No extra snapshot index is needed.

## Storage Bucket Plan

Bootstrap a private bucket only:

```sql
insert into storage.buckets (id, name, public)
values ('project-assets', 'project-assets', false)
on conflict (id) do nothing;
```

Do not add storage object policies in this task.

## Migration Strategy

Use a conditional rollout rule instead of assuming repo state:

1. If `supabase/migrations/20260324170000_init.sql` has not been shared or applied outside local development, replace its `owner_id` and `project_versions` shape in place.
2. If that migration has already been applied in any shared environment, add a new forward-only migration instead of rewriting history.

This keeps the plan factual and avoids guessing about deployment state.

## SQL Shape To Implement

```sql
create extension if not exists pgcrypto;

create table public.projects (
    id uuid primary key default gen_random_uuid(),
    name text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table public.project_members (
    project_id uuid not null references public.projects (id) on delete cascade,
    user_id uuid not null references auth.users (id) on delete cascade,
    created_at timestamptz not null default now(),
    primary key (project_id, user_id)
);

create table public.project_snapshots (
    project_id uuid not null references public.projects (id) on delete cascade,
    version_number integer not null,
    snapshot jsonb not null,
    created_at timestamptz not null default now(),
    primary key (project_id, version_number)
);

create index project_members_user_id_project_id_idx
    on public.project_members (user_id, project_id);

insert into storage.buckets (id, name, public)
values ('project-assets', 'project-assets', false)
on conflict (id) do nothing;
```

## API Notes

When the Rust API uses this schema:

- create project in one transaction:
  - insert `projects`
  - insert creator row into `project_members`
- save snapshot in one transaction:
  - compute the next `version_number`
  - insert into `project_snapshots`
  - update `projects.updated_at = now()`

Do not move editor geometry, level, or space logic into Rust or SQL. The snapshot blob remains the frontend-owned document.

## Verification Plan

When the migration is implemented:

1. run `supabase db reset`
2. confirm tables exist:
   - `public.projects`
   - `public.project_members`
   - `public.project_snapshots`
3. confirm constraints/indexes exist:
   - `projects_pkey`
   - `project_members_pkey`
   - `project_members_user_id_project_id_idx`
   - `project_snapshots_pkey`
4. confirm bucket exists and is private:
   - `project-assets`
5. smoke test the three target query paths:
   - load project by ID
   - list projects by member
   - fetch latest snapshot by project using `order by version_number desc limit 1`

## Implementation Notes

- The migration was implemented by editing `supabase/migrations/20260324170000_init.sql` directly.
- `owner_id`, `project_versions`, owner-only indexes, and owner-only RLS policies were removed.
- `project_snapshots` uses the natural key `(project_id, version_number)` instead of a surrogate snapshot ID.
- `created_at` and `updated_at` use `default now()` instead of `timezone('utc', now())`.

## Kinh Nghiem / Ghi Chu Van Hanh

- `project_snapshots` should use deterministic version ordering, not `created_at` plus random UUID ordering. `version_number` is the smallest schema that gives a correct latest-snapshot query.
- `default now()` is the minimal correct default for `timestamptz` columns here. The earlier `timezone('utc', now())` expression was unnecessary.
- For the required query paths, the schema only needs:
  - `projects_pkey`
  - `project_members_pkey`
  - `project_members_user_id_project_id_idx`
  - `project_snapshots_pkey`
- A separate snapshot UUID or `latest_snapshot_id` would add write complexity and index overhead without helping the current MVP.
- On Windows, local `supabase db reset` depends on Docker Desktop being installed and running. Without it, migration verification stops at static SQL review.

## Done Criteria

This plan is ready to implement when the migration:

1. stores editor state only as JSONB snapshots
2. uses `project_members` for member-to-project lookup
3. keeps `projects` and `project_snapshots` free of unnecessary ownership and audit columns
4. uses deterministic snapshot version ordering without extra snapshot indexes
5. creates the private `project-assets` bucket
6. leaves permission modeling for a later auth task
