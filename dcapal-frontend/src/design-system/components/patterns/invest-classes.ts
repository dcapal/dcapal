import { assetClassLabels, type AssetClass } from "../../fixtures/portfolio";
import { largestGap, totalDriftPp } from "./invest-plan";
import type {
  AllocationAfterClassGroup,
  AllocationAfterRow,
  ClassDriftAsset,
  ClassDriftGroup,
  DriftRowModel,
  InvestmentProposalModel,
  InvestmentResultChange,
  InvestmentResultModel,
  ProposalClassGroup,
} from "./invest-types";
import {
  assetClassOrder,
  ratioPercent,
  type StrategicTargets,
} from "./portfolio-asset-groups";

/**
 * Class-level view models of the Invest journey in Strategic allocation mode.
 * An asset class's target is the sum of its assets' absolute targets, and an
 * asset's relative share is its absolute target over that class target. All
 * class figures are derived, never stored, so every step agrees.
 */

/** Asset classes that hold the given items, in the canonical class order. */
function classesInOrder<Item extends { assetClass: AssetClass }>(
  items: readonly Item[]
): { assetClass: AssetClass; members: Item[] }[] {
  return assetClassOrder
    .map((assetClass) => ({
      assetClass,
      members: items.filter((item) => item.assetClass === assetClass),
    }))
    .filter((group) => group.members.length > 0);
}

const sum = (values: readonly number[]) =>
  values.reduce((total, value) => total + value, 0);

/**
 * Whether a class is shown as a single row without its assets. Cash is a
 * funding source, not an asset the investor transacts, so its class row says
 * everything there is to say.
 */
export function isClassOnly(assetClass: AssetClass): boolean {
  return assetClass === "cash";
}

/**
 * Whether a class is collapsed to its own row: a class-only class (cash) with
 * a single member. A class-only class that holds several assets, such as two
 * cash accounts, lists each of them so none of their changes is hidden.
 */
export function collapsesToClassRow(
  assetClass: AssetClass,
  memberCount: number
): boolean {
  return isClassOnly(assetClass) && memberCount <= 1;
}

/**
 * Groups drift rows by asset class. Classes and the assets inside them are
 * ordered by the widest gap first; ties keep the canonical class order.
 */
export function groupDriftByClass(
  rows: readonly DriftRowModel[],
  stated?: StrategicTargets
): ClassDriftGroup[] {
  return classesInOrder(rows)
    .map(({ assetClass, members }): ClassDriftGroup => {
      const currentWeight = sum(members.map((row) => row.currentWeight));
      const derivedTarget = sum(members.map((row) => row.targetWeight));
      // A stated Strategic allocation wins over targets derived from the
      // rows, so class rows and the Invest gate come from one target set.
      const targetWeight = stated?.classTargets[assetClass] ?? derivedTarget;
      const assets = members
        .map((row): ClassDriftAsset => {
          const share = stated?.shares[row.id];
          const rowTarget =
            share === undefined
              ? row.targetWeight
              : (targetWeight * share) / 100;
          return {
            ...row,
            targetWeight: rowTarget,
            driftPp: row.currentWeight - rowTarget,
            shareOfClass:
              share ?? ratioPercent(row.targetWeight, derivedTarget),
          };
        })
        .sort((a, b) => Math.abs(b.driftPp) - Math.abs(a.driftPp));
      return {
        assetClass,
        label: assetClassLabels[assetClass],
        currentWeight,
        targetWeight,
        driftPp: currentWeight - targetWeight,
        assets,
      };
    })
    .sort((a, b) => Math.abs(b.driftPp) - Math.abs(a.driftPp));
}

/** Total drift measured on class weights, in percentage points. */
export function classTotalDriftPp(groups: readonly ClassDriftGroup[]): number {
  return totalDriftPp(groups);
}

/** The class with the widest gap from its class target, or null when there are none. */
export function classLargestGap(
  groups: readonly ClassDriftGroup[]
): ClassDriftGroup | null {
  return largestGap(groups);
}

/**
 * Groups a proposal by asset class. Assets without a transaction stay in the
 * group as holds, built here from the allocation rows and never added to the
 * proposal's transactions. A class's net money is what its buys cost minus
 * what its sales raise; for cash it is the unallocated money that stays in
 * cash minus the cash balance spent. Classes are ordered by the money they
 * move, largest first.
 */
