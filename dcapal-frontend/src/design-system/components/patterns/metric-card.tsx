import { Card, CardContent } from "../ui";
import type { PortfolioMetric } from "../../fixtures/portfolio";
import { cn } from "../../lib/cn";

export interface MetricCardProps {
  metric: PortfolioMetric;
  className?: string;
  density?: "default" | "compact";
}

export function MetricCard({
  metric,
  className,
  density = "default",
}: MetricCardProps) {
  return (
    <Card
      className={cn("ds-metric-card", `ds-metric-card--${density}`, className)}
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
