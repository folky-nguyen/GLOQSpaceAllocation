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

create index project_members_user_id_project_id_idx on public.project_members (user_id, project_id);

insert into storage.buckets (id, name, public)
values ('project-assets', 'project-assets', false)
on conflict (id) do nothing;
