# Allocate journey screen map

Status: design reference

This map turns the current allocate flow into a linked screen journey. The source is the live DcaPal flow at `dcapal.com` and the end-to-end journey specs on `origin/master`:

- `tests/journeys/create-portfolio.spec.ts`
- `tests/journeys/edit-portfolio.spec.ts`
- `tests/journeys/import-portfolio.spec.ts`
- `tests/journeys/manage-portfolios.spec.ts`
- `tests/journeys/search-assets.spec.ts`
- `tests/journeys/transaction-fees.spec.ts`

The storyboard boards use option 3 as the visual direction. The revised storyboard treats the portfolio overview as the main hub: asset search, asset editing, removal, settings, allocation, and rebalancing branch from it. The test files remain the source of truth for validation and error states.

## Shared interaction decisions

- Mobile uses asset cards. Desktop may use a denser holdings table.
- Chart ranges are text links below the chart: `1Y`, `3Y`, `5Y`, `Max`.
- Desktop and mobile do not use a persistent bottom navigation bar.
- The portfolio overview owns the primary actions. A floating action button opens `Add asset`, `Allocate`, `Rebalance`, and `Edit portfolio`.
- `Simple` and `Strategic` are portfolio settings, not a persistent editor toggle.
- Asset operations open dialogs from the overview. The edit dialog keeps the existing fields: asset class, current value, current price, shares, PMC / basis cost, target allocation, drift band, and transaction fees.

## Screen links

| Screen | State | Primary source | Main link or submenu |
| --- | --- | --- | --- |
| 01 | DcaPal landing | `create-portfolio.spec.ts` | `Allocate savings` → 02; `Import portfolio` → import entry |
| 02 | My portfolios | `manage-portfolios.spec.ts` | `New portfolio` → 03; card edit → 04 |
| 03 | New portfolio form | `create-portfolio.spec.ts` | `Next` → 05 |
| 04 | Portfolio action menu and settings | `manage-portfolios.spec.ts`, `transaction-fees.spec.ts` | Rename, duplicate, delete; settings opens allocation method and fee choices |
| 05 | Asset setup and empty search | `search-assets.spec.ts` | Search field → 06 |
| 06 | Search results | `search-assets.spec.ts` | Select `VWCE.DE` → 07 |
| 07 | Asset detail card | `create-portfolio.spec.ts`, `edit-portfolio.spec.ts` | Transaction fees → 08; fill quantity and target → 09 |
| 08 | Transaction fee submenu | `transaction-fees.spec.ts` | Save fees → 07; variable fees expose percentage, minimum, and maximum |
| 09 | Completed asset editor | `create-portfolio.spec.ts` | `Confirm weights` → 10 |
| 10 | Allocation amount and options | `create-portfolio.spec.ts` | `Allocate budget` → 11 |
| 11 | Allocation result | existing `/allocate` result state | `Back to portfolio` → 12; update portfolio returns to the editor |
| 12 | Portfolio overview, mobile hub | portfolio redesign brief | Chart links, drift alert, holding cards; card tap → edit dialog; FAB → add, allocate, rebalance, or edit portfolio |
| 13 | Portfolio overview, desktop hub | portfolio redesign brief | Dense holdings table with row edit/add controls; no bottom navigation; expanded FAB actions |

## Overview branches

| Branch | Screens | Main link |
| --- | --- | --- |
| Add asset | Overview → Add asset dialog → Search results → New asset edit dialog | Save asset → overview with the new asset |
| Edit and remove asset | Overview → Edit asset dialog → Transaction fees dialog or Remove confirmation | Save changes → overview; Remove asset → overview without the asset |
| Portfolio settings | Overview → Portfolio settings → Strategic allocation configuration | Save → overview; allocation method stays hidden until settings is opened |
| Allocate or rebalance | Overview FAB → Allocation options → Allocation result → Overview | Both `Allocate` and `Rebalance` enter the existing allocation flow |

## Latest storyboard assets

These are the four latest generated boards used as the visual reference for this epic:

- [Add asset and search flow](assets/add-asset-flow.png)
- [Edit and remove asset flow](assets/edit-remove-asset-flow.png)
- [Portfolio settings and strategic allocation flow](assets/portfolio-settings-flow.png)
- [Allocate and rebalance flow](assets/allocate-rebalance-flow.png)

## Test states to preserve during implementation

The primary boards do not expand every validation state. Implementation should still preserve the states covered by `origin/master`: import success and unresolved-price error, search loading and empty/error results, under/exact/over target totals, portfolio rename/duplicate/delete, and zero/fixed/variable fee validation.
