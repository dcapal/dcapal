# design-sync notes (dcapal-frontend)

Run everything from `dcapal-frontend/` (the converter resolves `.design-sync/`, `overrides/`, `@ds-stories/` against cwd; config lives here, not at the git root).

## Setup
- App, not a library: no dist. `buildCmd` = `node scripts/build-design-system-bundle.mjs` -> `.design-sync/dist/` (index.js ESM of `src/design-system/design-sync-entry.ts` with npm imports external; index.css = compiled Tailwind preflight + design-system tokens/components; `types/` .d.ts via tsc; package.json named `@dcapal/frontend` so the converter treats it as PKG_DIR). Pass `--entry .design-sync/dist/index.js` (also in config `entry`).
- Scope = UI primitives + patterns only. `design-sync-entry.ts` omits `compositions.tsx`, `invest-composition.tsx`, fixtures. Compositions / Foundations story titles are dropped by the forked adapter.
- `DcapalThemeScope` (src/design-system/design-sync-theme-scope.tsx) is exported only via the entry and set as `cfg.provider`; it replaces the storybook decorator (bundling `.storybook/preview.tsx` fails: `[GENERAL]` src/style.css has `url(@images/...svg)` with no loader -> "decorator bundle failed").
- Reference storybook: `npx storybook build -c .storybook -o .design-sync/sb-reference --disable-telemetry` (rebuild when stories/DS source change).
- Fork `.design-sync/overrides/source-storybook.mjs` (libOverrides, needs `ln -sfn ../.ds-sync/node_modules .design-sync/node_modules` per clone): story titles are catalogs (UI / Primitives, Patterns / Portfolio, Patterns / Invest), so `.design-sync/story-owners.json` maps story id -> component export. New stories must be added there or they are dropped (build prints `! story-owners: no owner ...`).
- `storyImports.shim` forces `components/ui/` and `components/patterns/` imports in stories (barrels, not component-named files) to `window.Dcapal`; fixtures bundle from source into previews.
- Fonts: Instrument Sans via remote Google Fonts @import (`[FONT_REMOTE]`, renders correctly in compare). Needs network egress.
- cardMode: column for AppShell, FloatingActionMenu, PortfolioFooter, PortfolioIndex, PortfolioSetupPage, PortfolioWorkspace; single for Dialog (primaryStory DialogStates).

## Known render warns
- `[RENDER_THIN]` PortfolioFooter: mobile and desktop stories render identically at card width.

## Solo-phase fixes
- [GENERAL] Text looked heavier in previews than storybook (same Instrument Sans 500, same CSS) -> app's `body { antialiased }` (`-webkit-font-smoothing: antialiased`) lives in src/style.css, not shipped by the design-system CSS -> `ANTIALIAS_RULE` appended to the bundle css in `scripts/build-design-system-bundle.mjs` (applies to `.dcapal-theme`, which portaled overlays also carry; an inline style on DcapalThemeScope would NOT reach Dialog/Popover portals).
- [GENERAL] Framing noise, not a bug: compare screenshots the storybook root element (full height, cream canvas, no padding) vs the preview viewport (white page, 24px body padding, canvas only as tall as content). Judge the component. Same for "Open dialog"-style trigger strips.
- [GENERAL] Compare caps at 6 stories/component; use `--max-stories 11` (InvestmentProposal has 11) or tail stories stay ungraded.
- InvestmentProposal: tall stories (Add Money 1211px, Rebalance 1373px) were cropped on the preview side at the 900x700 capture viewport -> `overrides.InvestmentProposal.viewport = "900x1400"` (full build; also becomes the card viewport). Other tall components (PortfolioWorkspace, PortfolioSetupPage, AppShell, PortfolioIndex) likely want the same.
- Storybook mobile viewport params are not applied by compare on either side; *Mobile stories render at 900px on both sides (identical, graded match).
- PortfolioIconPicker story only shows the closed ghost trigger (MoreHorizontal icon); no remote images anywhere in solo set, no [ASSETS_BLOCKED]; the only remote asset is the Google Fonts @import (loaded OK, verified computed font Instrument Sans 500 on both sides).
- Graded all `match`: Button (2), Dialog (1), AssetTickerChip (1), PortfolioIconPicker (1), InvestmentProposal (11).

## Re-sync risks
- Many components share one story file: any edit to it re-grades all of them.
- story-owners.json is hand-maintained; owner guesses (e.g. form-states -> FormField, feedback -> Alert, dialog-states -> Dialog) are not verified.
- Exports without stories (Tabs, Table, Select, Input, Badge, ...) are not emitted in the storybook shape.
- Fork of source-storybook.mjs can drift from upstream; diff on re-sync.
- ANTIALIAS_RULE in the build script is a design-sync-only addition; if src/style.css body styles change, mirror them.
- Preflight in index.css is included to mirror the app (reference loads src/style.css); app globals (`body` styles, `* {border-border}`) are not shipped.
