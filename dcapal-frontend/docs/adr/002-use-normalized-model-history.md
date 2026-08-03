# ADR 002: Use normalized model history for historical performance

## Status

Accepted

## Context

DcaPal's Portfolio stores assets, target weights, and current holdings, but it does not need to become a transaction ledger just to show how a model allocation moved over time. A nominal currency chart without contribution timing would imply a level of performance precision the product does not have.

## Decision

Represent historical performance as a normalized model series calculated from target weights and historical asset prices, rebased to a common starting value. The backend provides asset-level time series; the frontend combines them using the Portfolio's saved absolute or derived weights. Show current Portfolio value separately, and do not label the model series as a money-weighted return.

## Consequences

The historical view can explain model movement without adding transaction records or cash-flow assumptions. A Portfolio clone never copies a history; the frontend recomputes the selected Portfolio's series from backend asset time series. The UI must clearly distinguish model performance from current value.
