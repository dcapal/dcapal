import * as React from "react";
import { formatCurrency } from "../../fixtures/portfolio";
import { cn } from "../../lib/cn";
import { Badge, Card, CardContent } from "../ui";
import {
  formatDeviation,
  formatPoints,
  formatTargetWeight,
} from "./invest-format";
import type {
  AllocationMode,
  DriftRowModel,
  DriftSnapshot,
} from "./invest-types";
import { MetricStrip } from "./metric-card";
import {
  classLargestGap,
  classTotalDriftPp,
  driftClassGroups,
  useDriftAssets,
} from "./drift-assets";
import { PortfolioAssetCollection } from "./portfolio-asset-collection";
import {
  largestGapPortfolioAssetSort,
  type PortfolioAssetClassGroup,
  type StrategicTargets,
} from "./portfolio-asset-groups";
import { TARGET_TOTAL_TOLERANCE_PP } from "./invest-targets";

/** Names and subtitles a row: cash is named by its asset class, others by ticker. */
function rowLabels(row: DriftRowModel): { title: string; subtitle: string } {
  return row.assetClass === "cash"
    ? { title: row.name, subtitle: row.ticker }
    : { title: row.ticker, subtitle: row.name };
}

/** Names the class targets of a Strategic allocation, such as "Equities 65% · Bonds 25%". */
export function describeClassTargets(
  classGroups: readonly PortfolioAssetClassGroup[],
  targets?: StrategicTargets
): string {
  const groups = [...classGroups].sort(
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
  // The shared collection lists the holdings; the snapshot only supplies weights.
  const assets = useDriftAssets(snapshot);
  const classGroups = React.useMemo(
    () => (strategic ? driftClassGroups(assets, targets) : []),
    [strategic, assets, targets]
  );
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
          {strategic ? (
            <div className="ds-drift-overview__mode">
              <Badge variant="primary">Strategic allocation</Badge>
              <span>
                Class targets: {describeClassTargets(classGroups, targets)}
              </span>
            </div>
          ) : null}
          <PortfolioAssetCollection
            assets={assets}
            presentation="overview"
            mode={mode}
            showModeToggle={false}
            title={title}
            subtitle={
              strategic
                ? "Biggest class gap first · target and drift per row"
                : "Biggest gap first · target and drift per row"
            }
            targets={strategic ? targets : undefined}
            driftBandPp={bandPp}
            defaultSort={largestGapPortfolioAssetSort}
            surface="invest-drift"
          />
        </CardContent>
      </Card>
    </div>
  );
}
