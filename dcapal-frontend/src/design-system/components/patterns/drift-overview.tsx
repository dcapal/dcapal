import { formatCurrency, formatPercent } from "../../fixtures/portfolio";
import { cn } from "../../lib/cn";
import { Badge, Card, CardContent } from "../ui";
import { AllocationBar, allocationBarScale } from "./allocation-bar";
import { formatDeviation, formatPoints } from "./invest-format";
import type { DriftRowModel, DriftSnapshot } from "./invest-types";
import { MetricStrip } from "./metric-card";

/** Default distance from target, in percentage points, that counts as a real gap. */
export const DEFAULT_DRIFT_BAND_PP = 5;

type DriftTone = "over" | "under" | "within";

function driftTone(driftPp: number, bandPp: number): DriftTone {
  if (driftPp > bandPp) return "over";
  if (driftPp < -bandPp) return "under";
  return "within";
}

const toneBadge = {
  over: "destructive",
  under: "warning",
  within: "neutral",
} as const;

const toneText = {
  over: "above target",
  under: "below target",
  within: "within range",
} as const;

/** Names and subtitles a row: cash is named by its asset class, others by ticker. */
function rowLabels(row: DriftRowModel): { title: string; subtitle: string } {
  return row.assetClass === "cash"
    ? { title: row.name, subtitle: row.ticker }
    : { title: row.ticker, subtitle: row.name };
}

export interface DriftRowProps {
  row: DriftRowModel;
  /** Weight that fills the bar track, shared by the rows of one list. */
  scaleMax?: number;
  /** Distance from target, in percentage points, beyond which the gap is flagged. */
  bandPp?: number;
  /** Compact rows trade the deviation badge for a one-line weight summary. */
  density?: "default" | "compact";
}

/**
 * One Portfolio Asset compared with its target: name, current weight, a bar
 * with a target marker, and how far it is from target.
 */
export function DriftRow({
  row,
  scaleMax = 100,
  bandPp = DEFAULT_DRIFT_BAND_PP,
  density = "default",
}: DriftRowProps) {
  const tone = driftTone(row.driftPp, bandPp);
  const { title, subtitle } = rowLabels(row);

  return (
    <li className={cn("ds-drift-row", `ds-drift-row--${density}`)}>
      <span className="ds-drift-row__identity">
        <strong>{title}</strong>
        <span>{subtitle}</span>
      </span>
      <AllocationBar
        name={row.name}
        assetClass={row.assetClass}
        today={row.currentWeight}
        target={row.targetWeight}
        scaleMax={scaleMax}
      />
      {density === "compact" ? (
        <span className="ds-drift-row__summary">
          {formatPercent(row.currentWeight)} · target{" "}
          {formatPercent(row.targetWeight)}
        </span>
      ) : (
        <>
          <span className="ds-drift-row__weight">
            {formatPercent(row.currentWeight)}
          </span>
          <Badge className="ds-drift-row__badge" variant={toneBadge[tone]}>
            {formatDeviation(row.driftPp)}
            <span className="ds-sr-only"> {toneText[tone]}</span>
          </Badge>
        </>
      )}
    </li>
  );
}

export interface DriftRowListProps {
  rows: readonly DriftRowModel[];
  bandPp?: number;
  density?: "default" | "compact";
  /** Accessible name of the list. */
  label: string;
}

/** The rows of a drift comparison, biggest gap first, on one shared bar scale. */
export function DriftRowList({
  rows,
  bandPp,
  density,
  label,
}: DriftRowListProps) {
  const scaleMax = allocationBarScale(
    rows.flatMap((row) => [row.currentWeight, row.targetWeight])
  );
  const ordered = [...rows].sort(
    (a, b) => Math.abs(b.driftPp) - Math.abs(a.driftPp)
  );

  return (
    <ul className="ds-drift-row-list" aria-label={label}>
      {ordered.map((row) => (
        <DriftRow
          key={row.id}
          row={row}
          scaleMax={scaleMax}
          bandPp={bandPp}
          density={density}
        />
      ))}
    </ul>
  );
}

export interface DriftOverviewProps {
  /** How far the Portfolio currently sits from its Target weights. */
  snapshot: DriftSnapshot;
  /** Distance from target, in percentage points, beyond which a gap is flagged. */
  bandPp?: number;
  /** Heading of the comparison card. */
  title?: string;
  className?: string;
}

/**
 * The drift view of a Portfolio: Total drift, the largest gap and the
 * Portfolio value, followed by every asset against its target.
 */
export function DriftOverview({
  snapshot,
  bandPp,
  title = "Current vs target",
  className,
}: DriftOverviewProps) {
  const { largestGap } = snapshot;

  return (
    <div className={cn("ds-drift-overview", className)}>
      <MetricStrip
        label="Portfolio drift summary"
        metrics={[
          { label: "Total drift", value: formatPoints(snapshot.totalDriftPp) },
          {
            label: "Largest gap",
            value: largestGap
              ? `${rowLabels(largestGap).title} ${formatDeviation(largestGap.driftPp)}`
              : "None",
          },
          {
            label: "Portfolio value",
            value: formatCurrency(snapshot.portfolioValue, snapshot.currency),
          },
        ]}
      />
      <Card>
        <CardContent>
          <div className="ds-drift-overview__heading">
            <div className="ds-drift-overview__titles">
              <h2 className="ds-card__title">{title}</h2>
              <span className="ds-drift-overview__hint">
                Marker = target · biggest gap first
              </span>
            </div>
          </div>
          <DriftRowList
            rows={snapshot.rows}
            bandPp={bandPp}
            label="Assets compared with their targets"
          />
        </CardContent>
      </Card>
    </div>
  );
}
