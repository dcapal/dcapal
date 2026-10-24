import { formatCurrency, formatPercent } from "../../fixtures/portfolio";
import { cn } from "../../lib/cn";
import { Badge, Card, CardContent } from "../ui";
import { AllocationBar, allocationBarScale } from "./allocation-bar";
import {
  classLargestGap,
  classTotalDriftPp,
  groupDriftByClass,
  collapsesToClassRow,
} from "./invest-classes";
import {
  formatDeviation,
  formatPoints,
  formatTargetWeight,
} from "./invest-format";
import type {
  AllocationMode,
  ClassDriftAsset,
  ClassDriftGroup,
  DriftRowModel,
  DriftSnapshot,
} from "./invest-types";
import { MetricStrip } from "./metric-card";
import {
  DEFAULT_DRIFT_BAND_PP,
  type StrategicTargets,
} from "./portfolio-asset-groups";
import { TARGET_TOTAL_TOLERANCE_PP } from "./invest-targets";

type DriftTone = "over" | "under" | "within";

export function driftTone(driftPp: number, bandPp: number): DriftTone {
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

/**
 * Signed distance from target with its over / under / within wording as
 * screen-reader text. One Portfolio-wide Drift band classifies every row, class
 * or asset.
 */
function DriftBadge({
  driftPp,
  bandPp,
  className,
}: {
  driftPp: number;
  bandPp: number;
  className?: string;
}) {
  const tone = driftTone(driftPp, bandPp);
  return (
    <Badge className={className} variant={toneBadge[tone]}>
      {formatDeviation(driftPp)}
      <span className="ds-sr-only"> {toneText[tone]}</span>
    </Badge>
  );
}

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
          <DriftBadge
            className="ds-drift-row__badge"
            driftPp={row.driftPp}
            bandPp={bandPp}
          />
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

/** An asset nested under its class: weights now and at target, and its share of the class. */
function DriftAssetRow({
  row,
  scaleMax,
  bandPp,
  density,
}: {
  row: ClassDriftAsset;
  scaleMax: number;
  bandPp: number;
  density: "default" | "compact";
}) {
  return (
    <li
      className={cn(
        "ds-drift-row ds-drift-row--asset",
        `ds-drift-row--${density}`
      )}
      data-row-kind="asset"
    >
      <span className="ds-drift-row__identity">
        <strong>{row.ticker}</strong>
        <span>{row.name}</span>
        <span className="ds-drift-row__detail">
          {formatPercent(row.currentWeight)} now ·{" "}
          {formatPercent(row.targetWeight)} target ·{" "}
          {formatPercent(row.shareOfClass)} of class
        </span>
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
          <DriftBadge
            className="ds-drift-row__badge"
            driftPp={row.driftPp}
            bandPp={bandPp}
          />
        </>
      )}
    </li>
  );
}

