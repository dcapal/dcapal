import { describe, expect, it } from "vitest";
import {
  checkTargetTotals,
  describeTargetIssues,
  targetTotalsMessage,
  type TargetDefinition,
} from "./invest-targets";

const simple = (weights: number[]): TargetDefinition => ({
  mode: "simple",
  assets: weights.map((weight, i) => ({
    id: `a${i}`,
    assetClass: "equities",
    weight,
  })),
});

describe("checkTargetTotals", () => {
  /**
   * GIVEN Simple mode absolute weights that total 100% (within rounding)
   * WHEN the totals are checked
   * THEN allocating is allowed and no issue is reported.
   */
  it("accepts simple weights that total 100%", () => {
    expect(checkTargetTotals(simple([60, 40])).ok).toBe(true);
    expect(checkTargetTotals(simple([33.33, 33.33, 33.34])).ok).toBe(true);
    expect(
      describeTargetIssues(checkTargetTotals(simple([60, 40])))
    ).toBeNull();
  });

  /**
   * GIVEN Simple mode weights that total 95%
   * WHEN the totals are checked
   * THEN the check fails and the description names the actual total.
   */
  it("rejects simple weights that miss 100%", () => {
    const check = checkTargetTotals(simple([60, 35]));
    expect(check.ok).toBe(false);
    expect(check.issues).toEqual([{ scope: "asset-weights", total: 95 }]);
    expect(describeTargetIssues(check)).toContain("total 95%");
  });

  const strategic = (
    classTargets: TargetDefinition["classTargets"],
    shares: [string, "equities" | "bonds", number][]
  ): TargetDefinition => ({
    mode: "strategic",
    classTargets,
    assets: shares.map(([id, assetClass, weight]) => ({
      id,
      assetClass,
      weight,
    })),
  });

  /**
   * GIVEN class targets and in-class shares that each total 100%
   * WHEN the totals are checked in Strategic mode
   * THEN allocating is allowed.
   */
  it("accepts strategic targets that are complete at both levels", () => {
    const check = checkTargetTotals(
      strategic({ equities: 70, bonds: 30 }, [
        ["a", "equities", 60],
        ["b", "equities", 40],
        ["c", "bonds", 100],
      ])
    );
    expect(check.ok).toBe(true);
  });

  /**
   * GIVEN class targets totalling 95% and equity shares totalling 97%
   * WHEN the totals are checked
   * THEN both issues are reported and described with their real totals.
   */
  it("reports every strategic level that misses 100%", () => {
    const check = checkTargetTotals(
      strategic({ equities: 65, bonds: 30 }, [
        ["a", "equities", 57],
        ["b", "equities", 40],
        ["c", "bonds", 100],
      ])
    );
    expect(check.issues).toEqual([
      { scope: "class-targets", total: 95 },
      { scope: "class-shares", assetClass: "equities", total: 97 },
    ]);
    const text = describeTargetIssues(check) ?? "";
    expect(text).toContain("class targets total 95%");
    expect(text).toContain("Equities asset shares total 97%");
  });

  /**
   * GIVEN an asset class that has assets but no class target
   * WHEN the totals are checked
   * THEN the missing target counts as zero, so the class targets fall short.
   */
  it("treats a missing class target as zero", () => {
    const check = checkTargetTotals(
      strategic({ equities: 100 }, [
        ["a", "equities", 100],
        ["c", "bonds", 100],
      ])
    );
    expect(check.ok).toBe(true);
    const short = checkTargetTotals(
      strategic({ equities: 60 }, [
        ["a", "equities", 100],
        ["c", "bonds", 100],
      ])
    );
    expect(short.issues).toEqual([{ scope: "class-targets", total: 60 }]);
  });
});

describe("targetTotalsMessage", () => {
  /**
   * GIVEN class targets that total 100% but put weight on a class with no assets
   * WHEN the blocked message is derived
   * THEN the title names the empty class problem, not a total that misses 100%.
   */
  it("titles a class-without-assets-only problem correctly", () => {
    const check = checkTargetTotals({
      mode: "strategic",
      classTargets: { equities: 90, bonds: 10 },
      assets: [{ id: "a", assetClass: "equities", weight: 100 }],
    });
    expect(targetTotalsMessage(check)?.title).toBe(
      "Some asset classes have weight but no assets"
    );
  });

  /**
   * GIVEN simple weights that total 95%
   * WHEN the blocked message is derived
   * THEN the title says target weights must total 100%.
   */
  it("keeps the total title for totals that miss 100%", () => {
    expect(
      targetTotalsMessage(checkTargetTotals(simple([60, 35])))?.title
    ).toBe("Target weights must total 100%");
  });
});
