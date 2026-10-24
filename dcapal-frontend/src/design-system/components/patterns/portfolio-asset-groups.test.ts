import { describe, expect, it } from "vitest";
import type { PortfolioAsset } from "../../fixtures/portfolio";
import {
  buildPortfolioAssetSummary,
  defaultPortfolioAssetSort,
  driftState,
  nextPortfolioAssetSort,
  normalizePortfolioAssetSort,
  sortAssetGroups,
} from "./portfolio-asset-groups";

/** Builds a minimal asset whose stored weights are passed explicitly. */
function asset(
  id: string,
  assetClass: PortfolioAsset["assetClass"],
  value: number,
  targetWeight: number,
  currentWeight: number,
  extra: Partial<PortfolioAsset> = {}
): PortfolioAsset {
  return {
    id,
    name: id.toUpperCase(),
    ticker: id.toUpperCase(),
    assetClass,
    quantity: 0,
    price: 1,
    value,
    targetWeight,
    currentWeight,
    driftPp: Math.round((currentWeight - targetWeight) * 10) / 10,
    ...extra,
  };
}

const assets = [
  asset("a", "equities", 300, 40, 30, { quantity: 10, averageCostBasis: 20 }),
  asset("b", "equities", 100, 20, 12, { quantity: 4, averageCostBasis: 10 }),
  asset("c", "bonds", 420, 30, 41),
  asset("d", "cash", 180, 10, 17),
];

describe("buildPortfolioAssetSummary", () => {
  /**
   * GIVEN four assets whose stored weights differ from their value shares
   * WHEN the summary is built
   * THEN asset rows keep the stored weight and drift, and class rows sum them.
   */
  it("aggregates classes from stored weights", () => {
    const summary = buildPortfolioAssetSummary(assets);
    expect(summary.totalValue).toBe(1000);
    expect(summary.groups.map((g) => g.assetClass)).toEqual([
      "equities",
      "bonds",
      "cash",
    ]);
    const [a] = summary.entries;
    expect(a.currentWeight).toBe(30);
    expect(a.driftPp).toBe(-10);
    const equities = summary.groups[0];
    expect(equities.value).toBe(400);
    expect(equities.currentWeight).toBe(42);
    expect(equities.targetWeight).toBe(60);
    expect(equities.driftPp).toBe(-18);
    expect(equities.invested).toBe(240);
    expect(summary.totalInvested).toBe(240);
    expect(summary.totalCurrentWeight).toBe(100);
  });

  /**
   * GIVEN a single-asset collection with stored weight 34.8 and drift 4.8
   * WHEN the summary is built
   * THEN the asset keeps 34.8 / 4.8 instead of becoming 100% of the portfolio.
   */
  it("keeps stored weights for a single asset", () => {
    const [entry] = buildPortfolioAssetSummary([
      asset("v", "equities", 3522.77, 30, 34.8),
    ]).entries;
    expect(entry.currentWeight).toBe(34.8);
    expect(entry.driftPp).toBe(4.8);
  });

  /**
   * GIVEN assets inside the Equities class
   * WHEN the summary is built
   * THEN shares of class use stored weights over the class sum.
   */
  it("computes shares of class and relative targets", () => {
    const [a, b] = buildPortfolioAssetSummary(assets).groups[0].entries;
    expect(a.currentShareOfClass).toBeCloseTo((30 / 42) * 100, 5);
    expect(b.currentShareOfClass).toBeCloseTo((12 / 42) * 100, 5);
    expect(a.targetShareOfClass).toBeCloseTo(66.67, 1);
  });

  /**
   * GIVEN a class with no cost basis and an empty portfolio
   * WHEN the summary is built
   * THEN invested is null and nothing is NaN.
   */
  it("handles missing cost and empty portfolios", () => {
    const summary = buildPortfolioAssetSummary(assets);
    expect(summary.groups[2].invested).toBeNull();
    const empty = buildPortfolioAssetSummary([]);
    expect(empty.groups).toEqual([]);
    expect(empty.totalInvested).toBeNull();
    const zero = buildPortfolioAssetSummary([asset("z", "cash", 0, 0, 0)]);
    expect(zero.groups[0].currentWeight).toBe(0);
    expect(zero.entries[0].targetShareOfClass).toBe(0);
  });
});

describe("sorting", () => {
  /**
   * GIVEN the default sort of target weight descending
   * WHEN classes are sorted
   * THEN classes order by aggregate target and assets by their own target.
   */
  it("orders classes by aggregate and assets within each class", () => {
    const sorted = sortAssetGroups(
      buildPortfolioAssetSummary(assets).groups,
      defaultPortfolioAssetSort
    );
    expect(sorted.map((g) => g.assetClass)).toEqual([
      "equities",
      "bonds",
      "cash",
    ]);
    expect(sorted[0].entries.map((e) => e.asset.id)).toEqual(["a", "b"]);
    const byValue = sortAssetGroups(buildPortfolioAssetSummary(assets).groups, {
      key: "value",
      direction: "asc",
    });
    expect(byValue.map((g) => g.assetClass)).toEqual([
      "cash",
      "equities",
      "bonds",
    ]);
  });

  /**
   * GIVEN a sort on one column
   * WHEN the same or another column is activated
   * THEN the same column flips and a new column picks its natural direction.
   */
  it("picks the next sort", () => {
    expect(nextPortfolioAssetSort(defaultPortfolioAssetSort, "target")).toEqual(
      { key: "target", direction: "asc" }
    );
    expect(nextPortfolioAssetSort(defaultPortfolioAssetSort, "name")).toEqual({
      key: "name",
      direction: "asc",
    });
  });
});

describe("normalizePortfolioAssetSort", () => {
  /**
   * GIVEN a class ordering carried over from simple mode
   * WHEN it is normalised for strategic mode
   * THEN it falls back to target descending, while other keys are kept.
   */
  it("drops the class key in strategic mode", () => {
    const byClass = { key: "class", direction: "asc" } as const;
    expect(normalizePortfolioAssetSort(byClass, "strategic")).toEqual(
      defaultPortfolioAssetSort
    );
    expect(normalizePortfolioAssetSort(byClass, "simple")).toEqual(byClass);
  });
});

describe("driftState", () => {
  /**
   * GIVEN drifts around the default 5 pp Portfolio-wide band
   * WHEN classified
   * THEN up to and including 5 pp is on target, so +4.8 pp is on target.
   */
  it("treats |drift| <= 5 pp as on target by default", () => {
    expect(driftState(4.8)).toBe("on-target");
    expect(driftState(5)).toBe("on-target");
    expect(driftState(-5)).toBe("on-target");
    expect(driftState(5.1)).toBe("over");
    expect(driftState(-5.1)).toBe("under");
  });

  /**
   * GIVEN a narrower 2 pp band
   * WHEN +4.8 pp is classified
   * THEN it is over target.
   */
  it("honours a custom band", () => {
    expect(driftState(4.8, 2)).toBe("over");
    expect(driftState(-2, 2)).toBe("on-target");
  });
});
