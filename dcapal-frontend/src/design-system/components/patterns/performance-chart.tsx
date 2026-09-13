import * as React from "react";
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip as ChartTooltip,
  XAxis,
  YAxis,
} from "recharts";
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  EmptyState,
  Skeleton,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Tabs,
  TabsList,
  TabsTrigger,
} from "../ui";
import { chartFixture, type ChartPoint } from "../../fixtures/portfolio";
import { formatCurrency } from "../../fixtures/portfolio";
import { cn } from "../../lib/cn";
import { Info } from "lucide-react";

export type ChartRange = "1Y" | "3Y" | "5Y" | "Max";
export type PerformanceChartState = "ready" | "loading" | "empty" | "gated";
export interface PerformanceChartProps {
  state?: PerformanceChartState;
  data?: readonly ChartPoint[];
  initialRange?: ChartRange;
  onRangeChange?: (range: ChartRange) => void;
  /** Quote currency label shown in the chart control. */
  currency?: "EUR" | "USD" | "GBP";
  /** Optional callback for a controlled quote-currency selector. */
  onCurrencyChange?: (currency: "EUR" | "USD" | "GBP") => void;
  className?: string;
}

/** A deterministic chart presentation. It never fetches data or calculates portfolio values. */
export function PerformanceChart({
  state = "ready",
  data = chartFixture,
  initialRange = "1Y",
  onRangeChange,
  currency = "EUR",
  onCurrencyChange,
  className,
}: PerformanceChartProps) {
  const [range, setRange] = React.useState<ChartRange>(initialRange);
  const [selectedCurrency, setSelectedCurrency] = React.useState(currency);
  React.useEffect(() => {
    setSelectedCurrency(currency);
  }, [currency]);
  const ranges: ChartRange[] = ["1Y", "3Y", "5Y", "Max"];
  const rangePointCount: Record<ChartRange, number> = {
    "1Y": 12,
    "3Y": 36,
    "5Y": 60,
    Max: data.length,
  };
  const visibleData = data.slice(-rangePointCount[range]);
  const chartCurrencyPrefix =
    selectedCurrency === "EUR" ? "€" : selectedCurrency === "GBP" ? "£" : "$";
  const changeRange = (next: ChartRange) => {
    setRange(next);
    onRangeChange?.(next);
  };
  return (
    <Card className={cn("ds-chart", className)}>
      <CardHeader>
        <div className="ds-chart__header">
          <div className="ds-chart__title-group">
            <CardTitle>Portfolio value over time</CardTitle>
            <button
              type="button"
              className="ds-chart__info"
              title="Historical portfolio value grouped by individual asset."
              aria-label="Historical portfolio value grouped by individual asset"
            >
              <Info size={14} aria-hidden />
            </button>
          </div>
          <Select
            value={selectedCurrency}
            onValueChange={(value) => {
              if (value === "EUR" || value === "USD" || value === "GBP") {
                setSelectedCurrency(value);
                onCurrencyChange?.(value);
              }
            }}
          >
            <SelectTrigger
              className="ds-chart__currency"
              aria-label="Display currency"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent className="ds-chart__currency-popup">
              <SelectItem value="EUR">EUR</SelectItem>
              <SelectItem value="USD">USD</SelectItem>
              <SelectItem value="GBP">GBP</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </CardHeader>
      <CardContent>
        {state === "loading" ? (
          <div aria-label="Loading chart">
            <Skeleton className="ds-chart__skeleton" />
            <Skeleton className="ds-chart__skeleton" />
          </div>
        ) : state === "empty" ? (
          <EmptyState
            title="No performance data yet"
            description="Add an asset to start tracking portfolio value."
          />
        ) : state === "gated" ? (
          <EmptyState
            title="Sign in to view history"
            description="Historical analytics are available after registration."
            action={<Button variant="primary">Create account</Button>}
          />
        ) : (
          <div
            aria-label={`Portfolio performance chart for ${range}`}
            className="ds-chart__plot-wrap"
          >
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={visibleData}>
                <CartesianGrid stroke="var(--dcapal-border)" vertical={false} />
                <XAxis
                  dataKey="period"
                  tick={{
                    fill: "var(--dcapal-foreground-muted)",
                  }}
                />
                <YAxis
                  tick={{
                    fill: "var(--dcapal-foreground-muted)",
                  }}
                  width={52}
                  tickFormatter={(value) => `${chartCurrencyPrefix}${value}k`}
                />
                <ChartTooltip />
                <Area
                  type="monotone"
                  dataKey="equities"
                  stackId="1"
                  stroke="var(--dcapal-asset-equities)"
                  fill="var(--dcapal-asset-equities)"
                  fillOpacity={0.72}
                />
                <Area
                  type="monotone"
                  dataKey="bonds"
                  stackId="1"
                  stroke="var(--dcapal-asset-bonds)"
                  fill="var(--dcapal-asset-bonds)"
                  fillOpacity={0.72}
                />
                <Area
                  type="monotone"
                  dataKey="cash"
                  stackId="1"
                  stroke="var(--dcapal-asset-cash)"
                  fill="var(--dcapal-asset-cash)"
                  fillOpacity={0.72}
                />
                <Area
                  type="monotone"
                  dataKey="crypto"
                  stackId="1"
                  stroke="var(--dcapal-asset-crypto)"
                  fill="var(--dcapal-asset-crypto)"
                  fillOpacity={0.72}
                />
              </AreaChart>
            </ResponsiveContainer>
            <p className="ds-chart__summary ds-sr-only">
              Showing {visibleData.length} months. Latest value:{" "}
              {formatCurrency(
                (visibleData[visibleData.length - 1]?.total ?? 0) * 1000,
                selectedCurrency
              )}
              .
            </p>
            <table className="ds-sr-only">
              <caption>Portfolio performance data for {range}</caption>
              <thead>
                <tr>
                  <th scope="col">Period</th>
                  <th scope="col">Total value</th>
                </tr>
              </thead>
              <tbody>
                {visibleData.map((point) => (
                  <tr key={point.period}>
                    <td>{point.period}</td>
                    <td>
                      {formatCurrency(point.total * 1000, selectedCurrency)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {state === "ready" ? (
          <Tabs
            className="ds-chart__ranges"
            value={range}
            onValueChange={(value) => changeRange(value as ChartRange)}
          >
            <TabsList aria-label="Chart range">
              {ranges.map((option) => (
                <TabsTrigger key={option} value={option}>
                  {option}
                </TabsTrigger>
              ))}
            </TabsList>
          </Tabs>
        ) : null}
      </CardContent>
    </Card>
  );
}
