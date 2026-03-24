# 004 Supabase Project Persistence

## Goal

Create Supabase SQL migrations for:

- `projects`
- `project_members`
- `project_snapshots`
- private storage bucket `project-assets`

The editor document must stay as JSONB snapshots so the TypeScript frontend domain remains canonical. Rust should persist opaque snapshots plus small relational metadata only, not a second BIM schema.

## Current Repo State

`supabase/migrations/20260324170000_init.sql` currently creates:

- `public.projects` with `owner_id`
- `public.project_versions`

That is close, but it does not match the current MVP direction:

- project access should be membership-based, not owner-column based
- snapshot naming should align with the API surface
- the storage bucket is still missing

## Scope

In scope:

- thin project metadata
- project membership join table
- append-only JSONB snapshots
- private storage bucket bootstrap
- only the indexes and constraints needed for:
  - load by project
  - list projects by member
  - fetch latest snapshot quickly

Out of scope:

- decomposed BIM/domain tables
- role matrix or per-action permissions
- storage object policies
- optimistic locking
- soft delete
- triggers and background jobs
- denormalized `latest_snapshot_id`

## Schema Decisions

### 1. `projects` stays thin

Use `public.projects` for project metadata only:

- `id uuid primary key default gen_random_uuid()`
- `name text not null`
- `created_by uuid not null references auth.users (id) on delete restrict`
- `created_at timestamptz not null default timezone('utc', now())`
- `updated_at timestamptz not null default timezone('utc', now())`

Do not add floors, spaces, levels, geometry, materials, or any other BIM-facing columns here.

### 2. `project_members` is presence-only

Use `public.project_members` as the only collaboration table:

- `project_id uuid not null references public.projects (id) on delete cascade`
- `user_id uuid not null references auth.users (id) on delete cascade`
- `created_at timestamptz not null default timezone('utc', now())`
- `primary key (project_id, user_id)`

Do not add a role enum, ACL flags, invitation state, or per-member metadata yet.

Owner semantics stay minimal for now:

- `projects.created_by` identifies the creator
- the creator is also inserted into `project_members` on project creation

This keeps listing and access joins simple without over-modeling permissions.

### 3. `project_snapshots` stores opaque editor state

Use `public.project_snapshots` for append-only document snapshots:

- `id uuid primary key default gen_random_uuid()`
- `project_id uuid not null references public.projects (id) on delete cascade`
- `created_by uuid not null references auth.users (id) on delete restrict`
- `snapshot jsonb not null`
- `created_at timestamptz not null default timezone('utc', now())`

Do not add:

- `version_number`
- decomposed geometry tables
- server-owned schema fields that mirror the frontend document

The API saves the full frontend document as JSONB and treats it as opaque application data.

## Minimum Index Plan

Only add these indexes and constraints:

### `projects`

- primary key on `id`

No extra secondary index is needed yet. Loading a project by ID uses the primary key.

### `project_members`

- primary key `(project_id, user_id)`
- secondary index `project_members_user_id_project_id_idx` on `(user_id, project_id)`

Why:

- load members for one project uses the primary key prefix on `project_id`
- list projects by member uses the reverse lookup index on `user_id`

### `project_snapshots`

- primary key on `id`
- secondary index `project_snapshots_project_id_created_at_id_idx` on `(project_id, created_at desc, id desc)`

Why:

- all snapshots for a project can be loaded by `project_id`
- latest snapshot lookup can use:

```sql
select *
from public.project_snapshots
where project_id = $1
order by created_at desc, id desc
limit 1;
```

Do not add `latest_snapshot_id` to `projects` yet. The composite snapshot index is enough for the MVP and avoids duplicate write paths.

## Storage Bucket Plan

Bootstrap a private Supabase storage bucket:

```sql
insert into storage.buckets (id, name, public)
values ('project-assets', 'project-assets', false)
on conflict (id) do nothing;
```

Do not add storage object policies yet. The bucket only needs to exist and stay private for now.

## Permission Strategy

Do not build a real permission model in this task.

Recommended minimum:

- enable RLS on `projects`, `project_members`, and `project_snapshots`
- do not add end-user table policies yet
- keep direct client access closed by default
- let the Rust API own auth-aware persistence later

This is intentionally thinner than the current owner-only policies in `init.sql`.

## Migration Strategy

Because the repo is still greenfield and only has a single initial migration, the smallest sensible change is:

1. Edit `supabase/migrations/20260324170000_init.sql` in place
2. Replace `owner_id` plus `project_versions` with:
   - `projects`
   - `project_members`
   - `project_snapshots`
3. Add the private `project-assets` bucket bootstrap
4. Remove the owner-specific policies from the current migration

Do not add a second corrective migration unless this initial migration has already been applied to a shared environment.

## SQL Shape To Implement

```sql
create extension if not exists pgcrypto;

create table public.projects (
    id uuid primary key default gen_random_uuid(),
    name text not null,
    created_by uuid not null references auth.users (id) on delete restrict,
    created_at timestamptz not null default timezone('utc', now()),
    updated_at timestamptz not null default timezone('utc', now())
);

create table public.project_members (
    project_id uuid not null references public.projects (id) on delete cascade,
    user_id uuid not null references auth.users (id) on delete cascade,
    created_at timestamptz not null default timezone('utc', now()),
    primary key (project_id, user_id)
);

create table public.project_snapshots (
    id uuid primary key default gen_random_uuid(),
    project_id uuid not null references public.projects (id) on delete cascade,
    created_by uuid not null references auth.users (id) on delete restrict,
    snapshot jsonb not null,
    created_at timestamptz not null default timezone('utc', now())
);

create index project_members_user_id_project_id_idx
    on public.project_members (user_id, project_id);

create index project_snapshots_project_id_created_at_id_idx
    on public.project_snapshots (project_id, created_at desc, id desc);

alter table public.projects enable row level security;
alter table public.project_members enable row level security;
alter table public.project_snapshots enable row level security;

insert into storage.buckets (id, name, public)
values ('project-assets', 'project-assets', false)
on conflict (id) do nothing;
```

## API Notes

When the Rust API starts using this schema:

- project creation should insert into `projects` and `project_members` in one transaction
- snapshot creation should insert into `project_snapshots`
- the same transaction should update `projects.updated_at`

The snapshot payload remains the canonical frontend document blob. Rust should not attempt to normalize or mirror the editor model.

## Verification Plan

When this plan is implemented:

1. run `supabase db reset`
2. confirm tables exist:
   - `public.projects`
   - `public.project_members`
   - `public.project_snapshots`
3. confirm bucket exists and is private:
   - `project-assets`
4. confirm indexes exist:
   - `project_members_user_id_project_id_idx`
   - `project_snapshots_project_id_created_at_id_idx`
5. smoke test representative queries:
   - load project by ID
   - list projects by member
   - fetch latest snapshot by project

## Done Criteria

This task is ready to implement when the migration does all of the following:

1. stores project documents only as JSONB snapshots
2. uses membership rows instead of an `owner_id` access model
3. adds no extra BIM/domain schema in Postgres
4. creates only the minimum indexes needed for the required query paths
5. creates a private `project-assets` storage bucket
6. leaves permission modeling intentionally thin for now
