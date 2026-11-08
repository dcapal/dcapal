import type { AssetClass, PortfolioAsset } from "../../fixtures/portfolio";
import { formatPoints } from "./invest-format";
import {
  checkTargetTotals,
  TARGET_TOTAL_TOLERANCE_PP,
  type TargetTotalsCheck,
} from "./invest-targets";
import { assetClassOrder } from "./portfolio-asset-groups";

/** Asset-class target weights across every asset class, in percent of the Portfolio. */
export type ClassWeights = Record<AssetClass, number>;

/** Relative target weights keyed by Portfolio Asset id, in percent of the asset's class. */
export type AssetShares = Record<string, number>;

/**
 * The two levels of a Strategic allocation: an Asset-class target weight per
 * class and a Relative target weight per asset inside its class. Absolute
 * target weights are derived from them and never stored.
 */
export interface StrategicAllocation {
  classTargets: ClassWeights;
  shares: AssetShares;
}

/** Whether a weight total is complete, short of 100% or over it. */
export type WeightTotalState = "balanced" | "short" | "over";

/** A weight total described for display: the figure, its gap to 100% and a label. */
export interface WeightTotal {
  total: number;
  /** Distance from 100%, in percentage points; never negative. */
  gap: number;
  state: WeightTotalState;
  /** Short status text, such as "Balanced" or "Short by 5.0 pp". */
  label: string;
}

/** All asset classes at zero. */
export function emptyClassWeights(): ClassWeights {
  return Object.fromEntries(
    assetClassOrder.map((assetClass) => [assetClass, 0])
  ) as ClassWeights;
}

/** Rounds to one decimal, the precision weights are shown and entered with. */
export function roundToTenth(value: number): number {
  return Math.round(value * 10) / 10;
}

/** Message shown beside a weight field whose text is not a number from 0 to 100. */
export const WEIGHT_INPUT_ERROR = "Enter a number from 0 to 100";

/** A weight read from typed text: the usable value and whether the text was acceptable. */
export interface WeightInput {
  /** The typed number, or 0 when the text is empty or invalid so totals never see NaN. */
  value: number;
  /** Set when the text is non-numeric or outside 0 to 100; the value then counts as 0. */
  error: string | null;
}

/**
 * Reads a weight typed into a percentage field. Empty text counts as 0
 * without an error. Non-numeric, negative or above-100 text is never clamped:
 * it is reported through `error` and contributes 0 to every total.
 */
export function readWeightInput(raw: string): WeightInput {
  const text = raw.trim().replace(",", ".");
  if (text === "") return { value: 0, error: null };
  const value = Number(text);
  if (!Number.isFinite(value) || value < 0 || value > 100) {
    return { value: 0, error: WEIGHT_INPUT_ERROR };
  }
  return { value, error: null };
}

/** Sums weights, ignoring non-finite entries. */
export function sumWeights(values: Iterable<number>): number {
  let total = 0;
  for (const value of values) if (Number.isFinite(value)) total += value;
  return total;
}

/**
 * Describes how a total relates to 100%. A total within rounding distance of
 * 100% is balanced.
 */
export function describeWeightTotal(total: number): WeightTotal {
  const gap = Math.abs(100 - total);
  if (gap <= TARGET_TOTAL_TOLERANCE_PP) {
    return { total, gap: 0, state: "balanced", label: "Balanced" };
  }
  return total > 100
    ? { total, gap, state: "over", label: `Over by ${formatPoints(gap)}` }
    : { total, gap, state: "short", label: `Short by ${formatPoints(gap)}` };
}

/** Sums one numeric field of the assets of every class. */
function sumByClass(
  assets: readonly PortfolioAsset[],
  read: (asset: PortfolioAsset) => number
): ClassWeights {
  const sums = emptyClassWeights();
  for (const asset of assets) sums[asset.assetClass] += read(asset);
  return sums;
}

/**
 * The asset-class target weights implied by the current absolute target
 * weights: each class target is the sum of its assets' targets.
 */
