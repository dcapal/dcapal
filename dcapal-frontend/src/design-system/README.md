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

The current direction follows the selected Editorial wealth-planner boards:

- a shared warm-ivory canvas for pages, navigation, the portfolio rail, and footer;
- deep ink text with muted supporting text and quiet dividers;
- rust primary actions, with separate positive, warning, information, and error colours;
- compact cards, thin borders, small radii, and restrained elevation;
- Instrument Sans throughout, loaded through Google Fonts with a local sans-serif fallback;
- equivalent mobile cards and desktop tables for portfolio assets.

Use semantic tokens from `styles/` for every visual decision. Keep colour,
spacing, radius, typography, and elevation values out of component files. The
`dcapal-theme` scope protects existing application screens from changes to the
new token layer. Do not add bottom navigation to portfolio journeys: on phones
the top bar's "Open navigation menu" button opens the canonical portfolio
drawer, and desktop uses the sidebar rail.

Card spacing is owned by the `Card` primitive. `CardContent` is padded when it
stands alone; adjacent header, content, and footer slots remove only their
shared interior edge. Use the finite `density="default" | "compact"` option
when a board calls for a tighter card, not a one-off padding class.
Use `variant="ghost"` for onboarding surfaces that should sit on the shared
canvas without a raised card treatment. `PortfolioFooter` and
`PortfolioWorkspaceSidebar` use the page canvas by default. The `warm` footer
tone remains available for compatibility and resolves to that same canvas;
pass `tone="dark"` only when the dark footer is intentional.

`PortfolioWorkspaceSidebar` is the canonical portfolio navigation for both the
desktop rail and mobile drawer. Portfolio identity uses the typed
`PortfolioIcon` registry and `PortfolioIconPicker`; do not pass arbitrary icon
nodes or build a second picker inside a card.

Use `InputWithSuffix` for currency and percentage fields so the unit remains
inside the control. Keep trigger values human-readable (for example, `Default`,
`Fixed`, or `±5 pp`); internal enum values never appear in the interface. Add
asset is an overview action that opens a dialog on desktop and a full-height
drawer on mobile. It is not a sidebar item. On mobile, the top bar's menu
button opens the shared navigation drawer; desktop never renders that button.

### Phone / desktop boundary

Every phone/desktop split uses one boundary: phone is `max-width: 47.99rem`,
desktop is `min-width: 48rem`, and `useNarrowViewport` matches the phone query.
At exactly 768px (48rem) only the desktop layout renders: header actions, no
floating menu, the table instead of cards, dialogs instead of drawers.

### Page actions menu (phones)

- `PortfolioWorkspace` takes `pageActions` (label, icon, `primary`, `disabled`,
  `onClick`). Below 48rem they appear behind one floating "+" button
  (`FloatingActionMenu`, "Open page actions") at the bottom right; from 48rem
  the button is `display: none` and the page header carries the same actions.
- The menu holds only the current page's actions, never navigation. Navigation
  stays in the top bar menu button and the desktop rail.
- Open, it is a modal Base UI dialog: a dimmed scrim, pill actions (label then a
  circular icon, 44px tall) stacked above the button, the primary one in the
  primary colour, and the button turned into a close button in the same spot.
  Escape, the scrim and the close button close it; focus moves to the first
  action and returns to the floating button. A chosen action runs after the
  menu has closed and keeps the focus its own dialog or drawer takes.
- Sticky action bar rule: a step that pins a `StickyActionBar` on phones (Invest
  plan, proposal, result) supplies no `pageActions`, so the button and the bar
  never share the corner. If both ever coexist the button sits above the bar
  through `--dcapal-sticky-action-bar-offset`. The bar uses
  `--dcapal-z-sticky-bar`, the button `--dcapal-z-fab`, the open menu the
  overlay tokens.
- Pages: Overview = Add asset (primary), Invest; Invest drift = Add money
  (primary), Rebalance (both disabled while targets are blocked); Portfolio
  settings = Save changes (primary) and, for a Strategic portfolio, Review
  strategic allocation.
