# Epic: Portfolio management hub and allocation workflows

## Problem Statement

DcaPal is useful today for defining a portfolio, choosing target assets and weights, and splitting a new investment amount across those assets. The current experience is still shaped around the allocation wizard. It does not give a self-directed investor a strong, persistent place to understand what they own, how the portfolio has moved, whether it has drifted, or how to change the portfolio without restarting the allocation flow.

The new product should make the Portfolio overview the main entry point. From there, an investor should be able to search for, add, edit, and remove Portfolio Assets; review quantity, market price, current value, average buy price, target weight, current weight, drift, and fees; open settings; and start allocation or rebalancing. The existing allocation flow remains a focused task flow reached from the overview.

The model must stay clear as the product grows. A Portfolio remains a lean collection of assets and their weights. Strategic allocation and other planning choices belong in separate entities. Simple allocation uses absolute target weights per asset. Strategic allocation uses Asset-class target weights plus relative target weights within each class, from which DcaPal derives the absolute asset weights. Bonds and cash are treated as defensive planning classes; other classes are risk-on. There is no tactical sleeve in this epic.

The experience is mobile first. Mobile uses cards that keep the holding's current value and ownership details visible. Desktop uses the same responsive system with a denser table where the wider viewport helps comparison. The product does not maintain separate mobile and desktop product lines and does not use a persistent bottom navigation bar.

## Solution

Build a responsive Portfolio management hub with four connected areas:

1. Portfolio overview: current value, day change, model performance history, drift diagnostics, and holdings.
2. Holding management: search, add, edit, fee details, and remove dialogs launched from the overview.
3. Portfolio settings: portfolio name, quote currency and fee defaults, plus the hidden-until-opened Simple or Strategic allocation mode and Strategic allocation guidance.
4. Allocation tasks: the existing allocation flow, extended for allocation and rebalancing, drift bands, buy-only recommendations, budget options, and a clear result that returns to the overview.

The overview is the hub. A floating action button opens `Add asset`, `Allocate`, `Rebalance`, and `Edit portfolio`. Asset editing is dialog-based so the user can make a focused change without losing the overview context. Settings contain the Simple or Strategic choice rather than displaying that choice on every edit screen.

In Simple mode, users enter absolute target weights for assets. In Strategic mode, users enter target weights for Asset Classes and relative weights for assets within each class. DcaPal calculates each absolute asset target as:

`Asset-class target weight × Relative target weight within the class = Absolute target weight`

The overview compares both current asset weights and aggregated Asset Class weights with their targets. Drift bands can be set at both levels and are expressed in percentage points. A drift diagnostic explains when a band has been crossed and provides an action to review or rebalance.

Model performance is calculated in the frontend from backend asset time series and the selected target weights. It is normalized to a common starting value, is not money weighted, and is not a Portfolio property. `1Y`, `3Y`, `5Y`, and `Max` are text links below the chart. Historical views remain a registration-gated feature for unauthenticated users because the server-side time-series path is heavier; the gate is also a clear registration CTA.

Strategic allocation guidance is optional and educational. The Bull rule is the first framework. It can explain its assumptions, show suggested weights, and be explicitly applied by the user. Future frameworks such as Merton's rule are not part of this implementation.

## User Stories

1. As a self-directed investor, I want the Portfolio overview to be the first screen after I open a portfolio, so that I can understand my holdings and choose the next action without entering the allocation wizard first.

2. As an investor with multiple portfolios, I want to switch between my portfolios from a clear portfolio selector, so that I can review each independent plan without confusing one portfolio's holdings or targets with another's.

3. As an investor experimenting with a different plan, I want to clone an existing Portfolio, so that I can change the clone's assets, weights, and settings without changing the original portfolio.

4. As an investor, I want a cloned Portfolio to copy its configuration and current holding inputs but not copy historical performance, so that the clone has an independent model history calculated for its own configuration.

5. As an investor, I want to see current Portfolio value, day change, quote currency, and the date of the displayed prices, so that I know the context of the figures before I make an allocation decision.

6. As an investor, I want to see a stacked model-performance time series for the Portfolio and its individual assets, so that I can understand how the selected model moved over time without implying that the chart is a transaction ledger or a money-weighted return.

7. As an unauthenticated visitor, I want to see a useful Portfolio overview and current allocation tools, so that I can evaluate DcaPal before registering.

8. As an unauthenticated visitor, I want a clear explanation and registration CTA when I open a historical view, so that I understand why registration is needed and what I will unlock.

9. As a registered investor, I want to choose `1Y`, `3Y`, `5Y`, or `Max` below the chart as clickable text, so that I can change the model-history range without spending space on large controls.

10. As a mobile investor, I want each holding to appear as a card showing its name, symbol, Asset Class, quantity owned, current price, current value, target weight, current weight, and drift, so that the information I need is readable without forcing a wide table onto a small screen.

