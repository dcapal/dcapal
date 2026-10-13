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
            metric.trend === "positive" && "ds-metric-card__value--positive",
            metric.trend === "negative" && "ds-metric-card__value--negative"
          )}
        >
          {metric.value}
        </div>
        {metric.context ? (
          <div className="ds-metric-card__context">{metric.context}</div>
        ) : null}
      </CardContent>
    </Card>
  );
}
