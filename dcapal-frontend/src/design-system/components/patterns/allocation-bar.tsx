import type { AssetClass } from "../../fixtures/portfolio";
import { formatPercent } from "../../fixtures/portfolio";
import { cn } from "../../lib/cn";

/**
 * Picks a shared scale for a set of bars: the largest value rounded up to the
 * next multiple of five, never below ten, so rows stay comparable.
 */
export function allocationBarScale(values: readonly number[]): number {
  const largest = values.reduce((max, value) => Math.max(max, value), 0);
  return Math.max(10, Math.ceil(largest / 5) * 5);
}

export interface AllocationBarProps {
  /** Name of the Portfolio Asset, used in the accessible description. */
  name: string;
  /** Asset class that sets the bar colour. */
  assetClass: AssetClass;
  /** Current weight today, in percent of the Portfolio. */
  today: number;
  /** Projected weight after a proposal; when set, today is shown as a ghost bar. */
  after?: number;
  /** Target weight, in percent, drawn as a marker. */
  target: number;
  /** Weight that fills the whole track, so several bars can share one scale. */
  scaleMax?: number;
  className?: string;
}

function clampPercent(value: number, scaleMax: number): number {
  return Math.min(100, Math.max(0, (value / scaleMax) * 100));
}

/**
 * A horizontal bar comparing a weight today, optionally the weight after a
 * proposal, and the target weight. Its text description states all values so
 * the comparison never depends on colour or position alone.
 */
export function AllocationBar({
  name,
  assetClass,
  today,
  after,
  target,
  scaleMax = 100,
  className,
}: AllocationBarProps) {
  const description =
    after === undefined
      ? `${name}: today ${formatPercent(today)}, target ${formatPercent(target)}`
      : `${name}: today ${formatPercent(today)}, after ${formatPercent(after)}, target ${formatPercent(target)}`;

  return (
    <span
      role="img"
      aria-label={description}
      className={cn(
        "ds-allocation-bar",
        `ds-allocation-bar--${assetClass}`,
        className
      )}
    >
      {after === undefined ? (
        <span
          className="ds-allocation-bar__fill"
          style={{ width: `${clampPercent(today, scaleMax)}%` }}
        />
      ) : (
        <>
          <span
            className="ds-allocation-bar__fill ds-allocation-bar__fill--today"
            style={{ width: `${clampPercent(today, scaleMax)}%` }}
          />
          <span
            className="ds-allocation-bar__fill"
            style={{ width: `${clampPercent(after, scaleMax)}%` }}
          />
        </>
      )}
      <span
        className="ds-allocation-bar__target"
        style={{ left: `${clampPercent(target, scaleMax)}%` }}
      />
    </span>
  );
}