11. As a desktop investor, I want the same holdings to appear in a denser table with the same fields and actions, so that I can compare many assets efficiently while using the same product model as mobile.

12. As an investor, I want to group holdings by Asset Class and sort them by current weight, target weight, current value, or drift, so that I can inspect the Portfolio from the perspective that helps me make a decision.

13. As an investor, I want individual assets to be the default grouping, so that the overview remains consistent with the current DcaPal experience while still allowing Asset Class summaries when I need them.

14. As an investor, I want to see an Asset Class summary with current weight, target weight, and drift, so that I can detect a strategic imbalance even when the individual asset drifts appear small.

15. As an investor, I want to search for an asset directly from the Portfolio overview, so that adding a holding does not require leaving the main Portfolio context.

16. As an investor, I want search results to show enough identifying information, including symbol, name, exchange or market when available, quote currency, and price availability, so that I can choose the intended asset before saving it.

17. As an investor, I want loading, empty, error, and unpriced search states to be explicit, so that I know whether to wait, refine the search, or choose another asset.

18. As an investor, I want to add a selected asset through a focused dialog, so that I can enter its holding and target details before it appears in the Portfolio.

19. As an investor, I want to edit a Portfolio Asset from its overview card or desktop row, so that I can correct or update the holding without navigating through a separate portfolio editor.

20. As an investor, I want the asset edit dialog to support quantity owned, current price, current value where applicable, average buy price or basis cost, target weight, asset class, and transaction-fee inputs, so that the Portfolio reflects both what I own and how I want to allocate it.

21. As an investor, I want current value and current weight to be derived consistently from quantity, price, quote currency, and the Portfolio total, so that the overview does not show contradictory holding figures.

22. As an investor, I want to record average buy price and transaction fees, so that I can see a more useful gain estimate and make allocation decisions that account for trading costs.

23. As an investor, I want Portfolio-level fee defaults and optional asset-level fee overrides, so that common fee rules are easy to maintain while exceptional instruments can use their own rules.

24. As an investor, I want zero, fixed, and variable fee settings to validate their minimum, maximum, percentage, and non-negative constraints, so that an invalid fee policy cannot silently distort an allocation result.

25. As an investor, I want to remove an asset through a confirmation dialog, so that an accidental tap does not delete a holding or its target from the Portfolio.

26. As an investor, I want cancelling an add, edit, fee, settings, or remove dialog to leave the Portfolio unchanged, so that exploring a form is safe and reversible.

27. As an investor, I want the Portfolio overview to show whether target weights total below, exactly, or above 100%, so that I can fix an invalid allocation before allocating money.

28. As an investor using Simple allocation mode, I want to enter absolute target weights directly for each Portfolio Asset, so that I can manage a flat allocation model without creating a separate class-level plan.

29. As an investor using Strategic allocation mode, I want to define target weights for Asset Classes and relative weights for the assets inside each class, so that I can express a two-level allocation policy that is easier to maintain as holdings change.

30. As an investor using Strategic allocation mode, I want DcaPal to derive each asset's absolute target weight from its Asset Class target and relative target, so that the allocation flow uses one unambiguous set of absolute weights.

31. As an investor, I want Simple or Strategic to be changed from Portfolio settings rather than shown as a persistent editor toggle, so that the overview and asset dialogs focus on the task at hand.

32. As an investor, I want Portfolio settings to contain name, quote currency, fee defaults, drift defaults, and allocation mode, so that configuration decisions are kept together and are easy to find.

33. As an investor, I want Asset Class target weights and Portfolio Asset relative weights to be stored as separate planning data, so that the Portfolio remains a lean asset-and-weight entity and future planning features can evolve without overloading it.

34. As an investor, I want sensible default Asset Classes for Equities, Bonds, Cash, Crypto, Commodities, and Other, so that I can classify common holdings quickly while retaining a path for less common assets.

35. As an investor, I want Bonds and Cash to be treated as defensive planning classes and all other classes as risk-on by default, so that strategic guidance can explain its risk split without claiming that any holding is literally risk free.

36. As an investor, I want to configure a drift band for an Asset Class, so that rebalancing guidance can wait until the class has moved beyond a tolerance that reflects my plan.

37. As an investor, I want to configure a drift band for an individual Portfolio Asset, so that I can give a specific holding a different tolerance from its Asset Class.

38. As an investor, I want drift to be displayed in percentage points against the applicable target, so that `+4.8 pp` and `-2.7 pp` have a clear meaning and can be compared with the configured band.

39. As an investor, I want the overview to surface a concise drift diagnostic when an Asset Class or asset crosses its band, so that I can decide whether to review, allocate, or rebalance without scanning every holding.

