import { assetClassLabels, type AssetClass } from "../../fixtures/portfolio";
import { formatTargetWeight } from "./invest-format";
import type { AllocationMode } from "./invest-types";

/** Distance from 100%, in percentage points, that still counts as a full total (rounding). */
export const TARGET_TOTAL_TOLERANCE_PP = 0.05;

/**
 * The target weights the investor has defined. In Simple mode `weight` is an
 * absolute target weight; in Strategic mode it is the relative target weight
 * inside the asset's class and `classTargets` holds the asset-class weights.
 */
export interface TargetDefinition {
  mode: AllocationMode;
  assets: { id: string; assetClass: AssetClass; weight: number }[];
  /** Asset-class target weights, in percent; used by Strategic mode only. */
  classTargets?: Partial<Record<AssetClass, number>>;
}

/** Where a target total misses 100%. */
export type TargetTotalIssue =
  | { scope: "asset-weights"; total: number }
  | { scope: "class-targets"; total: number }
  | { scope: "class-without-assets"; assetClass: AssetClass; total: number }
  | { scope: "class-shares"; assetClass: AssetClass; total: number };

/** Result of checking whether target weights can be used for an Allocation or Rebalancing. */
export interface TargetTotalsCheck {
  ok: boolean;
  issues: TargetTotalIssue[];
}

function isFull(total: number): boolean {
  return Math.abs(total - 100) <= TARGET_TOTAL_TOLERANCE_PP;
}

/**
 * Checks that target weights total 100% at every level they are defined: the
 * asset weights in Simple mode; the class targets and the relative weights
 * inside each class in Strategic mode. Allocating or rebalancing needs all of
 * them to be complete.
 */
export function checkTargetTotals(
  definition: TargetDefinition
): TargetTotalsCheck {
  const issues: TargetTotalIssue[] = [];
  const total = (values: readonly number[]) =>
    values.reduce((sum, value) => sum + value, 0);

  if (definition.mode === "simple") {
    const sum = total(definition.assets.map((asset) => asset.weight));
    if (!isFull(sum)) issues.push({ scope: "asset-weights", total: sum });
  } else {
    const classes = new Set(definition.assets.map((a) => a.assetClass));
    // Every defined class target counts, including classes without assets, so
    // the total shown while editing equals the total that gates Invest.
    const classSum = total(Object.values(definition.classTargets ?? {}));
    if (!isFull(classSum)) {
      issues.push({ scope: "class-targets", total: classSum });
    }
    // A target on a class with no assets can never be reached.
    for (const [assetClass, weight] of Object.entries(
      definition.classTargets ?? {}
    ) as [AssetClass, number][]) {
      if (!classes.has(assetClass) && weight > TARGET_TOTAL_TOLERANCE_PP) {
        issues.push({
          scope: "class-without-assets",
          assetClass,
          total: weight,
        });
      }
    }
    for (const assetClass of classes) {
      const shares = total(
        definition.assets
          .filter((asset) => asset.assetClass === assetClass)
          .map((asset) => asset.weight)
      );
      if (!isFull(shares)) {
        issues.push({ scope: "class-shares", assetClass, total: shares });
      }
    }
  }
  return { ok: issues.length === 0, issues };
}

/** Joins phrases as plain text, such as "a, b and c". */
function joinPhrases(phrases: readonly string[]): string {
  if (phrases.length <= 1) return phrases.join("");
  return `${phrases.slice(0, -1).join(", ")} and ${phrases[phrases.length - 1]}`;
}

/**
 * Explains in plain language which totals miss 100%, derived from the actual
 * totals. Returns null when every total is complete.
 */
export function describeTargetIssues(check: TargetTotalsCheck): string | null {
  if (check.ok) return null;
  const percent = (value: number) =>
    formatTargetWeight(Number(value.toFixed(1)));
  const totals: string[] = [];
  const empty: string[] = [];
  for (const issue of check.issues) {
    switch (issue.scope) {
      case "asset-weights":
        totals.push(`Your target weights total ${percent(issue.total)}`);
        break;
      case "class-targets":
        totals.push(`Your class targets total ${percent(issue.total)}`);
        break;
      case "class-shares":
        totals.push(
          `${assetClassLabels[issue.assetClass]} asset shares total ${percent(issue.total)}`
        );
        break;
      case "class-without-assets":
        empty.push(
          `${assetClassLabels[issue.assetClass]} has a ${percent(issue.total)} target but no assets`
        );
        break;
    }
  }
  const sentences = [
    totals.length > 0 ? `${joinPhrases(totals)}, not 100%` : null,
    empty.length > 0 ? joinPhrases(empty) : null,
  ].filter((sentence): sentence is string => sentence !== null);
  return `${sentences.join(". ")}. Adjust them from the Portfolio page, then try again.`;
}

/** Title and explanation of the proposal state shown when target weights are incomplete. */
export interface TargetTotalsMessage {
  title: string;
  description: string;
}

/**
 * The message of the blocked proposal state, derived from the actual totals.
 * Returns null when the targets are complete.
 */
export function targetTotalsMessage(
  check: TargetTotalsCheck
): TargetTotalsMessage | null {
  const description = describeTargetIssues(check);
  if (!description) return null;
  // Totals that already add up to 100% are not the problem when the only
  // issue is weight placed on a class that holds no assets.
  const onlyEmptyClasses = check.issues.every(
    (issue) => issue.scope === "class-without-assets"
  );
  return {
    title: onlyEmptyClasses
      ? "Some asset classes have weight but no assets"
      : "Target weights must total 100%",
    description,
  };
}
