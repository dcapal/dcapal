# Storybook visual QA — Editorial wealth planner

## Design source and scope

This review covers the design-system Storybook. The selected direction board is
`/Users/leonardoarcari/.codex/generated_images/01a0ba37-161a-7a80-a563-a239fd1874d3/exec-1073352e-bd1d-4ab2-b9a2-9e77db5c5e46.png`.
The eight journey boards are stored beside it as `exec-89cf1fe0-5a72-4eed-a825-cd47c0ef394c.png`,
`exec-38d84ed0-1467-4ec7-9b12-1d718bfd75c6.png`,
`exec-98921c11-29c2-41f7-9ab0-6ecc2f8ef118.png`,
`exec-4c280e41-cd0f-4513-a90a-dc1b9b0f9036.png`,
`exec-31bc84f1-5efc-4ded-92bf-c7d1533129fe.png`,
`exec-f8da9b1a-b3fd-4a89-8f91-eb770939d20b.png`,
`exec-3e428371-ccca-4591-a34e-58da1fe43d25.png`, and
`exec-8b00f541-70e9-41fc-92b8-e93ca32f4005.png`.

The selected board sets the warm ivory canvas, restrained rust actions, clear
financial status colors, readable portfolio hierarchy, and responsive layouts.
The implementation plan intentionally uses Instrument Sans throughout, even
though the visual boards use serif display headings. This is a deliberate
typography choice, not an accidental mismatch.

Add asset follows the user’s requested earlier interaction model: it is an
overview action, opens in a desktop dialog and a full-screen mobile drawer, and
does not appear in the portfolio sidebar. The mobile floating action button
opens the portfolio navigation drawer only; it is not shown on desktop.

## Review results

The first visual review covered 81 stories across Foundations, UI, Patterns,
and Compositions. It found five clear issues:

1. Desktop asset tables exceeded the workspace width. The shared asset
   collection now has a compact overview presentation and a detailed table
   presentation with reachable actions.
2. The first-asset setup state showed a single holding but full-portfolio
   metrics. Its value and day-change now use the displayed holding, with no
   cash metric before a cash balance exists.
3. Rebalance review omitted the per-asset comparison. It now shows current,
   target, and proposed weights in the shared semantic Table component, with
   buys, sells, costs, and cash adjustment kept in a separate transaction
   summary.
4. The empty portfolio list lacked a clear first-use message. It now says
   “You don’t have any portfolios yet” and includes a reusable icon
   illustration.
5. A gated chart still said “Historical analytics.” The label now reads
   “Portfolio performance.”

A follow-up browser check at the Storybook desktop 1440 × 900 and mobile
390 × 844 presets found that the mobile rebalance table gave its asset column
too little width. Its mobile card layout now resets the desktop column widths,
so long asset names wrap at word boundaries.

A fresh reviewer then navigated through every entry in the Storybook index. The
index had 82 stories across Foundations, UI, Patterns, and Compositions. This
final pass found two more clear mismatches:

1. Rebalance trade amounts were 100 times too large because percentage-point
   changes were not converted to fractions. The calculation now divides by 100;
   the review shows realistic trades and a €2,368.93 cash adjustment.
2. Three Floating Actions pattern stories still showed the retired action menu
   and desktop FAB. They now use the shared workspace preview: mobile opens the
   navigation drawer, and desktop has no FAB.

The reviewer rechecked both fixes in Storybook and found no remaining clear
mismatches against the selected boards.

Some browser captures were narrower than the 1440-pixel desktop preset. The
reviewer used scrolling, zoom, and the accessibility tree to inspect content
outside the visible crop. This is a visual comparison, not a claim of full
accessibility compliance; automated browser checks remain blocked below.

## QA coverage

- Check the desktop and mobile portfolio index, empty first-use journey, and
  populated portfolio list.
- Check each first-use overview stage: no steps complete, name and currency,
  first asset, and target weights. Confirm metric placeholders and setup
  progress match the stage.
- Check overview actions, desktop Add asset dialog, mobile Add asset drawer,
  search loading/results/empty/error/unpriced states, and the asset form.
- Check allocation, rebalance options and result, per-asset review values,
  edit/remove, fees, settings, performance chart states, icon customization,
  sidebar navigation, mobile navigation drawer, and footer.
- Check asset identity wrapping and actions in both detailed desktop tables and
  mobile asset cards.

The Storybook was visually reviewed in the browser at desktop and mobile
presets. The full final pass covered all 82 indexed stories and checked the
updated rebalance and Floating Actions states after the fixes. Automated
`storybook:test` could not complete because Chromium failed to launch in this
macOS environment (`bootstrap_check_in` permission denied, error 1100). Treat
that as an environment limitation, not a passing smoke test.

## Final verification

