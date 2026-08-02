CREATE TABLE IF NOT EXISTS portfolio_asset (
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
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_portfolio_asset_portfolio_id
        FOREIGN KEY (portfolio_id) REFERENCES portfolios (id)
);
