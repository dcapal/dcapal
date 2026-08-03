# ADR 003: Require registration for historical views

## Status

Accepted

## Context

Historical model analysis is more expensive to serve than the current allocation workflow. DcaPal should remain usable without an account, but anonymous users should not trigger the heavier historical-data path.

## Decision

Require registration before an investor can open historical Portfolio views. Anonymous users can still create and inspect Portfolios, allocate new money, and prepare rebalancing actions; the historical view presents registration as its clear next step.

## Consequences

Registration becomes a product value moment rather than a prerequisite for basic use. The dashboard must make the boundary clear and provide a useful CTA without hiding the rest of the Portfolio experience.
