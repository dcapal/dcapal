-- ASSETS
create table public.assets
(
    id          uuid primary key         default gen_random_uuid(),
    symbol      text                                   not null unique,
    name        text                                   not null,
    asset_class bigint                                   not null,
    currency    text                                   not null,
    created_at  timestamp with time zone default now() not null,
    updated_at  timestamp with time zone default now() not null,
    unique (symbol)
);

-- ASSET PRICES
create table public.asset_prices
(
    id            uuid primary key         default gen_random_uuid(),
    instrument_id uuid references public.assets          not null,
    eod_price     numeric(20, 10)                        not null,
    date          date                                   not null,
    created_at    timestamp with time zone default now() not null,
    updated_at    timestamp with time zone default now() not null
);

-- PORTFOLIOS
create table public.portfolios
(
    id          uuid primary key         default gen_random_uuid(),
    user_id     uuid references public.users           not null,
    name        text                                   not null,
    description text,
    currency    text                                   not null,
    created_at  timestamp with time zone default now() not null,
    updated_at  timestamp with time zone default now() not null
);

-- HOLDINGS
create table public.portfolio_holdings
(
    id                uuid primary key         default gen_random_uuid(),
    portfolio_id      uuid references public.portfolios      not null,
    instrument_id     uuid references public.assets          not null,
    quantity          numeric(20, 10)                        not null,
    weight            numeric(20, 10)                        not null,
    total             numeric(20, 10)                        not null,
    price             numeric(20, 10)                        not null,
    average_buy_price numeric(20, 10)                        not null,
    created_at        timestamp with time zone default now() not null,
    updated_at        timestamp with time zone default now() not null
);
