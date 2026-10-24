import { describe, expect, it } from "vitest";
import { portfolioAssets } from "../../fixtures/portfolio";
import { describeTargetIssues } from "./invest-targets";
import {
  allocationOnSwitchToStrategic,
  checkStrategicTotals,
  classTargetsFromAssets,
  currentMixFromAssets,
  deriveAbsoluteWeights,
  describeWeightTotal,
  emptyClassWeights,
  readWeightInput,
  WEIGHT_INPUT_ERROR,
  previewClassChange,
  previewShareChange,
  strategicFromAssets,
  withAssetShare,
  withClassTarget,
  withTargetWeights,
} from "./strategic-allocation";

const start = strategicFromAssets(portfolioAssets);

describe("starters", () => {
  /**
   * GIVEN the fixture portfolio with absolute targets 30/20/15 in equities
   * WHEN the class targets are derived
   * THEN each class target is the sum of its assets and empty classes are 0.
   */
  it("sums absolute targets into class targets", () => {
    expect(classTargetsFromAssets(portfolioAssets)).toEqual({
      equities: 65,
      bonds: 25,
      cash: 10,
      crypto: 0,
      commodities: 0,
      other: 0,
    });
  });

  /**
   * GIVEN current weights that total 99.9% after rounding
   * WHEN the current mix starter is built
   * THEN the missing 0.1 pp goes to the largest class and the mix totals 100%.
   */
  it("builds a complete current mix", () => {
    const mix = currentMixFromAssets(portfolioAssets);
    expect(mix.equities).toBeCloseTo(64, 5);
    expect(mix.bonds).toBeCloseTo(24.1, 5);
    expect(mix.cash).toBeCloseTo(11.9, 5);
    expect(
      describeWeightTotal(Object.values(mix).reduce((a, b) => a + b, 0)).state
    ).toBe("balanced");
  });

  /**
   * GIVEN the fixture targets
   * WHEN absolute weights are derived from the strategic starter
   * THEN they equal the original absolute targets.
   */
  it("round-trips absolute weights", () => {
    const weights = deriveAbsoluteWeights(portfolioAssets, start);
    expect(weights.vwce).toBeCloseTo(30, 6);
    expect(weights.ieur).toBeCloseTo(20, 6);
    expect(weights.vig).toBeCloseTo(15, 6);
    expect(weights.cash).toBeCloseTo(10, 6);
  });
});

describe("shares survive clearing", () => {
  /**
   * GIVEN all class weights cleared to 0
   * WHEN one class weight is entered again
   * THEN the asset shares inside the class are unchanged and the absolute
   * weights follow the new class weight.
   */
  it("keeps shares when the class weight is 0", () => {
    const cleared = { ...start, classTargets: emptyClassWeights() };
    const again = withClassTarget(cleared, "equities", 50);
    const weights = deriveAbsoluteWeights(portfolioAssets, again);
    expect(again.shares.vwce).toBeCloseTo(46.15, 1);
    expect(weights.vwce).toBeCloseTo(23.08, 1);
  });
});

describe("validation", () => {
  /**
   * GIVEN empty, valid, comma-decimal, negative, oversized and non-numeric text
   * WHEN it is read as a weight
   * THEN valid text keeps its value, empty is 0 without error, and invalid
   * text is flagged with an error and counts as 0 instead of being clamped.
   */
  it("reads percentage input without clamping", () => {
    expect(readWeightInput("")).toEqual({ value: 0, error: null });
    expect(readWeightInput("12,5")).toEqual({ value: 12.5, error: null });
    expect(readWeightInput("100")).toEqual({ value: 100, error: null });
    for (const bad of ["abc", "-3", "250", "100.1"]) {
      expect(readWeightInput(bad)).toEqual({
        value: 0,
        error: WEIGHT_INPUT_ERROR,
      });
    }
  });

  /**
   * GIVEN totals of 100, 95 and 103.5
   * WHEN they are described
   * THEN the states are balanced, short and over with the gap in the label.
   */
  it("labels totals", () => {
    expect(describeWeightTotal(100).label).toBe("Balanced");
    expect(describeWeightTotal(95).label).toBe("Short by 5.0 pp");
    expect(describeWeightTotal(103.5).label).toBe("Over by 3.5 pp");
  });

  /**
   * GIVEN a balanced strategic allocation
   * WHEN the totals gate is checked
   * THEN it passes.
   */
  it("passes a balanced allocation", () => {
    expect(checkStrategicTotals(portfolioAssets, start).ok).toBe(true);
  });

  /**
   * GIVEN a 5% weight on Crypto, which has no assets, and equities lowered by 5
   * WHEN the totals gate is checked
   * THEN the class total is 100% but the gate still fails and names Crypto.
   */
  it("blocks weight on a class without assets", () => {
    const moved = withClassTarget(
      withClassTarget(start, "equities", 60),
      "crypto",
      5
    );
    const check = checkStrategicTotals(portfolioAssets, moved);
    expect(check.ok).toBe(false);
    expect(check.issues.map((i) => i.scope)).toEqual(["class-without-assets"]);
    expect(describeTargetIssues(check)).toContain("Crypto has a 5% target");
  });

  /**
   * GIVEN the class weights sum to 90%
   * WHEN the gate is checked
   * THEN it reports the class-targets total.
   */
  it("reports unbalanced class targets", () => {
    const check = checkStrategicTotals(
      portfolioAssets,
      withClassTarget(start, "equities", 55)
    );
    expect(check.issues).toEqual([{ scope: "class-targets", total: 90 }]);
  });
});

