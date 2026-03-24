create extension if not exists pgcrypto;

create table public.projects (
    id uuid primary key default gen_random_uuid(),
    owner_id uuid not null references auth.users (id) on delete cascade,
    name text not null,
    created_at timestamptz not null default timezone('utc', now()),
    updated_at timestamptz not null default timezone('utc', now())
);

create table public.project_versions (
    id uuid primary key default gen_random_uuid(),
    project_id uuid not null references public.projects (id) on delete cascade,
    version_number integer not null,
    created_by uuid not null references auth.users (id) on delete restrict,
    snapshot jsonb not null,
    created_at timestamptz not null default timezone('utc', now()),
    unique (project_id, version_number)
);

create index project_owner_idx on public.projects (owner_id);
create index project_versions_project_id_created_at_idx on public.project_versions (project_id, created_at desc);

alter table public.projects enable row level security;
alter table public.project_versions enable row level security;

create policy "owners manage their projects"
on public.projects
for all
using (auth.uid() = owner_id)
with check (auth.uid() = owner_id);

create policy "owners manage project versions"
on public.project_versions
for all
using (
    exists (
        select 1
        from public.projects
        where projects.id = project_versions.project_id
          and projects.owner_id = auth.uid()
    )
)
with check (
    exists (
        select 1
        from public.projects
        where projects.id = project_versions.project_id
          and projects.owner_id = auth.uid()
    )
);
