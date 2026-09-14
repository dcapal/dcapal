# ADR 005: Use Storybook and an additive design-system boundary for new UI

## Status

Accepted

## Context

DcaPal's portfolio journey now spans an overview, asset management, portfolio
settings, allocation, and rebalancing. The visual references establish a
shared language for those journeys, but the current frontend also contains
working UI components built with Radix and Vaul. Replacing those components in
one change would increase risk and would mix a visual-system decision with
feature migration.

New UI also needs a place where its states, responsive behaviour, and
accessibility contract can be reviewed without opening a full product route.
The frontend already uses Webpack, PostCSS, Tailwind, and TypeScript for new
source, so the design-system boundary must fit that toolchain and must not
change existing screens by changing global variables.

## Decision

Use Storybook as the visual source of truth for new frontend UI. Organize its
stories as foundations, typed UI primitives, DcaPal patterns, and journey
compositions. Use deterministic data and accessibility checks to document the
states that investors can reach.

Create an additive design-system boundary under `src/design-system/` with:

- semantic, namespaced tokens scoped by `dcapal-theme`;
- typed primitives and patterns under `components/ui/` and
  `components/patterns/`;
- Base UI as the primitive foundation for all new design-system components;
- a responsive portfolio-asset collection contract shared by mobile cards and
  desktop tables.

Keep the current Radix and Vaul components in place for their existing
consumers. New design-system components do not add new dependencies on those
libraries, and existing features are migrated only as a separate,
feature-scoped change.

## Consequences

Storybook gives designers and engineers one review surface for tokens,
components, product patterns, and the main portfolio journeys. It makes
loading, validation, error, gated, and responsive states visible before a
route is changed.

The namespaced token scope prevents the new visual language from changing
legacy screens accidentally. The additive boundary means temporary coexistence
of Base UI, Radix, and Vaul, so the repository carries more than one primitive
model until migration is complete. That cost is accepted to keep the rollout
reversible and to preserve working journeys.

The design system remains presentational. Queries, mutation orchestration,
validation, and portfolio calculations stay in feature hooks or pure modules.
Patterns receive normalized presentation data and expose accessible actions;
they do not create a second domain model.
