# ADR 006: Present Allocation and Rebalancing as one Invest journey

## Status

Accepted

## Context

The glossary keeps Allocation (a buy-only recommendation for new money) and
Rebalancing (which may sell holdings) as distinct concepts. The navigation
exposed them as two separate entries, Allocate and Rebalance, each starting
from a form. Investors had to choose a strategy before seeing how far their
Portfolio had drifted from its Target weights, and the two entries duplicated
the same budget, unit rule, and fee inputs.

The optimizer already supports both behaviours through the same contract:
buy-only mode restricts the solver to purchases, and leaving it off allows
sales. Changing that contract is not needed to change how the journey is
presented.

## Decision

Offer one Invest entry that starts from the Total drift of the Portfolio and
then lets the investor choose a path:

- Add money is an Allocation and always runs in Buy-only mode.
- Rebalance is a Rebalancing, always allows sales, and has no standalone
  buy-only toggle.

Both paths share the plan inputs, produce Proposed transactions, and end in the
same result summary. Product copy says "transactions", not "orders". The
optimizer contract is unchanged; the choice of path only sets its existing
buy-only option.

## Consequences

The sidebar merges Allocate and Rebalance into a single Invest item, so routes,
navigation tests, and Storybook stories that referenced the two entries change.
Browser smoke tests are rewritten around the drift, plan, proposal, and result
steps.

The glossary gains Invest, Total drift, and Proposed transaction. Transaction
fee policy and Maximum fee impact still apply only to buys, because the
optimizer neither rejects nor prices sales; the interface says so rather than
implying sale commissions are estimated. Investors who want a sell-free outcome
use Add money; a Rebalancing can no longer be restricted to buys from the same
screen. That loss of a toggle is accepted to make the two outcomes easy to tell
apart.

The strict glossary split between Allocation and Rebalancing is kept in the
domain model while the interface presents them as one journey, so the
presentation and the domain terms must be kept consistent in copy.
