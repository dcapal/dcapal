CREATE TABLE IF NOT EXISTS portfolios (
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
