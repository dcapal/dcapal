# ADR 004: Support simple and strategic targeting modes

## Status

Accepted

## Context

DcaPal's current editor asks investors for absolute target weights for each asset. The new strategic guidance feature also needs to express an Asset Class target and, in the advanced editor, each asset's share within that class, without replacing the simple workflow or introducing arbitrary nested allocation levels.

## Decision

Support two explicit modes. Simple allocation mode lets the investor define absolute asset weights directly and derives Asset Class totals. Strategic allocation mode lets the investor define Asset Class target weights and relative asset weights within each class; DcaPal derives each asset's absolute Portfolio weight by multiplying the two values. Keep the Asset Class → Asset hierarchy in both modes, and let Strategic allocation guidance compare class targets with aggregated current weights in either mode.

## Consequences

Existing portfolios can continue using the simple editor. The advanced editor must label absolute and relative weights clearly, validate that class weights and within-class weights each total 100%, and show the derived absolute weights used by allocation and rebalancing. Guidance must remain useful in Simple mode, where it compares class targets with current class totals without requiring relative asset weights.
