-- INSTRUMENTS

create table public.instruments
(
    id          uuid primary key         default gen_random_uuid(),
    symbol      text                                   not null unique,
    name        text                                   not null,
    exchange    text                                   not null,
    asset_type  asset_type                             not null,
    data_source data_source                            not null,
    created_at  timestamp with time zone default now() not null,
    updated_at  timestamp with time zone default now() not null
);

-- INSTRUMENT PRICES
create table public.instrument_prices
(
    id            uuid primary key         default gen_random_uuid(),
    instrument_id uuid references public.instruments     not null,
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
create table public.holdings
(
    id            uuid primary key         default gen_random_uuid(),
    portfolio     uuid references public.portfolios      not null,
    instrument_id uuid references public.instruments     not null,
    symbol        text                                   not null,
    quantity      numeric(20, 10)                        not null,
    created_at    timestamp with time zone default now() not null,
    updated_at    timestamp with time zone default now() not null
);

-- INVESTMENTS_PREFERENCES
create table public.investment_preferences
(
    id                 uuid primary key         default gen_random_uuid(),
    user_id            uuid references public.users           not null,
    risk_tolerance     risk_tolerance                         not null,
    investment_horizon int                                    not null,
    investment_mode    investment_mode                        not null,
    investment_goal    investment_goal                        not null,
    ai_enabled         boolean                                not null,
    created_at         timestamp with time zone default now() not null,
    updated_at         timestamp with time zone default now() not null
);