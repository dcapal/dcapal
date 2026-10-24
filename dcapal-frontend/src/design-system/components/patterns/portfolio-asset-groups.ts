import {
  assetClassLabels,
  type AssetClass,
  type PortfolioAsset,
} from "../../fixtures/portfolio";

/**
 * Default Portfolio-wide Drift band: the distance from target, in percentage
 * points, that still counts as on target. The asset table, cards, class
 * groups, badges and the Invest drift overview all classify with this one band.
 */
export const DEFAULT_DRIFT_BAND_PP = 5;

/** Whether a drift is over, under or within the on-target band. */
export type DriftState = "over" | "under" | "on-target";

/** Columns a portfolio asset table or card list can be ordered by. */
export type PortfolioAssetSortKey =
  "name" | "class" | "value" | "current" | "target" | "drift";

export type SortDirection = "asc" | "desc";

/** The active ordering of a portfolio asset view. */
export interface PortfolioAssetSort {
  key: PortfolioAssetSortKey;
  direction: SortDirection;
}

/** Ordering applied before the investor chooses one: largest target first. */
export const defaultPortfolioAssetSort: PortfolioAssetSort = {
  key: "target",
  direction: "desc",
};

/** Canonical order used to list asset classes before any sorting is applied. */
export const assetClassOrder = Object.keys(assetClassLabels) as AssetClass[];

/** One asset together with the weights stored on it. */
export interface PortfolioAssetEntry {
  asset: PortfolioAsset;
  /** Stored share of the whole portfolio value, in percent. */
  currentWeight: number;
  /** Target share of the whole portfolio, in percent (absolute target weight). */
  targetWeight: number;
  /** Current weight minus target weight, in percentage points. */
  driftPp: number;
  /** Stored current weight over its class's current weight, in percent. */
  currentShareOfClass: number;
  /** Target share within its asset class (relative target weight), in percent. */
  targetShareOfClass: number;
}

/** Aggregates of every asset sharing one asset class. */
export interface PortfolioAssetClassGroup {
  assetClass: AssetClass;
  label: string;
  entries: PortfolioAssetEntry[];
  /** Currency used to format the class monetary totals. */
  currency: string;
  value: number;
  /** Sum of quantity times average cost basis, or `null` when nothing was bought. */
  invested: number | null;
  currentWeight: number;
  /** Asset-class target weight: the sum of its assets' absolute targets. */
  targetWeight: number;
  driftPp: number;
}

/** The presentation model shared by the table and the mobile cards. */
export interface PortfolioAssetSummary {
  entries: PortfolioAssetEntry[];
  groups: PortfolioAssetClassGroup[];
  currency: string;
  totalValue: number;
  totalInvested: number | null;
  /** Sum of the stored current weights of every asset. */
  totalCurrentWeight: number;
  totalTargetWeight: number;
}

/**
 * The stated Strategic allocation behind a summary: asset-class targets and
 * each asset's relative target weight. Without it both are derived from the
 * assets' absolute targets.
 */
export interface StrategicTargets {
  classTargets: Partial<Record<AssetClass, number>>;
  shares: Readonly<Record<string, number>>;
}

/** Divides, returning zero instead of NaN or Infinity for an empty denominator. */
export function ratioPercent(part: number, whole: number): number {
  return whole > 0 ? (part / whole) * 100 : 0;
}

/** Shows 100 when stored weights only miss it by rounding (within 0.5 pp). */
function snapToWhole(total: number): number {
  return Math.abs(total - 100) <= 0.5 ? 100 : total;
}

/** Cost invested in one asset; zero when it has no quantity or cost basis. */
function investedIn(asset: PortfolioAsset): number {
  return asset.averageCostBasis === undefined
    ? 0
    : asset.quantity * asset.averageCostBasis;
}

/** Classifies a drift using the Portfolio-wide Drift band (inclusive at the edge). */
export function driftState(
  driftPp: number,
  bandPp: number = DEFAULT_DRIFT_BAND_PP
): DriftState {
  if (Math.abs(driftPp) <= bandPp) return "on-target";
  return driftPp > 0 ? "over" : "under";
}

/** Plain-language status that accompanies a drift value so colour is never alone. */
export const driftStateLabels: Record<DriftState, string> = {
  over: "Over target",
  under: "Under target",
  "on-target": "On target",
};

/** Muted place or denomination shown beside an asset class. */
export function assetRegion(
  asset: Pick<PortfolioAsset, "assetClass" | "currency" | "region">
): string {
  return asset.assetClass === "cash"
    ? (asset.currency ?? "EUR")
    : (asset.region ?? "Global");
}

/**
 * Builds per-asset entries and per-class aggregates. Asset weights and drift
 * are the stored `currentWeight` / `driftPp` so every screen agrees; class
 * weights are the sums of their members' stored weights. Monetary class
 * totals come from value and cost basis. Classes appear in canonical order
 * and only when they hold at least one asset.
 */
