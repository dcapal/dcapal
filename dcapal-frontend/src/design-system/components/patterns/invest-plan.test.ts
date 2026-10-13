import { describe, expect, it } from "vitest";

import {
  canContinue,
  createDriftSnapshot,
  largestGap,
  parseAmountInput,
  totalDriftPp,
} from "./invest-plan";

/**
 * GIVEN text typed into the investment budget field
 * WHEN it is parsed
 * THEN digits, commas and dots are read and anything else, including minus
 * signs, is ignored.
 */
describe("parseAmountInput", () => {
  it.each([
    ["1000", 1000],
    ["1,000", 1000],
    ["1.000", 1000],
    ["1,000.50", 1000.5],
    ["1.000,50", 1000.5],
    ["12,5", 12.5],
    ["0.5", 0.5],
    ["1,000,000", 1000000],
    ["€ 250", 250],
    ["-300", 300],
    ["12.345", 12345],
    ["10.999", 10999],
    ["10.9", 10.9],
    ["10.987,6543", 10987.65],
  ])("reads %s as %s", (raw, expected) => {
    expect(parseAmountInput(raw)).toBe(expected);
  });

  it("returns null when GIVEN text WHEN it has no digit THEN nothing is read", () => {
    expect(parseAmountInput("")).toBeNull();
    expect(parseAmountInput("abc,.-")).toBeNull();
  });
});

/**
 * GIVEN an investment budget and a Portfolio value
 * WHEN the investor tries to continue
 * THEN continuing is allowed only when budget plus value is above zero, and a
 * reason is given otherwise.
 */
describe("canContinue", () => {
  it("allows a Rebalancing with no new money when the Portfolio has value", () => {
    expect(canContinue(0, 10825.4)).toEqual({ allowed: true, reason: null });
  });

  it("allows new money into an empty Portfolio", () => {
    expect(canContinue(1000, 0).allowed).toBe(true);
  });

  it("blocks with a reason when there is no budget and no value", () => {
    const check = canContinue(0, 0);
    expect(check.allowed).toBe(false);
    expect(check.reason).toMatch(/amount/i);
  });

  it("treats an empty field and a negative budget as zero", () => {
    expect(canContinue(null, 0).allowed).toBe(false);
    expect(canContinue(-50, 0).allowed).toBe(false);
    expect(canContinue(-50, 100).allowed).toBe(true);
  });
});

/**
 * GIVEN current and target weights
 * WHEN Total drift is computed
 * THEN it is half the sum of absolute deviations, in percentage points.
 */
describe("totalDriftPp", () => {
  it("is zero when every weight is on target", () => {
    expect(
      totalDriftPp([
        { currentWeight: 60, targetWeight: 60 },
        { currentWeight: 40, targetWeight: 40 },
      ])
    ).toBe(0);
  });

  it("halves the summed deviations", () => {
    expect(
      totalDriftPp([
        { currentWeight: 70, targetWeight: 60 },
        { currentWeight: 30, targetWeight: 40 },
      ])
    ).toBeCloseTo(10, 6);
  });

  it("returns zero for no rows", () => {
    expect(totalDriftPp([])).toBe(0);
  });
});

/**
 * GIVEN rows with current and target weights
 * WHEN the largest gap is requested
 * THEN the row furthest from its target is returned.
 */
describe("largestGap", () => {
  const rows = [
    { id: "a", currentWeight: 32, targetWeight: 30 },
    { id: "b", currentWeight: 2, targetWeight: 25 },
    { id: "c", currentWeight: 36, targetWeight: 10 },
  ];

  it("picks the widest absolute gap", () => {
    expect(largestGap(rows)?.id).toBe("c");
  });

  it("keeps the first row on a tie", () => {
    expect(
      largestGap([
        { id: "x", currentWeight: 10, targetWeight: 15 },
        { id: "y", currentWeight: 20, targetWeight: 15 },
      ])?.id
    ).toBe("x");
  });

  it("returns null for no rows", () => {
    expect(largestGap([])).toBeNull();
  });
});

/**
 * GIVEN drift rows and a Portfolio value
 * WHEN a snapshot is created
 * THEN it carries the rows, Total drift and the largest gap.
 */
describe("createDriftSnapshot", () => {
  it("summarises the rows", () => {
    const rows = [
      {
        id: "a",
        name: "A",
        ticker: "A",
        assetClass: "equities" as const,
        currentWeight: 70,
        targetWeight: 60,
        driftPp: 10,
      },
      {
        id: "b",
        name: "B",
        ticker: "B",
        assetClass: "bonds" as const,
        currentWeight: 30,
        targetWeight: 40,
        driftPp: -10,
      },
    ];

    const snapshot = createDriftSnapshot(rows, 5000);

    expect(snapshot.totalDriftPp).toBeCloseTo(10, 6);
    expect(snapshot.largestGap?.id).toBe("a");
    expect(snapshot.portfolioValue).toBe(5000);
  });
});
