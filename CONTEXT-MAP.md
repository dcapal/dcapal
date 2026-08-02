# Context Map

## Contexts

- [DcaPal Frontend](./dcapal-frontend/CONTEXT.md) — lets an investor describe portfolios and generate allocation recommendations.
- [DcaPal Backend](./dcapal-backend/CONTEXT.md) — provides market data, imported portfolio data, and remote storage for saved portfolios.
- [DcaPal Optimizer](./dcapal-optimizer-wasm/CONTEXT.md) — calculates allocation recommendations from portfolio state, target weights, budget, unit rules, and fee policies.

## Relationships

- **Frontend → Backend**: requests asset data, prices, and imported portfolio data; authenticated users can synchronize saved portfolios.
- **Frontend → Optimizer**: sends the current portfolio, target weights, investment budget, fee policy, buy-only mode, budget-use preference, and unit rules; receives an allocation recommendation, any fee-rejected allocation, and unallocated cash.

## Shared language

The contexts share these concepts: `Portfolio`, `Portfolio asset`, `quote currency`, `target weight`, `investment budget`, `transaction fee policy`, and `allocation`.