export function groupProposalByClass(
  proposal: Pick<
    InvestmentProposalModel,
    "transactions" | "allocationAfter" | "leftoverCash" | "fundedBy"
  >
): ProposalClassGroup[] {
  const cashUsed = proposal.fundedBy?.cashUsed ?? 0;
  return classesInOrder(proposal.allocationAfter)
    .map(({ assetClass, members }): ProposalClassGroup => {
      const targetWeight = sum(members.map((row) => row.targetWeight));
      const assets = members
        .map((row) => ({
          row,
          transaction:
            proposal.transactions.find((t) => t.id === row.id) ?? null,
          shareOfClass: ratioPercent(row.targetWeight, targetWeight),
        }))
        .sort((a, b) => {
          const moved = (x: typeof a) =>
            x.transaction && x.transaction.action !== "hold" ? 1 : 0;
          return (
            moved(b) - moved(a) ||
            Math.abs(b.transaction?.amount ?? 0) -
              Math.abs(a.transaction?.amount ?? 0)
          );
        });
      const real = assets.flatMap(({ transaction }) =>
        transaction && transaction.action !== "hold" ? [transaction] : []
      );
      const bought = sum(
        real.filter((t) => t.action === "buy").map((t) => t.amount)
      );
      const sold = sum(
        real.filter((t) => t.action === "sell").map((t) => t.amount)
      );
      return {
        assetClass,
        label: assetClassLabels[assetClass],
        weightBefore: sum(members.map((row) => row.weightBefore)),
        weightAfter: sum(members.map((row) => row.weightAfter)),
        targetWeight,
        netMoney: isClassOnly(assetClass)
          ? proposal.leftoverCash - cashUsed
          : bought - sold,
        buys: real.filter((t) => t.action === "buy").length,
        sells: real.filter((t) => t.action === "sell").length,
        assets,
      };
    })
    .sort((a, b) => Math.abs(b.netMoney) - Math.abs(a.netMoney));
}

/** Rolls the allocation rows of a proposal up to asset classes, in the canonical order. */
export function groupAllocationAfterByClass(
  rows: readonly AllocationAfterRow[]
): AllocationAfterClassGroup[] {
  return classesInOrder(rows).map(({ assetClass, members }) => ({
    assetClass,
    label: assetClassLabels[assetClass],
    assetCount: members.length,
    weightBefore: sum(members.map((row) => row.weightBefore)),
    weightAfter: sum(members.map((row) => row.weightAfter)),
    targetWeight: sum(members.map((row) => row.targetWeight)),
  }));
}

/** Total drift of a proposal measured on class weights: before and after, in percentage points. */
export function classDriftBeforeAfter(rows: readonly AllocationAfterRow[]): {
  before: number;
  after: number;
} {
  const groups = groupAllocationAfterByClass(rows);
  return {
    before: totalDriftPp(
      groups.map((g) => ({
        currentWeight: g.weightBefore,
        targetWeight: g.targetWeight,
      }))
    ),
    after: totalDriftPp(
      groups.map((g) => ({
        currentWeight: g.weightAfter,
        targetWeight: g.targetWeight,
      }))
    ),
  };
}

/** One asset class of an updated portfolio with the changes inside it. */
export interface ResultClassGroup {
  assetClass: AssetClass;
  label: string;
  /** Class weight once updated, when the result carries class figures. */
  weightNow: number | null;
  /** Asset-class target weight, when the result carries class figures. */
  targetWeight: number | null;
  changes: InvestmentResultChange[];
}

/**
 * Groups the quantity changes of a result by asset class, in the canonical
 * order. Changes without a class land in "Other". Classes without any change
 * do not appear.
 */
export function groupResultByClass(
  result: Pick<InvestmentResultModel, "changes" | "classes">
): ResultClassGroup[] {
  const withClass = result.changes.map((change) => ({
    ...change,
    assetClass: change.assetClass ?? ("other" as AssetClass),
  }));
  return classesInOrder(withClass).map(({ assetClass, members }) => {
    const summary = result.classes?.find((c) => c.assetClass === assetClass);
    return {
      assetClass,
      label: assetClassLabels[assetClass],
      weightNow: summary?.weightNow ?? null,
      targetWeight: summary?.targetWeight ?? null,
      changes: members,
    };
  });
}
