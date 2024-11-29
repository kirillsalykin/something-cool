create extension if not exists citext;

create table lead (
   email citext not null,
   user_id uuid not null references "user"(id),
   created_at timestamptz not null default now(),
   updated_at timestamptz not null default now(),
   primary key (user_id, email)
);

