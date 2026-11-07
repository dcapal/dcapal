import { describe, expect, it } from "vitest";
import {
  classLargestGap,
  classTotalDriftPp,
  driftClassGroups,
  driftRowsToPortfolioAssets,
} from "./drift-assets";
import type { DriftRowModel } from "./invest-types";
import { investDrift } from "../../fixtures/invest";

const row = (
  id: string,
  assetClass: DriftRowModel["assetClass"],
  current: number,
  target: number
): DriftRowModel => ({
  id,
  name: `${id} name`,
  ticker: id,
  assetClass,
  currentWeight: current,
  targetWeight: target,
  driftPp: current - target,
});

describe("driftRowsToPortfolioAssets", () => {
  /**
   * GIVEN drift rows and a Portfolio value
   * WHEN they are mapped to Portfolio Assets for the shared collection
   * THEN identity, weights and drift are kept and each value is its current
   * weight of the Portfolio value.
   */
  it("keeps weights and derives each value from the Portfolio value", () => {
    const [asset] = driftRowsToPortfolioAssets(
      [row("VWCE", "equities", 60, 50)],
      10_000,
      "EUR"
    );
    expect(asset).toMatchObject({
      id: "VWCE",
      name: "VWCE name",
      ticker: "VWCE",
      assetClass: "equities",
      currentWeight: 60,
      targetWeight: 50,
      driftPp: 10,
      value: 6_000,
      currency: "EUR",
    });
  });

  /**
   * GIVEN the Invest drift fixture
   * WHEN class groups are derived from the shared asset summary
   * THEN classes are ordered by widest gap and each class's weights are the
   * sums of its assets', so headers and the collection agree.
   */
  it("derives class groups from the shared summary, widest gap first", () => {
    const groups = driftClassGroups(
      driftRowsToPortfolioAssets(
        investDrift.rows,
        investDrift.portfolioValue,
        investDrift.currency
      )
    );
    const gaps = groups.map((group) => Math.abs(group.driftPp));
    expect(gaps).toEqual([...gaps].sort((a, b) => b - a));
    for (const group of groups) {
      const rows = investDrift.rows.filter(
        (row) => row.assetClass === group.assetClass
      );
      expect(group.currentWeight).toBeCloseTo(
        rows.reduce((sum, row) => sum + row.currentWeight, 0),
        6
      );
      expect(group.targetWeight).toBeCloseTo(
        rows.reduce((sum, row) => sum + row.targetWeight, 0),
        6
      );
    }
  });

  /**
   * GIVEN five rows in three classes and a stated Strategic allocation with an
   * equities target of 60
   * WHEN class figures are measured
   * THEN the stated target wins for its class, and total drift and the
   * largest gap are measured on class weights.
   */
  it("prefers stated class targets and measures drift on classes", () => {
    const assets = driftRowsToPortfolioAssets(
      [
        row("vwce", "equities", 32, 30),
        row("ieur", "equities", 14, 20),
        row("vig", "equities", 16, 15),
        row("aggh", "bonds", 2, 25),
        row("cash", "cash", 36, 10),
      ],
      1_000,
      "EUR"
    );
    const derived = driftClassGroups(assets);
    expect(classTotalDriftPp(derived)).toBeCloseTo(26, 5);
    expect(classLargestGap(derived)?.assetClass).toBe("cash");
    expect(classLargestGap([])).toBeNull();

    const stated = driftClassGroups(assets, {
      classTargets: { equities: 60 },
      shares: {},
    });
    const equities = stated.find((group) => group.assetClass === "equities");
    expect(equities?.targetWeight).toBe(60);
    expect(equities?.driftPp).toBe(2);
  });
});
