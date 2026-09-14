export type AssetClass =
  "equities" | "bonds" | "cash" | "crypto" | "commodities" | "other";
export type DriftStatus = "under" | "within" | "over";

export interface PortfolioAsset {
  id: string;
  name: string;
  ticker: string;
  assetClass: AssetClass;
  quantity: number;
  price: number;
  value: number;
  targetWeight: number;
  currentWeight: number;
  driftPp: number;
  /** Deterministic average purchase price used by the presentation stories. */
  averageCostBasis?: number;
  currency?: string;
  /** Human-readable region or denomination used by the presentation layer. */
  region?: string;
}

export interface PortfolioMetric {
  label: string;
  value: string;
  context?: string;
  trend?: "positive" | "negative" | "neutral";
}

export interface ChartPoint {
  period: string;
  total: number;
  equities: number;
  bonds: number;
  cash: number;
  crypto: number;
}

/** Labels used in the investor-facing portfolio surfaces. */
export const assetClassLabels: Record<AssetClass, string> = {
  equities: "Equities",
  bonds: "Bonds",
  cash: "Cash",
  crypto: "Crypto",
  commodities: "Commodities",
  other: "Other",
};

/** Builds the compact asset-class label shown in cards and tables. */
export function formatAssetClass(
  asset: Pick<PortfolioAsset, "assetClass" | "currency" | "region">
): string {
  if (asset.assetClass === "cash") {
    return `${assetClassLabels.cash} · ${asset.currency ?? "EUR"}`;
  }

  return `${assetClassLabels[asset.assetClass]} · ${asset.region ?? "Global"}`;
}

/** Formats quote-currency values consistently across responsive presentations. */
export function formatCurrency(value: number, currency = "EUR"): string {
  return new Intl.NumberFormat("en-IE", {
    style: "currency",
    currency,
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(value);
}

/** Formats percentages with tabular, presentation-ready precision. */
export function formatPercent(value: number): string {
  return `${value.toFixed(1)}%`;
}

/** Formats drift in percentage points while retaining the sign for positive values. */
export function formatDrift(value: number): string {
  return `${value > 0 ? "+" : ""}${value.toFixed(1)} pp`;
}

export const portfolioAssets: PortfolioAsset[] = [
  {
    id: "vwce",
    name: "Vanguard FTSE All-World UCITS ETF",
    ticker: "VWCE",
    assetClass: "equities",
    quantity: 28.43,
    price: 123.84,
    value: 3522.77,
    targetWeight: 30,
    currentWeight: 34.8,
    driftPp: 4.8,
    averageCostBasis: 97.37,
    currency: "EUR",
    region: "Global",
  },
  {
    id: "aggh",
    name: "iShares Core Global Aggregate Bond ETF",
    ticker: "AGGH",
    assetClass: "bonds",
    quantity: 41.2,
    price: 5.12,
    value: 210.94,
    targetWeight: 25,
    currentWeight: 24.1,
    driftPp: -0.9,
    averageCostBasis: 4.85,
    currency: "EUR",
    region: "Global",
  },
  {
    id: "ieur",
    name: "iShares MSCI Europe ETF",
    ticker: "IEUR",
    assetClass: "equities",
    quantity: 15.6,
    price: 94.35,
    value: 1471.86,
    targetWeight: 20,
    currentWeight: 15,
    driftPp: -5,
    averageCostBasis: 89.22,
    currency: "EUR",
    region: "Europe",
  },
  {
    id: "vig",
    name: "Vanguard US Dividend Appreciation ETF",
    ticker: "VIG",
    assetClass: "equities",
    quantity: 10.25,
    price: 173.25,
    value: 1775.81,
    targetWeight: 15,
    currentWeight: 14.1,
    driftPp: -0.9,
    averageCostBasis: 165.7,
    currency: "EUR",
    region: "USA",
  },
  {
    id: "cash",
    name: "Cash",
    ticker: "EUR",
    assetClass: "cash",
    quantity: 0,
    price: 1,
    value: 3842.15,
    targetWeight: 10,
    currentWeight: 11.9,
    driftPp: 1.9,
    currency: "EUR",
    region: "EUR",
  },
];

const chartMonths = [
  "Jan",
  "Feb",
  "Mar",
  "Apr",
  "May",
  "Jun",
  "Jul",
  "Aug",
  "Sep",
  "Oct",
  "Nov",
  "Dec",
] as const;

/** Stable five-year history used by the presentational chart stories. */
export const chartFixture: ChartPoint[] = Array.from(
  { length: 60 },
  (_, index) => {
    const date = new Date(Date.UTC(2021, 8 + index, 1));
    const equities = 31 + Math.round(index * 0.47);
    const bonds = 19 + Math.round(index * 0.22);
    const cash = 12 + Math.round(index * 0.15);
    const crypto = 10 + Math.round(index * 0.1);

    return {
      period: `${chartMonths[date.getUTCMonth()]} '${String(
        date.getUTCFullYear()
      ).slice(-2)}`,
      total: equities + bonds + cash + crypto,
      equities,
      bonds,
      cash,
      crypto,
    };
  }
);