/** One asset class against its class target, followed by its nested assets. */
function DriftClassGroup({
  group,
  scaleMax,
  bandPp,
  density,
}: {
  group: ClassDriftGroup;
  scaleMax: number;
  bandPp: number;
  density: "default" | "compact";
}) {
  const classOnly = collapsesToClassRow(group.assetClass, group.assets.length);
  const count = group.assets.length;
  const detail = classOnly
    ? `${group.assets[0]?.ticker ?? ""} · target ${formatPercent(group.targetWeight)}`
    : `${count} ${count === 1 ? "asset" : "assets"} · target ${formatPercent(group.targetWeight)}`;

  return (
    <li
      className="ds-drift-group"
      data-asset-class={group.assetClass}
      data-row-kind="class"
    >
      <div
        className={cn(
          "ds-drift-row ds-drift-row--class",
          `ds-drift-row--${density}`
        )}
      >
        <span className="ds-drift-row__identity">
          <strong>{group.label}</strong>
          <span>{detail}</span>
        </span>
        <AllocationBar
          name={`${group.label} class`}
          assetClass={group.assetClass}
          today={group.currentWeight}
          target={group.targetWeight}
          scaleMax={scaleMax}
        />
        {density === "compact" ? (
          <span className="ds-drift-row__summary">
            {formatPercent(group.currentWeight)} · target{" "}
            {formatPercent(group.targetWeight)}
          </span>
        ) : (
          <>
            <span className="ds-drift-row__weight">
              {formatPercent(group.currentWeight)}
            </span>
            <DriftBadge
              className="ds-drift-row__badge"
              driftPp={group.driftPp}
              bandPp={bandPp}
            />
          </>
        )}
      </div>
      {classOnly ? null : (
        <ul
          className="ds-drift-group__assets"
          aria-label={`${group.label} assets`}
        >
          {group.assets.map((row) => (
            <DriftAssetRow
              key={row.id}
              row={row}
              scaleMax={scaleMax}
              bandPp={bandPp}
              density={density}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

export interface DriftClassGroupListProps {
  rows: readonly DriftRowModel[];
  /** Stated Strategic allocation that overrides targets derived from the rows. */
  targets?: StrategicTargets;
  bandPp?: number;
  density?: "default" | "compact";
  /** Accessible name of the list. */
  label: string;
}

/**
 * The drift rows of a Strategic allocation: classes with the widest gap
 * first, each followed by its assets, all on one bar scale and judged by the
 * same Portfolio-wide Drift band.
 */
export function DriftClassGroupList({
  rows,
  targets,
  bandPp = DEFAULT_DRIFT_BAND_PP,
  density = "default",
  label,
}: DriftClassGroupListProps) {
  const groups = groupDriftByClass(rows, targets);
  const scaleMax = allocationBarScale(
    rows
      .flatMap((row) => [row.currentWeight, row.targetWeight])
      .concat(groups.flatMap((g) => [g.currentWeight, g.targetWeight]))
  );

  return (
    <ul
      className="ds-drift-row-list ds-drift-row-list--classes"
      aria-label={label}
    >
      {groups.map((group) => (
        <DriftClassGroup
          key={group.assetClass}
          group={group}
          scaleMax={scaleMax}
          bandPp={bandPp}
          density={density}
        />
      ))}
    </ul>
  );
}

/** Names the class targets of a Strategic allocation, such as "Equities 65% · Bonds 25%". */
export function describeClassTargets(
  rows: readonly DriftRowModel[],
  targets?: StrategicTargets
): string {
  const groups = [...groupDriftByClass(rows, targets)].sort(
    (a, b) => b.targetWeight - a.targetWeight
  );
  const listed = groups
    .map((group) => `${group.label} ${formatTargetWeight(group.targetWeight)}`)
    .join(" · ");
  if (!targets) return listed;
  // Stated targets can miss 100%; say so instead of presenting them as complete.
  const total = Object.values(targets.classTargets).reduce<number>(
    (sum, value) => sum + (value ?? 0),
    0
  );
  return Math.abs(total - 100) > TARGET_TOTAL_TOLERANCE_PP
    ? `${listed} (total ${formatTargetWeight(Number(total.toFixed(1)))}, incomplete)`
    : listed;
}

export interface DriftOverviewProps {
  /** How far the Portfolio currently sits from its Target weights. */
  snapshot: DriftSnapshot;
  /** Distance from target, in percentage points, beyond which a gap is flagged. */
  bandPp?: number;
  /**
   * The Strategic allocation the Invest gate checks. When set, class rows, the
   * header figures and the class-targets line use it instead of targets
   * derived from the snapshot rows.
   */
  targets?: StrategicTargets;
  /** Allocation mode: Strategic groups rows by asset class. Defaults to simple. */
  mode?: AllocationMode;
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
  targets,
  mode = "simple",
  title = "Current vs target",
  className,
}: DriftOverviewProps) {
  const strategic = mode === "strategic";
  const classGroups = strategic
    ? groupDriftByClass(snapshot.rows, targets)
    : [];
  const classGap = classLargestGap(classGroups);
  const { largestGap } = snapshot;

  return (
    <div className={cn("ds-drift-overview", className)}>
      <MetricStrip
        label="Portfolio drift summary"
        metrics={[
          {
            label: "Total drift",
            value: formatPoints(
              strategic ? classTotalDriftPp(classGroups) : snapshot.totalDriftPp
            ),
          },
          {
            label: "Largest gap",
            value: strategic
              ? classGap
                ? `${classGap.label} ${formatDeviation(classGap.driftPp)}`
                : "None"
              : largestGap
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
                {strategic
                  ? "Marker = target · biggest class gap first"
                  : "Marker = target · biggest gap first"}
              </span>
            </div>
            {strategic ? (
              <div className="ds-drift-overview__mode">
                <Badge variant="primary">Strategic allocation</Badge>
                <span>
                  Class targets: {describeClassTargets(snapshot.rows, targets)}
                </span>
              </div>
            ) : null}
          </div>
          {strategic ? (
            <DriftClassGroupList
              rows={snapshot.rows}
              targets={targets}
              bandPp={bandPp}
              label="Asset classes and assets compared with their targets"
            />
          ) : (
            <DriftRowList
              rows={snapshot.rows}
              bandPp={bandPp}
              label="Assets compared with their targets"
            />
          )}
        </CardContent>
      </Card>
    </div>
  );
}