- Stories (`Patterns / Portfolio`): `Floating actions mobile`, `... invest
  mobile`, `... settings mobile` (closed) and `Floating actions open mobile`,
  `... invest open mobile`, `... settings open mobile` (Canvas-only);
  `Floating actions desktop` shows no button.

### First-use setup wizard

- `PortfolioSetupWizard` is the "Set up your portfolio" card of an empty
  overview: one white card with a segmented progress bar (Name, First asset,
  Target weight; reached steps can be reopened), the step's title and one-line
  reason on the left and its real form on the right (name and base currency;
  asset search with popular picks; the weight with live validation), with
  Back, Continue and Finish setup. On phones it is the same pattern in one
  column with 44px controls.
- Finishing the last step removes the card; the overview then shows the
  portfolio itself, with no "all done" state. `usePortfolioSetup` owns the step
  state and `portfolio-setup-wizard.ts` the pure rules (validation, which steps
  can be opened, search, step wording); `PortfolioSetupPage` is the standalone
  "New portfolio" page around the same wizard.
- The last step follows the allocation method: Simple asks for the "Target
  weight" ("Set target weights"), Strategic for the "Asset class weight" ("Set
  the class weight"). Both must total 100% while there is one asset.
- Stories: Portfolio overview `First use`, `First use name complete`, `First use
  asset added`, `First use weights complete` (card gone), `First use asset
  added mobile`.

Search results and unpriced results use the shared neutral `AssetTickerChip`.

### Portfolio asset collection

`PortfolioAssetCollection` is the only component that shows portfolio assets.
Overview, asset-management and edit surfaces all render it; do not build a
second holdings table or card list. It owns one `mode` and one sort and
collapse state, and renders a table (from 48rem) and cards (below 48rem) from
the same model, so both always agree.

- `mode` is `simple` (default) or `strategic`; it is controlled with
  `mode` / `onModeChange` or started with `defaultMode`. A segmented
  "Table mode" control in the header switches it; hide it with
  `showModeToggle={false}` on surfaces that fix the mode. Product compositions
  (overview, edit asset) hide it: their mode is the Portfolio's allocation
  method. The toggle stays in the standalone pattern stories for demonstration.
  `targets` passes a stated Strategic allocation (class targets and shares) so
  the table shows them as entered, and `onEditClass` adds an `Edit {Class}`
  button to every class row and class card.
- `presentation` is the column density: `detailed` (quantity, price, cost
  basis) or `overview` (value, weights and drift only).
- Simple is a flat list. Detailed adds a dedicated "Asset class" column;
  overview shows the class muted after the ticker chip. Strategic groups
  assets under one row per asset class (collapse button, count, value, total
  invested, current weight, class target, drift) with a "Portfolio total"
  row. Asset rows show their share of the class ("x% of class") under the
  weights, where the target share is the relative target weight. Use
  `defaultCollapsedClasses` to start with classes collapsed.
- Below 48rem the header shows a compact "Table options" button (size `sm`,
  outline) instead of the inline mode switch. It opens one Base UI drawer with
  the Table mode switch, the "Sort by" select and a reverse-order button; changes
  apply live to the cards and Done or Escape closes it. From 48rem the mode
  switch sits inline and the table headers sort, so the button is hidden.
- `headerActions` renders in the header toolbar on every viewport. The overview
  composition passes none: on phones Add asset and Invest live in the floating
  page-actions menu, and from 48rem the page header carries them. The page
  header never renders a full-width Add asset button.
- When the table is wider than its container the wrapper scrolls horizontally
  and the actions column (Edit / More actions) stays pinned on the right in the
  header, asset, group and total rows, each with the row's own opaque background.
  `Table` sets `data-overflow-end` while columns are hidden to the right, which
  adds a subtle left shadow to the pinned column.
- Column headers that sort are real buttons inside the `th`, which carries
  `aria-sort`. The default order is target weight descending. Strategic
  sorting orders classes by their aggregate and assets within each class.
