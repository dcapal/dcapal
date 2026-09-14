# DcaPal design system

The design system is the forward visual and interaction boundary for new
frontend work. Storybook shows the supported foundations, UI primitives,
product patterns, and journey compositions. Use it before creating a new
piece of interface.

## Where things live

- `styles/` owns semantic design tokens and the scoped theme.
- `components/ui/` owns typed, reusable primitives backed by Base UI.
- `components/patterns/` owns reusable DcaPal patterns such as
  `PortfolioAssetCollection` and `DriftDiagnostic`.
- `stories/` documents the supported states and responsive behaviour in one
  searchable Storybook catalogue.

The design-system directory is additive. Existing application components under
`src/components/` remain valid consumers while migration happens feature by
feature.

## Design language

The current direction follows the portfolio journey references:

- one light theme with warm canvas and surface colours;
- dark navigation chrome;
- violet primary actions;
- clear success, warning, information, and destructive states;
- compact rounded cards with restrained elevation;
- equivalent mobile cards and desktop tables for portfolio assets.

Use semantic tokens from `styles/` for every visual decision. Keep colour,
spacing, radius, typography, and elevation values out of component files. The
`dcapal-theme` scope protects existing application screens from changes to the
new token layer.

Card spacing is owned by the `Card` primitive. `CardContent` is padded when it
stands alone; adjacent header, content, and footer slots remove only their
shared interior edge. Use the finite `density="default" | "compact"` option
when a board calls for a tighter card, not a one-off padding class.

Use `InputWithSuffix` for currency and percentage fields so the unit remains
inside the control. Keep trigger values human-readable (for example,
`Portfolio default`, `Fixed`, or `±5 pp`); internal enum values never appear in
the interface. The Add asset action is a ghost action in the Investments
header on desktop and a bottom-right FAB on mobile.

Search results and unpriced results use the shared neutral `AssetTickerChip`.
Collection cards and tables show the ticker as light text on the asset-name
line, separated by a centered dot. They expose the same `Average cost basis`
label and show an em dash when the fixture has no value. Numeric headers,
values, and drift remain right-aligned and do not wrap.

Fee controls use the compact `SegmentedControl` for policy selection. Portfolio
settings render the selected Zero, Fixed, or Variable form inline. Asset forms
show the current policy and configured values with a compact edit icon; that
editor opens one nested Base UI overlay, a dialog on desktop or a drawer on
mobile. The editor has `%` and `EUR` suffix inputs, one Save action, and a
close icon/Escape dismissal path; it never adds a redundant Cancel button.
Dialog and drawer content may use `layer="nested"` to raise the overlay above
its parent while preserving focus restoration.

## Build new UI

1. Read the relevant Storybook stories and the frontend domain glossary.
2. Choose an existing primitive or pattern before adding a new one.
3. Add a typed component only when the interaction or visual contract is
   reusable.
4. Compose Base UI primitives through their supported composition API. Use
   `render` where a component needs to render another element; keep public
   APIs free of Radix-style `asChild` contracts.
5. Expose finite semantic variants and named slots. `className` is for layout
   composition, not for replacing tokens or adding arbitrary visual values.
6. Add stories for the states a user can reach, then add the component to the
   appropriate journey composition when it represents product behaviour.

New design-system components use Base UI. Radix and Vaul remain supported for
legacy consumers under `src/components/`; migrate those consumers separately
when a feature is intentionally changed. Do not import either legacy library
into a new design-system component.

## Naming and domain boundaries

Name product patterns with the frontend glossary first. Prefer
`PortfolioAssetCard`, `PortfolioAssetTable`, and `DriftDiagnostic` over vague
names such as `AssetCard`, `TableThing`, or `DriftAlert`. User-facing copy may
still use the language that is clearest to investors.

Keep domain and data orchestration outside presentational components. A pattern
may receive a normalized presentation model, callbacks, and state, but it does
not fetch data or perform portfolio calculations. `PortfolioAssetCollection`
is the responsive contract: it renders an equivalent asset-card experience on
small screens and a dense table on larger screens, with equivalent labels,
actions, and accessible names.

## Storybook coverage

Organize stories under these groups:

- `Foundations` for colours, typography, spacing, radius, and elevation;
- `UI` for reusable primitives;
- `Patterns` for domain-shaped reusable components;
- `Compositions` for portfolio overview, add asset, edit/remove asset,
  portfolio settings, and allocate/rebalance journeys.

Stories should use deterministic data and typed CSF metadata. Show the light
theme by default and include dark-shell examples where navigation or overlay
contrast matters. Use mobile and desktop viewport presets for responsive
patterns.

Document intentional user states, not every possible prop combination. At
minimum, cover disabled, loading, invalid, success, destructive, focus,
keyboard, open/closed, and responsive states when the component supports
them. Journey compositions should also show first-use and populated overview,
search loading/empty/error/unpriced results, save success/error, drift below,
within, and above target, registration-gated history, and dialog focus,
Escape dismissal, and focus restoration.

Use the accessibility panel during review and keep the Storybook smoke test
covering representative journeys at mobile and desktop widths. Composition
Docs stories use a tall iframe (at least 900px) so the closed journey can be
inspected and scrolled; interactive dialogs and menus stay Canvas-only. A story is
complete when it renders without console errors, has an accessible name and
focus path, and makes its state visible without relying on colour alone.

For visual comparison, open the matching closed overview story and capture it
at 1440×900 and 390×844 before inspecting an interactive Canvas story. Compare
the same state against the supplied journey board, then record any mismatch in
the review notes before adjusting tokens or patterns. Keep dialogs, drawers,
menus, and mutually exclusive states Canvas-only (or non-inline in docs) so a
docs page never mounts several overlays at once. Use the closed overview
stories as the stable documentation examples and navigate to each interactive
state separately.

## Change checklist

- Read the relevant Storybook story and `dcapal-frontend/CONTEXT.md`.
- Reuse semantic tokens and an existing primitive or pattern where possible.
- Keep new design-system source typed and independent of Radix/Vaul.
- Preserve equivalent mobile and desktop behaviour where a layout changes.
- Add intentional state stories and accessibility coverage.
- Keep feature data, queries, validation, and calculations in hooks or pure
  modules outside presentational components.
- Update the relevant ADR or glossary term when a new durable design or domain
  decision is made.
