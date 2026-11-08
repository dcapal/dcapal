import type { DriftRowModel, DriftSnapshot } from "./invest-types";

/** Outcome of checking whether the investor may continue from the plan step. */
export interface ContinueCheck {
  allowed: boolean;
  /** Why continuing is blocked; null when it is allowed. */
  reason: string | null;
}

/**
 * Reads an investment budget typed by the investor.
 *
 * Accepts digits with comma or dot separators, ignores any other character
 * including minus signs, and keeps at most two decimals. Returns null when no
 * digit was typed.
 */
export function parseAmountInput(raw: string): number | null {
  const cleaned = raw.replace(/[^0-9.,]/g, "");
  if (!/[0-9]/.test(cleaned)) {
    return null;
  }

  const lastSeparator = Math.max(
    cleaned.lastIndexOf("."),
    cleaned.lastIndexOf(",")
  );
  let integerPart = cleaned;
  let decimalPart = "";

  if (lastSeparator >= 0) {
    const before = cleaned.slice(0, lastSeparator);
    const after = cleaned.slice(lastSeparator + 1);
    const hasOtherSeparator = /[.,]/.test(before);
    const sameKind = new Set(cleaned.match(/[.,]/g)).size === 1;
    const looksLikeThousands =
      sameKind &&
      after.length === 3 &&
      /[1-9]/.test(before.replace(/[.,]/g, ""));
    const repeatedSameSeparator = sameKind && hasOtherSeparator;

    if (!looksLikeThousands && !repeatedSameSeparator) {
      integerPart = before;
      decimalPart = after;
    }
  }

  const digits = integerPart.replace(/[.,]/g, "") || "0";
  const value = Number(`${digits}.${decimalPart.slice(0, 2) || "0"}`);
  return Number.isFinite(value) ? value : null;
}

/**
 * Decides whether the investor may leave the plan step.
 *
 * Continuing needs the investment budget plus the Portfolio value to be above
 * zero; otherwise there is nothing to invest or rebalance.
 */
export function canContinue(
  amount: number | null,
  portfolioValue: number
): ContinueCheck {
  const budget = amount !== null && amount > 0 ? amount : 0;
  const value = portfolioValue > 0 ? portfolioValue : 0;

  if (budget + value > 0) {
    return { allowed: true, reason: null };
  }

  return {
    allowed: false,
    reason: "Enter an amount to invest, or add holdings to your portfolio.",
  };
}

/**
 * Computes Total drift: the share of the Portfolio that would have to move to
 * reach the Target weights, in percentage points.
 */
export function totalDriftPp(
  rows: ReadonlyArray<Pick<DriftRowModel, "currentWeight" | "targetWeight">>
): number {
  const deviations = rows.reduce(
    (sum, row) => sum + Math.abs(row.currentWeight - row.targetWeight),
    0
  );
  return deviations / 2;
}

/**
 * Finds the row whose weight is furthest from its target, or null when there
 * are no rows. The first row wins a tie.
 */
export function largestGap<
  Row extends Pick<DriftRowModel, "currentWeight" | "targetWeight">,
>(rows: readonly Row[]): Row | null {
  let widest: Row | null = null;
  let widestGap = -1;

  for (const row of rows) {
    const gap = Math.abs(row.currentWeight - row.targetWeight);
    if (gap > widestGap) {
      widest = row;
      widestGap = gap;
    }
  }

  return widest;
}

/** Builds the drift summary for a set of rows and the Portfolio value. */
export function createDriftSnapshot(
  rows: DriftRowModel[],
  portfolioValue: number,
  currency = "EUR"
): DriftSnapshot {
  return {
    currency,
    portfolioValue,
    rows,
    totalDriftPp: totalDriftPp(rows),
    largestGap: largestGap(rows),
  };
}
