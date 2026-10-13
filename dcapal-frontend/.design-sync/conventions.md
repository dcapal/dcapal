# DcaPal design system conventions

DcaPal is a self-directed investing app (portfolio, drift from target, allocate and rebalance). The UI is warm, calm and data-dense: ivory canvas, white bordered cards, one rust accent, Instrument Sans.

## Setup (always)

Every component reads its tokens from the `.dcapal-theme` scope. Without it you get unstyled, wrong-font output. Wrap the whole screen once in `DcapalThemeScope` (it renders a `div.dcapal-theme`), and load `styles.css` then `_ds_bundle.js`:

```jsx
const { DcapalThemeScope, Card, CardHeader, CardTitle, CardContent, Button, Badge } = window.Dcapal;
<DcapalThemeScope>
  <Card>
    <CardHeader><CardTitle>Total drift</CardTitle></CardHeader>
    <CardContent>
      <div style={{ display: "flex", gap: "var(--dcapal-space-3)", alignItems: "center" }}>
        <Badge variant="warning">-23.1 pp</Badge>
        <Button variant="primary">Add money</Button>
      </div>
    </CardContent>
  </Card>
</DcapalThemeScope>
```

Portaled overlays (Dialog, Drawer, menus) already carry the scope; do not wrap them again.

## Styling idiom: components first, tokens for glue

There is no utility-class system. Build screens from the exported components; use inline styles with `var(--dcapal-*)` tokens only for your own layout glue (flex/grid, gaps, padding). Do not invent hex colors, sizes or `ds-*` class names: component styling is internal.

Token families (all declared in `_ds_bundle.css`, read it for the full list):
- Surfaces and text: `--dcapal-canvas` (page), `--dcapal-surface`, `--dcapal-surface-subtle`, `--dcapal-surface-emphasis`, `--dcapal-foreground`, `--dcapal-foreground-muted`, `--dcapal-border`, `--dcapal-border-strong`.
- Accent and status: `--dcapal-primary`, `--dcapal-primary-foreground`, `--dcapal-primary-soft`; `--dcapal-success`, `--dcapal-warning`, `--dcapal-destructive`, `--dcapal-info`, each with a `-soft` background pair (for example `--dcapal-success-soft`).
- Type: `--dcapal-font-sans` (Instrument Sans), sizes `--dcapal-text-xs` to `--dcapal-text-2xl`.
- Space and shape: `--dcapal-space-1` to `--dcapal-space-16`, `--dcapal-radius-sm|md|lg|pill`, `--dcapal-elevation-raised|overlay`.

## Rules of the look

- One primary (rust) button per view; secondary actions are `variant="secondary"`, `outline` or `ghost`. Button variants: primary, secondary, outline, ghost, destructive, link; sizes sm, md, lg, icon.
- Status is never colour alone: pair `Badge` (neutral, primary, success, warning, destructive) or `Alert` (info, success, warning, destructive) with text. Buys are success, sells destructive.
- Money and percentages are right-aligned tabular numbers; amounts use the portfolio currency (EUR), weights one decimal, drift in percentage points ("+25.9 pp").
- Prefer the product patterns over rebuilding them: `AppShell`/`PortfolioWorkspace` for page chrome, `DriftOverview`, `InvestmentPlanForm`, `InvestmentProposal`, `ProposedTransactionCollection`, `InvestmentResult`, `PortfolioAssetCollection`. Pattern components are presentational: pass view-model props and callbacks.
- Layouts respond to the width of their container, not the viewport; keep a screen inside `PortfolioWorkspace` or a width-constrained div.

## Where to look

- Per component: `components/<group>/<Name>/<Name>.prompt.md` (usage) and `<Name>.d.ts` (props).
- Styles: `_ds_bundle.css` (imported by `styles.css`). Components without a preview card (Badge, Tabs, Table, Select, Input, Tooltip, MetricCard, PageHeader) are still on `window.Dcapal`; read their usage in the bundle before use.
