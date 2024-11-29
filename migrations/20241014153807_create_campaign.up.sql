create table campaign (
   id uuid primary key,
   name varchar(200) not null,
   user_id uuid not null references "user"(id),
   created_at timestamptz not null default now(),
   updated_at timestamptz not null default now()
);