- Asset weights and drift are the stored `currentWeight` / `driftPp` of each
  asset, and class weights are the sums of their assets' stored weights (a
  stated Strategic allocation supplies the class target); only value and
  invested totals come from `value` and cost basis. The derivation lives in the
  pure module `portfolio-asset-groups.ts` (`buildPortfolioAssetSummary`).
- One Portfolio-wide Drift band (`DEFAULT_DRIFT_BAND_PP`, 5 pp, set with the
  collection's `driftBandPp`) classifies drift everywhere: table, cards, class
  groups, badges and the Invest drift overview. Drift within the band
  (inclusive) is on target, so +4.8 pp reads "On target".
- Drift is a badge with a signed value (`+2.5 pp`) plus a tooltip and
  screen-reader text (Over, Under or On target), so status never relies on
  colour alone.
- Asset names sit on one line beside the ticker chip and truncate with a title
  tooltip. On mobile, names wrap beside the drift badge. Edit and "More
  actions" icon buttons keep the `Edit {ticker}` and
  `More actions for {ticker}` names.

Fee controls use the compact `SegmentedControl` for policy selection. Portfolio
settings render the selected Zero, Fixed, or Variable form inline. Asset forms
show the current policy and configured values with a compact edit icon; that
editor opens one nested Base UI overlay, a dialog on desktop or a drawer on
mobile. The editor has `%` and `EUR` suffix inputs, one Save action, and a
close icon/Escape dismissal path; it never adds a redundant Cancel button.
Dialog and drawer content may use `layer="nested"` to raise the overlay above
its parent while preserving focus restoration.

### Allocation method and weight editing

The Portfolio's allocation method (Simple or Strategic) is set only in
Portfolio settings, which holds the method radio cards, one Portfolio-wide
Drift band and the transaction fee configuration. There are no per-class drift
bands and no separate Strategic allocation screen. With Strategic saved, a note
says weights are edited from the Portfolio page and links there ("Go to
Portfolio"); the overview header gear opens settings (`onOpenSettings`).

- Saving a move from Simple to Strategic opens `SetClassWeightsDialog`
  ("Set your asset class weights"): every class (Equities, Bonds, Cash,
  Crypto, Commodities, Other) in one table with a stacked preview bar, current
  weight and gap, and the starters "Use my current targets", "Use my current
  mix" and "Clear". Save is always enabled; totals that miss 100% (or weight on
  a class without assets) only show a warning that Invest stays unavailable.
  The warning is validated against the `shares` prop, the shares that will
  really be stored on save (`usePortfolioAllocation().sharesOnSwitch`), so it
  equals the Invest gate. Typed weights are never clamped: text that is not a
  number from 0 to 100 keeps its value, gets an inline "Enter a number from 0
  to 100" error (`aria-invalid` plus `aria-describedby`) and counts as 0.
  Cancel or Escape keeps Simple. A drawer replaces the dialog below 48rem
  (`ResponsiveOverlay`).
- In Strategic mode the edit icon of a class row opens `EditAssetClassDialog`
  ("Edit Equities"), which changes only the class weight and shows asset
  shares read-only. The edit icon of an asset row opens
  `EditPortfolioAssetDialog`: Simple keeps the single absolute "Target weight";
  Strategic shows "Share of {Class}" (relative target weight) with the
  resulting weight in portfolio and each sibling's share, and a warning when
  shares do not total 100%. Save is always enabled.
- Nothing is redistributed automatically anywhere. The pure module
  `strategic-allocation.ts` owns the starters, derived absolute weights (class
  weight x share), previews and totals ("Balanced", "Short by x pp", "Over by x
  pp"); `checkStrategicTotals` reuses `checkTargetTotals`, the same gate Invest
  uses, so what the dialogs report is what blocks Invest. Shares are stored, not
  back-derived, so clearing the class weights never loses them.
  `usePortfolioAllocation` holds the method and weights for a composition.
  Simple edits are never lost on a switch: when an absolute weight was edited
  since Strategic was last in force, the shares are re-derived from the
  current absolute weights (`allocationOnSwitchToStrategic`); otherwise the
  stored shares are kept. Switching back derives absolutes from class target
  x share.
- The setup wizard's "Set target weights" step and the Add asset form respect
  the method: Simple asks for the absolute Target weight; Strategic asks for the
  asset class weight (wizard) or the "Share of {Class}" (Add asset).
- Stories (Canvas-only for dialogs): Portfolio settings `Settings simple`,
  `Settings strategic`, `Set class weights`, `Set class weights unbalanced`;
  Portfolio overview `Portfolio overview strategic`, `Edit class dialog`; Edit
  and remove asset `Edit asset strategic`, `Edit asset simple`; each with a
  Mobile sibling.

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
actions, and accessible names. Its table mode is described under Portfolio
asset collection.

## Storybook coverage

Organize stories under these groups:

- `Foundations` for colours, typography, spacing, radius, and elevation;
- `UI` for reusable primitives;
- `Patterns` for domain-shaped reusable components;
- `Compositions` for portfolio overview, add asset, edit/remove asset,
  portfolio settings, and the Invest journey (drift, plan, proposal, result).

The Invest composition (`Compositions / Invest`) is a fixture-driven demo of
the `Patterns / Invest` components: `DriftOverview`, `InvestmentPlanForm`,
`InvestmentProposal` (every proposal state), and `InvestmentResult`. Portfolio
navigation has one Invest entry; Add money and Rebalance are paths inside the
journey, offered as a primary Add money and a secondary Rebalance button
(`InvestmentPathActions`) in the page header, outside the `DriftOverview` card,
and switched with a full-width segmented control across the plan's Budget and
rules card. Invest layouts respond to the width of their own container (CSS
container queries), not the viewport, because the portfolio sidebar narrows the
main column; only the pinned mobile action bar follows the viewport. Metric
values may wrap at a "→" but never inside a figure; copy says "transactions", never "orders". Its figures are static,
so the optimizer is not needed to run Storybook.

The Invest components take `mode: "simple" | "strategic"` (default `simple`,
which keeps the flat asset lists). In Strategic allocation every step that lists
assets lists asset classes first and nests the assets below: `DriftOverview` and
the plan's "Where you are today" (`DriftClassGroupList`), the grouped
`ProposedTransactionCollection` (`groups`), class-level `AllocationAfterList`,
and the grouped `InvestmentResult`. Class figures are derived, never stored, by
the pure helpers in `invest-classes.ts`: a class target is the sum of its
assets' absolute targets, an asset's relative share is its target over that
class target, and Total drift, Largest gap and the proposal's drift before and
after are measured on class weights. Classes sort by widest gap (drift) or by
money moved (proposal); cash is a class row only. One Portfolio-wide Drift band
judges class and asset rows alike; there are no per-class bands. When target
weights do not total 100% (asset weights in Simple mode; class targets and the
shares inside each class in Strategic mode), `checkTargetTotals` in
`invest-targets.ts` blocks Add money and Rebalance, the proposal's blocked
state quotes the real totals, and `onEditTargets` is called from "Edit targets".
Strategic stories: `DriftStrategic`, `PlanStrategic`,
`ProposalStrategicAddMoney`, `ProposalStrategicRebalance`, `UpdatedStrategic`
and `ProposalBlockedStrategic`, each with Mobile (and, for drift and proposal,
Laptop and Compact) siblings.

Stories should use deterministic data and typed CSF metadata. Show the light
theme by default and include dark-shell examples where navigation or overlay
contrast matters. Use mobile and desktop viewport presets for responsive
patterns.

Document intentional user states, not every possible prop combination. At
minimum, cover disabled, loading, invalid, success, destructive, focus,
keyboard, open/closed, and responsive states when the component supports
them. Journey compositions should also show empty, name-complete, asset-added,
target-weights-complete, and populated overview,
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
- Use `PortfolioAssetCollection` for every responsive asset presentation; do
  not introduce a second holdings table for a shell composition.
- Keep first-use metric placeholders and the setup wizard's progress visible in
  every deterministic overview stage until the setup is finished.
- Keep feature data, queries, validation, and calculations in hooks or pure
  modules outside presentational components.
- Update the relevant ADR or glossary term when a new durable design or domain
  decision is made.
