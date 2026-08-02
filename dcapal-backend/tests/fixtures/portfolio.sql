INSERT INTO portfolios (
    id, user_id, name, currency, deleted, last_updated_at,
    max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee,
    created_at, updated_at
)
VALUES (
    '10000000-0000-0000-0000-000000000001',
    '00000000-0000-0000-0000-000000000001',
    'Existing portfolio',
    'EUR',
    FALSE,
    '2025-01-01T00:00:00Z',
    NULL,
    'ZeroFee',
    NULL,
    NULL,
    NULL,
    NULL,
    '2025-01-01T00:00:00Z',
    '2025-01-01T00:00:00Z'
);

INSERT INTO portfolio_asset (
    id, symbol, portfolio_id, name, asset_class, currency, provider,
    quantity, target_weight, price, max_fee_impact, fee_type, fee_amount,
    fee_rate, min_fee, max_fee, average_buy_price, created_at, updated_at
)
VALUES
    (
        '20000000-0000-0000-0000-000000000001',
        'VWCE',
        '10000000-0000-0000-0000-000000000001',
        'Vanguard FTSE All-World',
        'Stock',
        'EUR',
        'IBKR',
        10,
        1,
        100,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        NULL,
        90,
        '2025-01-01T00:00:00Z',
        '2025-01-01T00:00:00Z'
    ),
    (
        '20000000-0000-0000-0000-000000000002',
        'CASH',
        '10000000-0000-0000-0000-000000000001',
        'Cash',
        'Cash',
        'EUR',
        'IBKR',
        100,
        0,
        1,
        NULL,
        'ZeroFee',
        NULL,
        NULL,
        NULL,
        NULL,
        1,
        '2025-01-01T00:00:00Z',
        '2025-01-01T00:00:00Z'
    );
