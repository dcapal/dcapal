CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE users (
    id UUID NOT NULL PRIMARY KEY,
    username VARCHAR,
    email VARCHAR NOT NULL UNIQUE,
    role VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE seaql_migrations (
    version BIGINT NOT NULL PRIMARY KEY,
    applied_at BIGINT NOT NULL,
    migration_type VARCHAR NOT NULL
);

INSERT INTO seaql_migrations (version, applied_at, migration_type)
VALUES
    (20250131084915, 1738313355, 'up'),
    (20250201132150, 1738416110, 'up'),
    (20250201132246, 1738416166, 'up'),
    (20250207000000, 1738886400, 'up');

CREATE TABLE portfolios (
    id UUID NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR NOT NULL,
    currency VARCHAR NOT NULL,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    last_updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    max_fee_impact NUMERIC(20, 10),
    fee_type VARCHAR,
    fee_amount NUMERIC(20, 10),
    fee_rate NUMERIC(20, 10),
    min_fee NUMERIC(20, 10),
    max_fee NUMERIC(20, 10),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_portfolios_user_id FOREIGN KEY (user_id) REFERENCES users (id)
);

CREATE TABLE portfolio_asset (
    id UUID NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol TEXT NOT NULL,
    portfolio_id UUID NOT NULL,
    name TEXT NOT NULL,
    asset_class TEXT NOT NULL,
    currency TEXT NOT NULL,
    provider TEXT NOT NULL,
    quantity NUMERIC NOT NULL,
    target_weight NUMERIC NOT NULL,
    price NUMERIC NOT NULL,
    max_fee_impact NUMERIC,
    fee_type TEXT,
    fee_amount NUMERIC,
    fee_rate NUMERIC,
    min_fee NUMERIC,
    max_fee NUMERIC,
    average_buy_price NUMERIC,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_portfolio_asset_portfolio_id
        FOREIGN KEY (portfolio_id) REFERENCES portfolios (id)
);

CREATE TABLE portfolio_asset_reference (
    id UUID NOT NULL PRIMARY KEY,
    portfolio_asset_id UUID NOT NULL,
    CONSTRAINT fk_portfolio_asset_reference_asset
        FOREIGN KEY (portfolio_asset_id) REFERENCES portfolio_asset (id)
        ON UPDATE CASCADE
);
