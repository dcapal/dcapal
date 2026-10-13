import * as React from "react";
import { formatPercent } from "../../fixtures/portfolio";
import { cn } from "../../lib/cn";
import { AllocationBar, allocationBarScale } from "./allocation-bar";
import type { AllocationAfterRow } from "./invest-types";

export interface AllocationAfterListProps {
  rows: readonly AllocationAfterRow[];
  /** Heading of the list. */
  title?: string;
  className?: string;
}

/** Cash is named by its asset class; other assets by ticker. */
function rowTitle(row: AllocationAfterRow): string {
  return row.assetClass === "cash" ? row.name : row.ticker;
}

/**
 * Every asset's weight today, after a proposal and at target: a shaded bar for
 * today, a solid bar for after, and a marker for the target.
 */
export function AllocationAfterList({
  rows,
  title = "Allocation after",
  className,
}: AllocationAfterListProps) {
  const headingId = React.useId();
  const scaleMax = allocationBarScale(
    rows.flatMap((row) => [row.weightBefore, row.weightAfter, row.targetWeight])
  );

  return (
    <section
      aria-labelledby={headingId}
      className={cn("ds-allocation-after", className)}
    >
      <h3 className="ds-allocation-after__title" id={headingId}>
        {title}
      </h3>
      <p className="ds-allocation-after__hint">
        Shaded bar is today, solid bar is after, marker is your target.
      </p>
      <ul className="ds-allocation-after__list">
        {rows.map((row) => (
          <li className="ds-allocation-after__row" key={row.id}>
            <strong>{rowTitle(row)}</strong>
            <AllocationBar
              name={row.name}
              assetClass={row.assetClass}
              today={row.weightBefore}
              after={row.weightAfter}
              target={row.targetWeight}
              scaleMax={scaleMax}
            />
            <span className="ds-allocation-after__values">
              {formatPercent(row.weightBefore)} {"→"}{" "}
              {formatPercent(row.weightAfter)} · target{" "}
              {formatPercent(row.targetWeight)}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