- Typecheck: passed
- Lint: passed
- Unit tests: passed (5 files, 24 tests)
- Storybook build: passed (bundle-size advisory warnings only)
- `git diff --check`: passed
- Automated browser smoke: blocked by Chromium bootstrap permission error 1100
- Fresh all-story visual review: passed; 82 stories inspected, no remaining
  clear mismatches

## Allocation and rebalance composition refinement

The allocation/rebalance boards were checked again against the selected
editorial mockups, including the desktop rebalance review (screen 6) and the
stacked mobile results. Desktop options now pair the investment controls with
a current-versus-target snapshot. Review starts with a full-width, weights-only
comparison; buy/sell quantities, estimates, commissions, and cash adjustment
are presented separately rather than repeating transaction amounts in the
table. Mobile keeps the stacked cards and a single full-width primary action;
the existing navigation FAB remains available with reserved space below the
action.

The allocation fixture uses a €1,000 budget: 195 AGGH units at €5.12 cost
€998.40, the €1.00 commission and €0.60 remaining cash bring the total to
€1,000. The result also shows the holding moving from 24.1% to 24.7% of the
portfolio. Back returns to options, and confirming leaves a success state in
the review.

Visual inspection used the Storybook desktop 1440 × 900 and mobile 390 × 844
presets. The mobile review was manually confirmed in the in-app browser,
including the confirmation state. The desktop Storybook canvas was wider than
the visible in-app capture, so its far-right content was not fully legible in
the screenshot; the accessibility tree confirmed the complete review content.
The focused Playwright smoke run remained blocked because Chromium could not
register its macOS Mach rendezvous service (`bootstrap_check_in`, permission
error 1100). Typecheck, lint, unit tests, and Storybook build passed.

### Allocation and rebalance interaction update

The supplied allocation board is the reference for the options, allocation
review, and rebalance review stories. The options layout pairs the budget and
strategy controls with the portfolio's current/target allocation; desktop
rebalance review leads with the full-width weight comparison and places the
trade preview beneath it. Mobile keeps the same decisions and review data in a
single-column flow. The “Buy only” option is checked by default as shown in the
board; turning it off enables sales from overweight positions. Sell quantities
are capped at the displayed position value, and trade counts, commissions,
proposed weights, and post-trade cash are recalculated from the fixture's
displayed holdings and cash balance.

The default €1,000 buy-only rebalance preview shows €747.52 and €188.70 in
purchases, €2.00 estimated commission, €61.78 unallocated, and €3,903.93 cash
after. A €500 sell-capable preview validates the opposite path, including a
sale from an overweight position and €1,091.87 available cash after trades.
These are deterministic Storybook values, not financial advice or a live
portfolio calculation. Mobile comparison cards now keep current, target, and
proposed percentages on one compact line per holding, and transaction details
align each label with its value instead of creating a tall label/value stack.

### Screen 6 comparison and final mobile clearance

**Final result: passed**

- Source visual: `/Users/leonardoarcari/.codex/generated_images/01a0ba37-161a-7a80-a563-a239fd1874d3/exec-31bc84f1-5efc-4ded-92bf-c7d1533129fe.png` (1536 × 1024 px).
- Implementation capture: Storybook `Compositions / Allocate and rebalance / Rebalance Result` and its mobile options/result variants, opened and captured in the Codex in-app browser. Browser captures were reviewed inline and were not exported to a local screenshot file.
- Comparison viewports: desktop Storybook preset 1440 × 900 CSS px; mobile Storybook preset 390 × 844 CSS px. Source and implementation both used device-scale-independent CSS layouts; no image rescaling was used for the reviewer’s viewport measurements.
- Full-view comparison: the desktop review separates the current/target/proposed weight comparison from proposed purchase rows, following screen 6’s hierarchy. Mobile retains the stacked comparison and transaction cards.
- Focused comparison: the CTA/FAB collision was measured at 390 × 844. Before the fix, the fixed FAB covered the primary action. After the fix, the action ends at x=281 and the FAB begins at x=310 (29 px clearance). At scrollY=1004, both transaction cards and the confirm action are visible without intersection.
- Fidelity surfaces: the existing Editorial font and semantic rust/ivory tokens remain consistent with the rest of Storybook; spacing is compacted and aligned to the shared card system; no image assets are used by this composition; labels and values were checked against the deterministic fixture.
- Iteration history: a fresh reviewer found the mobile navigation FAB obscured the options CTA and a scrolled transaction card. The CTA now reserves a right-side FAB zone, and the composition has bottom scroll clearance. The reviewer rechecked all eight desktop/mobile allocation/rebalance variants and reported no remaining clear mismatches.
- Residual test gap: the focused Playwright smoke test still cannot launch Chromium in this macOS environment (`bootstrap_check_in`, permission error 1100). Typecheck, lint, unit tests, Storybook build, and live browser visual review passed.