describe("share and class previews", () => {
  /**
   * GIVEN VWCE's share is raised to 50% in Equities (65%)
   * WHEN the change is previewed
   * THEN siblings keep their shares, nothing is redistributed, the total is
   * over 100% and VWCE's weight in the portfolio is 32.5%.
   */
  it("previews a share without redistributing", () => {
    const preview = previewShareChange(portfolioAssets, start, "vwce", 50);
    const vwce = preview.rows.find((r) => r.assetId === "vwce");
    const ieur = preview.rows.find((r) => r.assetId === "ieur");
    expect(preview.rows).toHaveLength(3);
    expect(vwce?.absolute).toBeCloseTo(32.5, 6);
    expect(ieur?.after).toBeCloseTo(ieur?.before ?? 0, 6);
    expect(preview.total.state).toBe("over");
  });

  /**
   * GIVEN Equities lowered to 55%
   * WHEN the class change is previewed
   * THEN the other classes are unchanged and the total is short by 10 pp.
   */
  it("previews a class change", () => {
    const preview = previewClassChange(start, "equities", 55);
    expect(preview.classTargets.bonds).toBe(25);
    expect(preview.total.label).toBe("Short by 10.0 pp");
  });

  /**
   * GIVEN derived absolute weights
   * WHEN they are applied to the assets
   * THEN drift is recomputed as current minus target.
   */
  it("recomputes drift from the new targets", () => {
    const changed = withTargetWeights(portfolioAssets, { vwce: 40 });
    expect(changed[0].driftPp).toBeCloseTo(34.8 - 40, 6);
    expect(changed[1].targetWeight).toBe(25);
    expect(withAssetShare(start, "vwce", 10).shares.vwce).toBe(10);
  });
});

describe("switching between Simple and Strategic", () => {
  const absoluteOf = (weights: Record<string, number>) => weights;
  const base = Object.fromEntries(
    portfolioAssets.map((a) => [a.id, a.targetWeight])
  );

  /**
   * GIVEN a Simple Portfolio where the investor edited VWCE's absolute target to 40
   * WHEN they switch to Strategic with the class targets implied by the current weights
   * THEN VWCE still derives to 40, and switching back to Simple keeps it at 40.
   */
  it("keeps Simple edits across Strategic and back", () => {
    const absolute = absoluteOf({ ...base, vwce: 40 });
    const edited = withTargetWeights(portfolioAssets, absolute);
    const strategic = allocationOnSwitchToStrategic(
      portfolioAssets,
      absolute,
      start,
      classTargetsFromAssets(edited),
      true
    );
    const derived = deriveAbsoluteWeights(portfolioAssets, strategic);
    expect(derived.vwce).toBeCloseTo(40, 6);
    const backToSimple = deriveAbsoluteWeights(portfolioAssets, strategic);
    expect(backToSimple.vwce).toBeCloseTo(40, 6);
  });

  /**
   * GIVEN a Portfolio whose Simple weights were not edited since Strategic last applied
   * WHEN the investor switches to Strategic
   * THEN the stored shares are kept untouched.
   */
  it("keeps stored shares when nothing was edited in Simple", () => {
    const custom = withAssetShare(start, "vwce", 70);
    const result = allocationOnSwitchToStrategic(
      portfolioAssets,
      base,
      custom,
      custom.classTargets,
      false
    );
    expect(result.shares.vwce).toBe(70);
  });
});