export function classTargetsFromAssets(
  assets: readonly PortfolioAsset[]
): ClassWeights {
  const sums = sumByClass(assets, (asset) => asset.targetWeight);
  for (const assetClass of assetClassOrder) {
    sums[assetClass] = roundToTenth(sums[assetClass]);
  }
  return sums;
}

/**
 * The asset-class weights the Portfolio holds today, rounded to one decimal.
 * When the rounded weights miss 100% by half a point or less, the difference
 * goes to the largest class so the starter is a complete allocation.
 */
export function currentMixFromAssets(
  assets: readonly PortfolioAsset[]
): ClassWeights {
  const sums = sumByClass(assets, (asset) => asset.currentWeight);
  for (const assetClass of assetClassOrder) {
    sums[assetClass] = roundToTenth(sums[assetClass]);
  }
  const total = sumWeights(Object.values(sums));
  const difference = roundToTenth(100 - total);
  if (difference !== 0 && Math.abs(difference) <= 0.5) {
    const largest = assetClassOrder.reduce((best, assetClass) =>
      sums[assetClass] > sums[best] ? assetClass : best
    );
    sums[largest] = roundToTenth(sums[largest] + difference);
  }
  return sums;
}

/**
 * Each asset's share of its class, from the absolute target weights. A class
 * whose targets are all zero splits evenly so its assets keep a usable share.
 */
export function sharesFromAssets(
  assets: readonly PortfolioAsset[]
): AssetShares {
  const classTotals = sumByClass(assets, (asset) => asset.targetWeight);
  const classCounts = sumByClass(assets, () => 1);
  const shares: AssetShares = {};
  for (const asset of assets) {
    const classTotal = classTotals[asset.assetClass];
    shares[asset.id] =
      classTotal > 0
        ? (asset.targetWeight / classTotal) * 100
        : 100 / classCounts[asset.assetClass];
  }
  return shares;
}

/** The Strategic allocation equivalent to the current absolute target weights. */
export function strategicFromAssets(
  assets: readonly PortfolioAsset[]
): StrategicAllocation {
  return {
    classTargets: classTargetsFromAssets(assets),
    shares: sharesFromAssets(assets),
  };
}

/**
 * The Strategic allocation a Portfolio holds right after switching from Simple.
 *
 * Rule: Simple edits are never lost. The class targets are the ones the
 * investor chose when switching. The shares are the stored ones while no
 * absolute weight was edited since the Strategic allocation was last in
 * force; once the investor has edited an absolute weight in Simple mode, the
 * shares are re-derived from the current absolute weights (absolute weight
 * divided by the sum of its class), so every asset's absolute weight is
 * preserved when its class keeps the sum it had.
 */
export function allocationOnSwitchToStrategic(
  baseAssets: readonly PortfolioAsset[],
  absolute: Readonly<Record<string, number>>,
  stored: StrategicAllocation,
  classTargets: ClassWeights,
  simpleEdited: boolean
): StrategicAllocation {
  return {
    classTargets,
    shares: sharesOnSwitchToStrategic(
      baseAssets,
      absolute,
      stored,
      simpleEdited
    ),
  };
}

/** The shares {@link allocationOnSwitchToStrategic} will store, for validating before the switch. */
export function sharesOnSwitchToStrategic(
  baseAssets: readonly PortfolioAsset[],
  absolute: Readonly<Record<string, number>>,
  stored: StrategicAllocation,
  simpleEdited: boolean
): AssetShares {
  return simpleEdited
    ? sharesFromAssets(withTargetWeights(baseAssets, absolute))
    : stored.shares;
}

/** Absolute target weight of each asset: class target times share of the class. */
export function deriveAbsoluteWeights(
  assets: readonly PortfolioAsset[],
  allocation: StrategicAllocation
): Record<string, number> {
  return Object.fromEntries(
    assets.map((asset) => [
      asset.id,
      (allocation.classTargets[asset.assetClass] *
        (allocation.shares[asset.id] ?? 0)) /
        100,
    ])
  );
}

