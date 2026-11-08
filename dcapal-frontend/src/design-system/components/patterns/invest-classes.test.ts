import { describe, expect, it } from "vitest";
import {
  classDriftBeforeAfter,
  collapsesToClassRow,
  groupAllocationAfterByClass,
  groupProposalByClass,
  groupResultByClass,
} from "./invest-classes";
import type { AllocationAfterRow, ProposedTransaction } from "./invest-types";

const after = (
  id: string,
  assetClass: AllocationAfterRow["assetClass"],
  before: number,
  now: number,
  target: number
): AllocationAfterRow => ({
  id,
  name: id,
  ticker: id.toUpperCase(),
  assetClass,
  weightBefore: before,
  weightAfter: now,
  targetWeight: target,
});

const allocationAfter = [
  after("vwce", "equities", 32, 30, 30),
  after("ieur", "equities", 14, 16, 20),
  after("aggh", "bonds", 2, 7, 25),
  after("cash", "cash", 36, 33, 10),
];

const buy = (id: string, amount: number): ProposedTransaction => ({
  id,
  name: id,
  ticker: id.toUpperCase(),
  assetClass: id === "aggh" ? "bonds" : "equities",
  action: "buy",
  quantity: 1,
  price: amount,
  amount,
  weightBefore: 0,
  weightAfter: 0,
  targetWeight: 0,
});

describe("groupProposalByClass", () => {
  /**
   * GIVEN a proposal that buys one bond and one equity and holds VWCE
   * WHEN it is grouped by class
   * THEN hold rows are synthesised only in the groups, net money is the buys
   * of the class, cash holds the unallocated money, and classes sort by the
   * money they move.
   */
  it("groups transactions, keeps holds and nets money per class", () => {
    const transactions = [buy("ieur", 365), buy("aggh", 614)];
    const groups = groupProposalByClass({
      transactions,
      allocationAfter,
      leftoverCash: 18,
      fundedBy: { newMoney: 1000, saleProceeds: 0, cashUsed: 0 },
    });
    expect(transactions).toHaveLength(2);
    expect(groups.map((g) => [g.assetClass, g.netMoney])).toEqual([
      ["bonds", 614],
      ["equities", 365],
      ["cash", 18],
    ]);
    const equities = groups[1];
    expect(equities.buys).toBe(1);
    expect(equities.weightBefore).toBe(46);
    expect(equities.weightAfter).toBe(46);
    expect(equities.targetWeight).toBe(50);
    expect(
      equities.assets.map((a) => [a.row.id, a.transaction?.action])
    ).toEqual([
      ["ieur", "buy"],
      ["vwce", undefined],
    ]);
  });

  /**
   * GIVEN a rebalancing that spends 2,800 of the cash balance
   * WHEN it is grouped
   * THEN the cash class shows the money leaving it as a negative amount.
   */
  it("shows cash spent as negative net money", () => {
    const groups = groupProposalByClass({
      transactions: [buy("aggh", 2800)],
      allocationAfter,
      leftoverCash: 0,
      fundedBy: { newMoney: 0, saleProceeds: 0, cashUsed: 2800 },
    });
    expect(groups.find((g) => g.assetClass === "cash")?.netMoney).toBe(-2800);
  });
});

describe("class-level allocation", () => {
  /**
   * GIVEN allocation rows
   * WHEN they are rolled up to classes
   * THEN weights and targets are summed in canonical class order, and total
   * drift before and after is measured on class weights.
   */
  it("rolls rows up to classes and measures drift on them", () => {
    const groups = groupAllocationAfterByClass(allocationAfter);
    expect(groups.map((g) => g.assetClass)).toEqual([
      "equities",
      "bonds",
      "cash",
    ]);
    expect(groups[0]).toMatchObject({
      assetCount: 2,
      weightBefore: 46,
      weightAfter: 46,
      targetWeight: 50,
    });
    const { before, after: afterDrift } =
      classDriftBeforeAfter(allocationAfter);
    expect(before).toBeCloseTo((4 + 23 + 26) / 2, 5);
    expect(afterDrift).toBeCloseTo((4 + 18 + 23) / 2, 5);
  });
});

describe("groupResultByClass", () => {
  /**
   * GIVEN changes with asset classes and class summaries
   * WHEN the result is grouped
   * THEN classes follow the canonical order and carry their summary, and
   * classes without a change are left out.
   */
  it("groups changes and attaches class summaries", () => {
    const groups = groupResultByClass({
      changes: [
        {
          id: "aggh",
          name: "AGGH",
          ticker: "AGGH",
          assetClass: "bonds",
          quantityBefore: 1,
          quantityAfter: 3,
        },
        {
          id: "ieur",
          name: "IEUR",
          ticker: "IEUR",
          assetClass: "equities",
          quantityBefore: 1,
          quantityAfter: 2,
        },
      ],
      classes: [{ assetClass: "equities", weightNow: 60, targetWeight: 65 }],
    });
    expect(groups.map((g) => g.assetClass)).toEqual(["equities", "bonds"]);
    expect(groups[0].weightNow).toBe(60);
    expect(groups[1].weightNow).toBeNull();
  });
});

describe("collapsesToClassRow", () => {
  /**
   * GIVEN cash held in one account and in two accounts
   * WHEN deciding whether the class is one row
   * THEN a single cash asset collapses to its class row, two cash assets do
   * not so each one's change stays visible, and other classes never collapse.
   */
  it("lists every cash asset when there is more than one", () => {
    expect(collapsesToClassRow("cash", 1)).toBe(true);
    expect(collapsesToClassRow("cash", 2)).toBe(false);
    expect(collapsesToClassRow("equities", 1)).toBe(false);
  });
});
