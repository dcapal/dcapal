# ADR 001: Keep strategic allocation separate from Portfolio

## Status

Accepted

## Context

DcaPal's Portfolio currently represents the asset-level collection: its assets, target weights, holdings, quote currency, and fee policies. The new product also needs strategic allocation guidance and other auxiliary planning decisions, but embedding those concerns inside Portfolio would make the existing allocation model harder to evolve.

## Decision

Keep Portfolio as the asset-level collection. Model strategic allocation, allocation-framework suggestions, and other auxiliary decisions as separate domain entities that may reference a Portfolio or its asset classes. Use a direct Asset Class → Asset hierarchy and do not introduce a tactical-sleeve entity.

Offer two targeting modes. Simple allocation mode lets an investor enter absolute target weights for each asset and derives the Asset Class totals. Strategic allocation mode lets an investor enter Asset Class target weights and relative asset weights within each class; DcaPal derives each asset's absolute Portfolio weight from those two values.

## Consequences

The current allocator can remain simple and compatible with existing portfolios. Strategic guidance can evolve independently and multiple frameworks can coexist. The UI must distinguish editing a Portfolio from reviewing or applying strategic guidance, make the two targeting modes explicit, and show derived absolute weights when the advanced mode is active.