40. As an investor, I want a drift diagnostic to explain whether it comes from an Asset Class or an individual asset and to link to the affected holding or settings, so that the reason for the recommendation is clear.

41. As an investor, I want the floating action button on the overview to expose Add asset, Allocate, Rebalance, and Edit portfolio, so that the key next actions are available without a persistent bottom navigation bar.

42. As an investor, I want `Allocate` to open the existing allocation options flow, so that I can enter an investment budget and receive proposed buys based on the Portfolio's current targets.

43. As an investor, I want `Rebalance` to open the same focused allocation task flow with drift-aware recommendations, so that I can review the trades needed to move back inside my target bands.

44. As an investor, I want allocation options for tax-efficient or buy-only behavior, full-budget use, and fractional shares where the asset supports them, so that the recommendation fits my constraints.

45. As an investor, I want the allocation result to show each proposed buy or sell, current amount, new amount, current and new weight, target weight, commission, and fee impact, so that I can review the recommendation before updating the Portfolio.

46. As an investor, I want the allocation result to show unallocated cash when the budget cannot be fully used, so that I understand what remains and why.

47. As an investor, I want to return from the allocation result to the Portfolio overview, so that I can verify the updated holding values and drift after accepting the recommendation.

48. As an investor, I want allocation and rebalancing calculations to use the same absolute target weights regardless of whether those weights came from Simple or Strategic mode, so that the task flow does not need two different calculation models.

49. As an investor, I want optional Strategic allocation guidance to explain the selected framework and show suggested Asset Class weights, so that I can learn from a reference model without mistaking it for an automatic decision.

50. As an investor, I want The Bull rule to be the first available allocation framework and to show its assumptions, suggested defensive and risk-on split, and last-updated context, so that I can assess the suggestion before applying it.

51. As an investor, I want applying guidance to be an explicit action that previews the changes before saving, so that DcaPal never changes my targets silently.

52. As an investor, I want guidance to work in both Simple and Strategic mode by aggregating current asset weights into Asset Classes and comparing them with the suggested strategic allocation, so that the guidance remains useful without forcing a mode change.

53. As an investor, I want to import an existing Portfolio and see unresolved-price or invalid-data errors clearly, so that I can recover from incomplete imported data rather than accepting a misleading overview.

54. As a registered investor, I want saved Portfolio changes and price synchronisation to remain consistent when I return to the app, so that the overview reflects the latest stored configuration and market data.

55. As an investor, I want empty Portfolio, first-use, populated, loading, save-success, save-error, and registration-gated states, so that the user journey remains understandable before and after I have holdings.

56. As an investor using keyboard navigation or assistive technology, I want dialogs, menus, cards, forms, and the floating action button to expose clear names, focus order, escape behavior, and validation messages, so that the Portfolio can be managed without relying on a pointing device.

57. As an investor, I want the same controls and labels to work at mobile and desktop widths, with only density and component layout adapting, so that I do not have to learn two versions of DcaPal.

58. As an investor, I want the app to state that DcaPal provides information and allocation suggestions but does not execute trades, so that the allocation result is not mistaken for broker execution.

## Implementation Decisions

- Make the Portfolio overview the canonical hub and keep allocation and rebalancing as task flows reached from it.
- Use a responsive, mobile-first layout. Use cards for mobile holdings and a denser table for desktop. Do not add a persistent bottom navigation bar.
- Use one floating action button on the overview for Add asset, Allocate, Rebalance, and Edit portfolio.
- Use dialogs or drawers for add, edit, remove confirmation, fee details, and Portfolio settings. Preserve the overview context behind the dialog.
- Keep the Portfolio domain lean: a Portfolio contains its Portfolio Assets and their weights. Strategic allocation, guidance, drift configuration, and other auxiliary planning decisions are separate entities or tables.
- Support one active Strategic Allocation per Portfolio. To explore another plan, clone the Portfolio and modify the clone.
- Keep the hierarchy at Asset Class → Asset. Do not add tactical sleeves in this epic.
- In Simple mode, store and edit absolute target weights per Portfolio Asset. Derive Asset Class totals for summaries and guidance.
- In Strategic mode, store Asset Class target weights and relative Portfolio Asset weights within each class. Derive absolute asset target weights for allocation, drift, and display.
- Treat Bonds and Cash as defensive classes for planning guidance. Treat Equities, Crypto, Commodities, and Other as risk-on by default. Keep the distinction explicit as a planning classification, not a statement of actual risk.
- Represent drift bands in percentage points. Resolve the effective band from the individual asset override, the Asset Class setting, or the Portfolio default according to the product's precedence rule, and show that source when it matters.
- Use current asset inputs (quantity, price, quote currency, current value, average buy price, and fees) to calculate current weights and gain-related display values. Do not turn historical performance into a Portfolio property.
- Build model performance in the frontend from backend asset time series and the selected weights. Normalize the series to a common starting value and keep current Portfolio value as a separate metric.
- Support `1Y`, `3Y`, `5Y`, and `Max` model-history ranges. Put the range selectors below the chart as clickable text.
- Gate historical views for unauthenticated users and use registration as the call to action. Keep basic portfolio creation, editing, allocation, and rebalancing available before registration where the current product allows it.
- Reuse the current allocation concepts: investment budget, buy-only or tax-efficient behavior, full-budget preference, fractional-share support, transaction-fee policy, unallocated cash, and an explicit update step.
- Keep the current asset-search, price-provider, import, and synchronisation behavior. Extend their visible states to the new dialogs and overview rather than creating a second search model.
- Use the storyboard screen map as the journey contract. The 12/13 overview states are the main entry point; the earlier screens describe the current first-use and allocation branch that the new hub must continue to support.

