create table "user" (
   id uuid primary key,
   created_at timestamptz not null default now(),
   updated_at timestamptz not null default now()
);

