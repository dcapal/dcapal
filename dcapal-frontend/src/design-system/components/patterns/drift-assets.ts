import * as React from "react";
import type { PortfolioAsset } from "../../fixtures/portfolio";
import { largestGap, totalDriftPp } from "./invest-plan";
import type { DriftRowModel, DriftSnapshot } from "./invest-types";
import {
  buildPortfolioAssetSummary,
  type PortfolioAssetClassGroup,
  type StrategicTargets,
} from "./portfolio-asset-groups";

/**
 * Turns the rows of a drift snapshot into the Portfolio Assets the shared
 * asset collection renders. The snapshot carries weights, not holdings, so the
 * value of each asset is its current weight of the Portfolio value; quantity
 * and price are unknown (zero) and only shown by the detailed presentation,
 * which the drift view never uses.
 */
export function driftRowsToPortfolioAssets(
  rows: readonly DriftRowModel[],
  portfolioValue: number,
  currency: string
): PortfolioAsset[] {
  return rows.map((row) => ({
    id: row.id,
    name: row.name,
    ticker: row.ticker,
    assetClass: row.assetClass,
    quantity: 0,
    price: 0,
    value: (portfolioValue * row.currentWeight) / 100,
    targetWeight: row.targetWeight,
    currentWeight: row.currentWeight,
    driftPp: row.driftPp,
    currency,
  }));
}

/** The Portfolio Assets of a drift snapshot, recomputed only when it changes. */
export function useDriftAssets(
  snapshot: Pick<DriftSnapshot, "rows" | "portfolioValue" | "currency">
): PortfolioAsset[] {
  const { rows, portfolioValue, currency } = snapshot;
  return React.useMemo(
    () => driftRowsToPortfolioAssets(rows, portfolioValue, currency),
    [rows, portfolioValue, currency]
  );
}

/**
 * The asset classes of a drift view, widest gap first, from the same summary
 * the asset collection renders, so headers and rows always agree.
 */
export function driftClassGroups(
  assets: readonly PortfolioAsset[],
  targets?: StrategicTargets
): PortfolioAssetClassGroup[] {
  return [...buildPortfolioAssetSummary(assets, targets).groups].sort(
    (a, b) => Math.abs(b.driftPp) - Math.abs(a.driftPp)
  );
}

/** Total drift measured on class weights, in percentage points. */
export function classTotalDriftPp(
  groups: readonly PortfolioAssetClassGroup[]
): number {
  return totalDriftPp(groups);
}

/** The class with the widest gap from its class target, or null when there are none. */
export function classLargestGap(
  groups: readonly PortfolioAssetClassGroup[]
): PortfolioAssetClassGroup | null {
  return largestGap(groups);
}