## Testing Decisions

Use one primary Playwright end-to-end seam that starts from a populated Portfolio overview and crosses the real user-visible boundaries: open the floating action button, search and add an asset, edit its holding fields, open fee details, remove an asset, open Portfolio settings, switch allocation mode, launch Allocate or Rebalance, review the result, and return to the overview. Back this journey with the existing MSW fixture and provider seams so the test can control search results, prices, time series, fee rules, and calculation outcomes without depending on live market data.

The E2E journey should be exercised at a mobile viewport and a desktop viewport. It should assert mobile cards, desktop dense holdings, the absence of a persistent bottom navigation bar, dialog focus and dismissal, floating action menu destinations, and the same visible field contract in both layouts. It should cover populated, empty, first-use, loading, save-error, and registration-gated states.

Keep focused tests at the highest existing seam for each behavior:

- Use Playwright journey tests for the connected overview and allocation flows, including persisted visible state after save, remove, clone, and update.
- Use the existing service and MSW seams for asset search, price availability, import errors, portfolio synchronisation, and historical-series loading or failure.
- Use the existing compute and worker seams for absolute-weight derivation, Simple and Strategic formulas, current weight, drift, drift-band precedence, allocation budgets, fees, fractional-unit rules, unallocated cash, and normalized model-history calculations.
- Keep regression coverage for target totals under 100%, exactly 100%, and over 100%; rename, duplicate, and delete; zero, fixed, and variable fees; unresolved imported prices; search empty and error states; and unsupported fractional shares.
- Add assertions that guidance is optional, applying guidance is explicit, and no guidance or rebalance action silently edits stored targets.
- Add accessibility checks for dialog labels, form errors, keyboard focus, escape behavior, menu names, and chart alternatives or summaries.
- Treat the screen map and the four storyboard boards as review fixtures for the user journey. The boards are visual references; automated tests should assert the underlying behavior and accessible labels rather than pixel-match generated images.

## Out of Scope

- Broker integrations, order execution, trade settlement, or automatic transactions.
- A transaction ledger or money-weighted performance based on investment dates and cash flows.
- Multiple alternative Strategic Allocations attached to one Portfolio. Use Portfolio cloning for experimentation.
- Tactical sleeves or a third allocation hierarchy below Asset Class → Asset.
- Automatic application of strategic guidance, drift changes, or rebalancing recommendations.
- Treating Bonds or Cash as literally risk-free investments.
- Merton's rule or additional literature-based frameworks beyond the first The Bull rule integration.
- Automated scheduled rebalancing, notifications, or broker-connected alerts.
- Portfolio sharing, collaboration, or multi-user permissions.
- A separate desktop product line or a persistent bottom navigation system.
- Anonymous access to the heavy historical-analysis view.
- Rebuilding the existing allocation calculator from scratch when it can be reached and extended from the new overview.

## Further Notes

The first Strategic allocation framework is The Bull rule, based on the references supplied for this product direction: [The Bull](https://www.thebull.it/podcast/asset-allocation-la-regola-di-the-bull/), [Finanza Cafona](https://finanzacafona.it/2021/03/scegliere-asset-allocation-the-bull.html), and [Ivalio](https://ivalio.it/calcolatore-regola-di-the-bull/). The framework is a reference and educational aid. The user remains responsible for deciding whether to apply it.

The current live journey and `origin/master` journey specifications remain important compatibility references. The screen map records the links from first use through the existing allocation wizard and then into the new overview hub. It also records validation states that may not fit in the primary boards.

### Design references

The four latest generated storyboard boards are included with this epic:

- [Add asset and search flow](assets/add-asset-flow.png)
- [Edit and remove asset flow](assets/edit-remove-asset-flow.png)
- [Portfolio settings and strategic allocation flow](assets/portfolio-settings-flow.png)
- [Allocate and rebalance flow](assets/allocate-rebalance-flow.png)

The complete linked journey is in the [allocate journey screen map](allocate-journey-screen-map.md).
