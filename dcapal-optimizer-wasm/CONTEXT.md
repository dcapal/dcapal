# DcaPal Optimizer

DcaPal Optimizer calculates allocation recommendations from a portfolio's current holdings, target weights, investment budget, unit rules, and transaction fee policies. It can recommend buying and, when buy-only mode is off, selling existing holdings; it never executes trades or owns portfolio data.

## Allocation language

**Projected portfolio value**:
The current portfolio value plus the investment budget used as the basis for target weights in an allocation recommendation. Unallocated cash remains part of the projected value.
_Avoid_: Portfolio amount, post-trade balance

**Allocation feasibility**:
The ability to produce a recommendation within the active hard rules while moving holdings towards their target weights. A recommendation can leave cash unallocated, remain slightly away from target weights because of unit prices, unit rules, or fee limits, or exceed a target weight when the investor chooses to use all the budget.
_Avoid_: Exact rebalancing, guaranteed full allocation

**Suggested investment amount**:
An estimate of new money that would reduce the portfolio's most-weighted asset above its target weight to that target weight, based on current holdings and target weights. It is zero when no asset is above target, and otherwise is a planning suggestion, not a required contribution or an executed trade.
_Avoid_: Required investment, guaranteed minimum

## Fee language

**Fee impact**:
The estimated transaction fee divided by the amount allocated to an asset. The optimizer compares it with the maximum fee impact in the applicable transaction fee policy before accepting an allocation.
_Avoid_: Fee rate, fee amount

**Fee-rejected allocation**:
A candidate allocation that is excluded because its fee impact exceeds the accepted maximum. It shows the candidate holdings and estimated fee so the investor can understand the trade-off.
_Avoid_: Theoretical allocation, rejected transaction
