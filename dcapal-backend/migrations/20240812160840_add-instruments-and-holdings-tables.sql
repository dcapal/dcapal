-- PORTFOLIOS
create table public.portfolios
(
    id             uuid primary key         default gen_random_uuid(),
    user_id        uuid references public.users           not null,
    name           text                                   not null,
    description    text,
    currency       text                                   not null,
    deleted        boolean                                not null default false,
    last_updated_at   timestamp with time zone default now() not null,
    max_fee_impact numeric(20, 10)                        null,
    fee_type       text                                   not null,
    fee_amount     numeric(20, 10)                        null,
    fee_rate       numeric(20, 10)                        null,
    min_fee        numeric(20, 10)                        null,
    max_fee        numeric(20, 10)                        null,
    created_at     timestamp with time zone default now() not null,
    updated_at     timestamp with time zone default now() not null
);

-- PORTFOLIO ASSETS
create table public.portfolio_assets
(
    id             uuid primary key         default gen_random_uuid(),
    portfolio_id   uuid references public.portfolios      not null,
    asset_id       uuid references public.assets          not null,
    quantity       numeric(20, 10)                        not null,
    target_weight  numeric(20, 10)                        not null,
    price          numeric(20, 10)                        not null,
    max_fee_impact numeric(20, 10)                        null,
    fee_type       text                                   not null,
    fee_amount     numeric(20, 10)                        null,
    fee_rate       numeric(20, 10)                        null,
    min_fee        numeric(20, 10)                        null,
    max_fee        numeric(20, 10)                        null,
    created_at     timestamp with time zone default now() not null,
    updated_at     timestamp with time zone default now() not null
);

-- ASSETS
create table public.assets
(
    id          uuid primary key         default gen_random_uuid(),
    symbol      text                                   not null unique,
    name        text                                   not null,
    asset_class text                                   not null,
    currency    text                                   not null,
    provider    text                                   not null,
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
