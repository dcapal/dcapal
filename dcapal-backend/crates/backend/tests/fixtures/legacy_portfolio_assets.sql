INSERT INTO users (id, username, email, role)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'legacy-user',
    'legacy-user@example.com',
    'User'
);

INSERT INTO portfolios (
    id, user_id, name, currency, deleted, last_updated_at,
    max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee,
    created_at, updated_at
)
VALUES (
    '10000000-0000-0000-0000-000000000001',
    '00000000-0000-0000-0000-000000000001',
    'Legacy portfolio',
    'EUR',
    FALSE,
    '2025-01-01T00:00:00Z',
    NULL,
    NULL,
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
        '20000000-0000-0000-0000-000000000001', 'foo',
        '10000000-0000-0000-0000-000000000001', 'Foo first', 'equity', 'EUR', 'dCaPaL',
        1, 0.1, 10, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '2025-01-02T00:00:00Z', '2025-01-02T00:00:00Z'
    ),
    (
        '20000000-0000-0000-0000-000000000002', 'FOO',
        '10000000-0000-0000-0000-000000000001', 'Foo oldest', 'BOND', 'EUR', 'Kraken',
        2, 0.2, 20, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '2025-01-01T00:00:00Z', '2025-01-01T00:00:00Z'
    ),
    (
        '20000000-0000-0000-0000-000000000003', 'eur',
        '10000000-0000-0000-0000-000000000001', 'Euro', 'CURRENCY', 'EUR', 'Yahoo',
        3, 0.3, 30, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '2025-01-03T00:00:00Z', '2025-01-03T00:00:00Z'
    ),
    (
        '20000000-0000-0000-0000-000000000004', 'cash',
        '10000000-0000-0000-0000-000000000001', 'Cash', 'CASH', 'EUR', 'YF',
        4, 0.4, 40, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '2025-01-04T00:00:00Z', '2025-01-04T00:00:00Z'
    ),
    (
        '20000000-0000-0000-0000-000000000005', 'btc',
        '10000000-0000-0000-0000-000000000001', 'Bitcoin', 'CRYPTO', 'EUR', 'YF',
        5, 0.5, 50, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '2025-01-05T00:00:00Z', '2025-01-05T00:00:00Z'
    ),
    (
        '20000000-0000-0000-0000-000000000006', 'gold',
        '10000000-0000-0000-0000-000000000001', 'Gold', 'COMMODITY', 'EUR', 'Yahoo',
        6, 0.6, 60, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '2025-01-06T00:00:00Z', '2025-01-06T00:00:00Z'
    ),
    (
        '20000000-0000-0000-0000-000000000007', 'other',
        '10000000-0000-0000-0000-000000000001', 'Other', 'Unrecognised', 'EUR', 'Kraken',
        7, 0.7, 70, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '2025-01-07T00:00:00Z', '2025-01-07T00:00:00Z'
    ),
    (
        '20000000-0000-0000-0000-000000000008', 'deleted',
        '10000000-0000-0000-0000-000000000001', 'Deleted', 'EQUITY', 'EUR', 'IBKR',
        8, 0.8, 80, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '2025-01-08T00:00:00Z', '2025-01-08T00:00:00Z'
    ),
    (
        '20000000-0000-0000-0000-000000000009', 'tie',
        '10000000-0000-0000-0000-000000000001', 'Tie lower id', 'BOND', 'EUR', 'Kraken',
        9, 0.9, 90, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '2025-01-09T00:00:00Z', '2025-01-09T00:00:00Z'
    ),
    (
        '20000000-0000-0000-0000-000000000010', 'TIE',
        '10000000-0000-0000-0000-000000000001', 'Tie higher id', 'BOND', 'EUR', 'Kraken',
        10, 1.0, 100, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '2025-01-09T00:00:00Z', '2025-01-09T00:00:00Z'
    ),
    (
        '20000000-0000-0000-0000-000000000011', 'equity',
        '10000000-0000-0000-0000-000000000001', 'Equity', 'EQUITY', 'EUR', 'YF',
        11, 1.1, 110, NULL, NULL, NULL, NULL, NULL, NULL, NULL, '2025-01-10T00:00:00Z', '2025-01-10T00:00:00Z'
    );