/** Returns the assets with new absolute target weights and the drift they imply. */
export function withTargetWeights(
  assets: readonly PortfolioAsset[],
  weights: Readonly<Record<string, number>>
): PortfolioAsset[] {
  return assets.map((asset) => {
    const targetWeight = weights[asset.id] ?? asset.targetWeight;
    return {
      ...asset,
      targetWeight,
      driftPp: asset.currentWeight - targetWeight,
    };
  });
}

/** Sets one asset-class target weight without touching the shares inside classes. */
export function withClassTarget(
  allocation: StrategicAllocation,
  assetClass: AssetClass,
  weight: number
): StrategicAllocation {
  return {
    ...allocation,
    classTargets: { ...allocation.classTargets, [assetClass]: weight },
  };
}

/** Sets one asset's share of its class without touching any other asset. */
export function withAssetShare(
  allocation: StrategicAllocation,
  assetId: string,
  share: number
): StrategicAllocation {
  return {
    ...allocation,
    shares: { ...allocation.shares, [assetId]: share },
  };
}

/**
 * Checks every total that gates Invest in Strategic mode: the class targets
 * across all classes, the shares inside each class, and weight placed on a
 * class that holds no assets.
 */
export function checkStrategicTotals(
  assets: readonly PortfolioAsset[],
  allocation: StrategicAllocation
): TargetTotalsCheck {
  return checkTargetTotals({
    mode: "strategic",
    classTargets: allocation.classTargets,
    assets: assets.map((asset) => ({
      id: asset.id,
      assetClass: asset.assetClass,
      weight: allocation.shares[asset.id] ?? 0,
    })),
  });
}

/** One sibling in the share preview of the asset editor. */
export interface ShareRow {
  assetId: string;
  ticker: string;
  /** Share before the edit, in percent of the class. */
  before: number;
  /** Share after the edit, in percent of the class. */
  after: number;
  /** Resulting weight in the Portfolio: class weight times the new share. */
  absolute: number;
  edited: boolean;
}

/** The shares of one class after an edit, with the total they reach. */
export interface ClassSharesPreview {
  rows: ShareRow[];
  total: WeightTotal;
}

/**
 * Previews changing one asset's share of its class: every sibling keeps its
 * share, the edited asset takes the new one, and nothing is redistributed. The
 * total shows whether the class still adds up to 100%.
 */
export function previewShareChange(
  assets: readonly PortfolioAsset[],
  allocation: StrategicAllocation,
  assetId: string,
  newShare: number
): ClassSharesPreview {
  const edited = assets.find((asset) => asset.id === assetId);
  if (!edited) return { rows: [], total: describeWeightTotal(0) };
  const classTarget = allocation.classTargets[edited.assetClass];
  const rows = assets
    .filter((asset) => asset.assetClass === edited.assetClass)
    .map((asset) => {
      const before = allocation.shares[asset.id] ?? 0;
      const after = asset.id === assetId ? newShare : before;
      return {
        assetId: asset.id,
        ticker: asset.ticker,
        before,
        after,
        absolute: (classTarget * after) / 100,
        edited: asset.id === assetId,
      };
    });
  return {
    rows,
    total: describeWeightTotal(sumWeights(rows.map((row) => row.after))),
  };
}

/** Class targets after the investor changes one class, for the Edit class preview. */
export function previewClassChange(
  allocation: StrategicAllocation,
  assetClass: AssetClass,
  newWeight: number
): { classTargets: ClassWeights; total: WeightTotal } {
  const classTargets = { ...allocation.classTargets, [assetClass]: newWeight };
  return {
    classTargets,
    total: describeWeightTotal(sumWeights(Object.values(classTargets))),
  };
}

/** Asset classes that hold at least one of the assets, in canonical order. */
export function populatedClasses(
  assets: readonly PortfolioAsset[]
): AssetClass[] {
  return assetClassOrder.filter((assetClass) =>
    assets.some((asset) => asset.assetClass === assetClass)
  );
}
