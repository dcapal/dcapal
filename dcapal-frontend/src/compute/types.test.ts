import { describe, expect, it } from "vitest";
import { isAnalyzeRequest, isSolveRequest } from "./types";

describe("compute types guards", () => {
  it("accepts analyze payload only for plain object", () => {
    expect(isAnalyzeRequest({ SPY: {} })).toBe(true);
    expect(isAnalyzeRequest(null)).toBe(false);
    expect(isAnalyzeRequest([])).toBe(false);
  });

  it("validates solve request required fields and types", () => {
    expect(
      isSolveRequest({
        budget: 100,
        assets: { SPY: {} },
        pfolioCcy: "USD",
        fees: null,
        isBuyOnly: true,
        useAllBudget: false,
      })
    ).toBe(true);

    expect(
      isSolveRequest({
        budget: "100",
        assets: { SPY: {} },
        pfolioCcy: "USD",
        fees: null,
        isBuyOnly: true,
        useAllBudget: false,
      })
    ).toBe(false);

    expect(
      isSolveRequest({
        budget: 100,
        assets: { SPY: {} },
        pfolioCcy: "USD",
        isBuyOnly: true,
        useAllBudget: false,
      })
    ).toBe(false);
  });
});
