import * as React from "react";
import { formatPercent } from "../../fixtures/portfolio";
import { cn } from "../../lib/cn";
import { AllocationBar, allocationBarScale } from "./allocation-bar";
import { groupAllocationAfterByClass } from "./invest-classes";
import { formatDeviation } from "./invest-format";
import type { AllocationAfterRow, AllocationMode } from "./invest-types";

export interface AllocationAfterListProps {
  rows: readonly AllocationAfterRow[];
  /** Strategic mode shows one bar per asset class instead of one per asset. Defaults to simple. */
  mode?: AllocationMode;
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
  mode = "simple",
  title = "Allocation after",
  className,
}: AllocationAfterListProps) {
  const headingId = React.useId();
  if (mode === "strategic") {
    return (
      <ClassAllocationAfter
        rows={rows}
        title={title}
        headingId={headingId}
        className={className}
      />
    );
  }
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

/**
 * Every asset class's weight today, after a proposal and at its class target.
 * Each class has a one-line summary of how far from target it ends up.
 */
function ClassAllocationAfter({
  rows,
  title,
  headingId,
  className,
}: {
  rows: readonly AllocationAfterRow[];
  title: string;
  headingId: string;
  className?: string;
}) {
  const groups = groupAllocationAfterByClass(rows);
  const scaleMax = allocationBarScale(
    groups.flatMap((g) => [g.weightBefore, g.weightAfter, g.targetWeight])
  );

  return (
    <section
      aria-labelledby={headingId}
      className={cn(
        "ds-allocation-after ds-allocation-after--classes",
        className
      )}
    >
      <h3 className="ds-allocation-after__title" id={headingId}>
        {title}
      </h3>
      <p className="ds-allocation-after__hint">
        Shaded bar is today, solid bar is after, marker is your class target.
      </p>
      <ul className="ds-allocation-after__list">
        {groups.map((group) => (
          <li
            className="ds-allocation-after__row ds-allocation-after__row--class"
            key={group.assetClass}
            data-asset-class={group.assetClass}
            data-row-kind="class"
          >
            <strong>{group.label}</strong>
            <AllocationBar
              name={`${group.label} class`}
              assetClass={group.assetClass}
              today={group.weightBefore}
              after={group.weightAfter}
              target={group.targetWeight}
              scaleMax={scaleMax}
            />
            <span className="ds-allocation-after__values">
              {formatPercent(group.weightBefore)} {"→"}{" "}
              {formatPercent(group.weightAfter)} · target{" "}
              {formatPercent(group.targetWeight)}
            </span>
            <span className="ds-allocation-after__sub">
              {formatDeviation(group.weightAfter - group.targetWeight)} from
              target after · {group.assetCount}{" "}
              {group.assetCount === 1 ? "asset" : "assets"}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
