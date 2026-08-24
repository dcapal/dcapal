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

INSERT INTO assets_data (id, provider, symbol, name, currency, asset_class)
VALUES
    ('00000000-0000-7000-8000-000000000001', 2, 'VWCE', 'Vanguard FTSE All-World', 'EUR', 1),
    ('00000000-0000-7000-8000-000000000002', 2, 'CASH', 'Cash', 'EUR', 3);

INSERT INTO portfolio_asset (
    id, portfolio_id, assets_data_id, asset_class_override,
    quantity, target_weight, manual_price, max_fee_impact, fee_type, fee_amount,
    fee_rate, min_fee, max_fee, average_buy_price, created_at, updated_at
)
SELECT
    values.id,
    '10000000-0000-0000-0000-000000000001',
    assets_data.id,
    NULL,
    values.quantity,
    values.target_weight,
    values.manual_price,
    values.max_fee_impact,
    values.fee_type,
    values.fee_amount,
    values.fee_rate,
    values.min_fee,
    values.max_fee,
    values.average_buy_price,
    values.created_at,
    values.updated_at
FROM (
    VALUES
        ('00000000-0000-7000-8000-000000000011'::uuid, 'VWCE'::text, 10::numeric, 1::numeric, 100::numeric, NULL::numeric, NULL::text, NULL::numeric, NULL::numeric, NULL::numeric, NULL::numeric, 90::numeric, '2025-01-01T00:00:00Z'::timestamptz, '2025-01-01T00:00:00Z'::timestamptz),
        ('00000000-0000-7000-8000-000000000012'::uuid, 'CASH'::text, 100::numeric, 0::numeric, 1::numeric, NULL::numeric, 'ZeroFee'::text, NULL::numeric, NULL::numeric, NULL::numeric, NULL::numeric, 1::numeric, '2025-01-01T00:00:00Z'::timestamptz, '2025-01-01T00:00:00Z'::timestamptz)
) AS values(id, symbol, quantity, target_weight, manual_price, max_fee_impact, fee_type, fee_amount, fee_rate, min_fee, max_fee, average_buy_price, created_at, updated_at)
JOIN assets_data ON assets_data.symbol = values.symbol AND assets_data.provider = 2;
