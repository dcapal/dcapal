import * as React from "react";
import { Card, CardContent } from "../ui";
import type { PortfolioMetric } from "../../fixtures/portfolio";
import { cn } from "../../lib/cn";

export interface MetricCardProps {
  metric: PortfolioMetric;
  className?: string;
  density?: "default" | "compact";
  /** Renders an informative not-yet-available state instead of a live value. */
  placeholder?: boolean;
}

export function MetricCard({
  metric,
  className,
  density = "default",
  placeholder = false,
}: MetricCardProps) {
  // A "before → after" value may wrap at the arrow, never inside a figure.
  const parts = metric.value.split(/\s*→\s*/).filter(Boolean);
  const compoundParts = parts.length > 1 ? parts : null;

  return (
    <Card
      className={cn(
        "ds-metric-card",
        `ds-metric-card--${density}`,
        placeholder && "ds-metric-card--placeholder",
        className
      )}
      data-placeholder={placeholder || undefined}
    >
      <CardContent>
        <div className="ds-metric-card__label">{metric.label}</div>
        <div
          className={cn(
            "ds-metric-card__value",
            compoundParts && "ds-metric-card__value--compound",
            metric.trend === "positive" && "ds-metric-card__value--positive",
            metric.trend === "negative" && "ds-metric-card__value--negative"
          )}
        >
          {compoundParts
            ? compoundParts.map((part, index) => (
                <React.Fragment key={part}>
                  {index > 0 ? " " : null}
                  <span className="ds-metric-card__part">
                    {index > 0 ? "→ " : null}
                    {part}
                  </span>
                </React.Fragment>
              ))
            : metric.value}
        </div>
        {metric.context ? (
          <div className="ds-metric-card__context">{metric.context}</div>
        ) : null}
      </CardContent>
    </Card>
  );
}

export interface MetricStripProps {
  /** The summary figures to show side by side. */
  metrics: readonly PortfolioMetric[];
  /** Accessible name announced for the group of figures. */
  label?: string;
  className?: string;
}

/** A row of compact MetricCards that fills the available width and wraps to fewer columns as its container narrows. */
export function MetricStrip({ metrics, label, className }: MetricStripProps) {
  return (
    <div
      role="group"
      aria-label={label}
      className={cn("ds-metric-strip", className)}
    >
      {metrics.map((metric) => (
        <MetricCard key={metric.label} metric={metric} density="compact" />
      ))}
    </div>
  );
}