export function buildPortfolioAssetSummary(
  assets: readonly PortfolioAsset[],
  targets?: StrategicTargets
): PortfolioAssetSummary {
  const totalValue = assets.reduce((sum, asset) => sum + asset.value, 0);
  const currency = assets[0]?.currency ?? "EUR";

  const groups: PortfolioAssetClassGroup[] = [];
  for (const assetClass of assetClassOrder) {
    const members = assets.filter((asset) => asset.assetClass === assetClass);
    if (members.length === 0) continue;

    const value = members.reduce((sum, asset) => sum + asset.value, 0);
    // A Strategic allocation states the class target itself; otherwise the
    // class target is the sum of its assets' absolute targets.
    const targetWeight =
      targets?.classTargets[assetClass] ??
      members.reduce((sum, asset) => sum + asset.targetWeight, 0);
    const currentWeight = members.reduce(
      (sum, asset) => sum + asset.currentWeight,
      0
    );
    const invested = members.reduce((sum, asset) => sum + investedIn(asset), 0);

    groups.push({
      assetClass,
      label: assetClassLabels[assetClass],
      currency: members[0].currency ?? currency,
      value,
      invested: invested > 0 ? invested : null,
      currentWeight,
      targetWeight,
      driftPp: currentWeight - targetWeight,
      entries: members.map((asset) => ({
        asset,
        currentWeight: asset.currentWeight,
        targetWeight: asset.targetWeight,
        driftPp: asset.driftPp,
        currentShareOfClass: ratioPercent(asset.currentWeight, currentWeight),
        targetShareOfClass:
          targets?.shares[asset.id] ??
          ratioPercent(asset.targetWeight, targetWeight),
      })),
    });
  }

  const totalInvested = groups.reduce(
    (sum, group) => sum + (group.invested ?? 0),
    0
  );

  return {
    entries: groups.flatMap((group) => group.entries),
    groups,
    currency,
    totalValue,
    totalInvested: totalInvested > 0 ? totalInvested : null,
    totalCurrentWeight: snapToWhole(
      groups.reduce((sum, group) => sum + group.currentWeight, 0)
    ),
    totalTargetWeight: groups.reduce(
      (sum, group) => sum + group.targetWeight,
      0
    ),
  };
}

/** Sort value of an asset or class for one column. */
function sortValue(
  key: PortfolioAssetSortKey,
  subject:
    | { kind: "asset"; entry: PortfolioAssetEntry }
    | { kind: "group"; group: PortfolioAssetClassGroup }
): string | number {
  if (subject.kind === "asset") {
    const { entry } = subject;
    switch (key) {
      case "name":
        return entry.asset.name;
      case "class":
        return assetClassLabels[entry.asset.assetClass];
      case "value":
        return entry.asset.value;
      case "current":
        return entry.currentWeight;
      case "target":
        return entry.targetWeight;
      case "drift":
        return entry.driftPp;
    }
  }
  const { group } = subject;
  switch (key) {
    case "name":
    case "class":
      return group.label;
    case "value":
      return group.value;
    case "current":
      return group.currentWeight;
    case "target":
      return group.targetWeight;
    case "drift":
      return group.driftPp;
  }
}

/** Compares two sort values; text sorts alphabetically, numbers numerically. */
function compareValues(a: string | number, b: string | number): number {
  return typeof a === "string" && typeof b === "string"
    ? a.localeCompare(b)
    : Number(a) - Number(b);
}

/** Orders a flat list of asset entries without mutating the input. */
export function sortAssetEntries(
  entries: readonly PortfolioAssetEntry[],
  sort: PortfolioAssetSort
): PortfolioAssetEntry[] {
  const sign = sort.direction === "asc" ? 1 : -1;
  return [...entries].sort(
    (a, b) =>
      sign *
      compareValues(
        sortValue(sort.key, { kind: "asset", entry: a }),
        sortValue(sort.key, { kind: "asset", entry: b })
      )
  );
}

/**
 * Orders classes by their aggregate for the sort key and the assets inside
 * every class by their own value, without mutating the input.
 */
export function sortAssetGroups(
  groups: readonly PortfolioAssetClassGroup[],
  sort: PortfolioAssetSort
): PortfolioAssetClassGroup[] {
  const sign = sort.direction === "asc" ? 1 : -1;
  return [...groups]
    .sort(
      (a, b) =>
        sign *
        compareValues(
          sortValue(sort.key, { kind: "group", group: a }),
          sortValue(sort.key, { kind: "group", group: b })
        )
    )
    .map((group) => ({
      ...group,
      entries: sortAssetEntries(group.entries, sort),
    }));
}

/**
 * Ordering that is valid for a mode: the class column only exists in simple
 * mode, so strategic mode falls back to the default ordering.
 */
export function normalizePortfolioAssetSort(
  sort: PortfolioAssetSort,
  mode: "simple" | "strategic"
): PortfolioAssetSort {
  return mode === "strategic" && sort.key === "class"
    ? defaultPortfolioAssetSort
    : sort;
}

/**
 * Next ordering after the investor activates a column header: the same column
 * flips direction, a new column starts with names ascending and numbers
 * descending.
 */
export function nextPortfolioAssetSort(
  current: PortfolioAssetSort,
  key: PortfolioAssetSortKey
): PortfolioAssetSort {
  if (current.key === key) {
    return { key, direction: current.direction === "asc" ? "desc" : "asc" };
  }
  return {
    key,
    direction: key === "name" || key === "class" ? "asc" : "desc",
  };
}
