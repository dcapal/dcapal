-- USERS
create table public.users (
  id          uuid references auth.users not null primary key, -- UUID from auth.users
  username    text,
  email text not null unique
);
comment on table public.users is 'Profile data for each user.';
comment on column public.users.id is 'References the internal Supabase Auth user.';
