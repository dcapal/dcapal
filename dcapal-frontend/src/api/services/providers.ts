import axios, { type CancelToken } from "axios";
import { api } from "../httpClient";
import { ACLASS } from "@components/allocationFlow/portfolioSlice";

export const Provider = Object.freeze({
  DCA_PAL: "DCAPal",
  YF: "YF",
} as const);

export type ProviderType = (typeof Provider)[keyof typeof Provider];

export const FetchError = Object.freeze({
  BAD_DATA: "BAD_DATA",
  REQUEST_CANCELED: "REQUEST_CANCELED",
} as const);

export type FetchErrorType = (typeof FetchError)[keyof typeof FetchError];

type AssetType = "fiat" | "crypto";

type Fetcher = (symbol: string, quote: string) => Promise<number | null>;
type PriceResult = [number, string];

interface PriceResponse {
  price: number;
}

interface YahooChartQuote {
  close?: Array<number | null>;
}

interface YahooChartResult {
  meta?: {
    currency?: string;
  };
  indicators?: {
    quote?: YahooChartQuote[];
  };
}

interface YahooChartResponse {
  chart?: {
    error?: unknown;
    result?: YahooChartResult[];
  };
}

interface DcaPalAssetResponse {
  id: string;
  symbol: string;
}

export interface DcaPalAsset {
  symbol: string;
  name: string;
  aclass: number;
}

const isFetchError = (value: unknown): value is FetchErrorType => {
  return (
    typeof value === "string" &&
    (value === FetchError.BAD_DATA || value === FetchError.REQUEST_CANCELED)
  );
};

export const fetchPrice = async (
  base: string,
  quote: string,
  token: CancelToken | null
): Promise<number | null> => {
  const url = `/price/${base}?quote=${quote}`;
  try {
    const response = await api.get<PriceResponse>(url, {
      cancelToken: token ?? undefined,
    });

    if (response.status !== 200) {
      console.error(
        `Response {status: ${response.status}, data: ${response.data}`
      );
      return null;
    }

    return response.data.price;
  } catch (error) {
    if (!axios.isCancel(error)) {
      console.error(error);
    }
    return null;
  }
};

const toUnixTimestamp = (date: Date, startOfDay: boolean): number => {
  const d = new Date(date.getTime());
  if (startOfDay) {
    d.setUTCHours(0, 0, 0, 0);
  }
  return Math.floor(d.getTime() / 1000);
};

export const fetchPriceYF = async (
  symbol: string,
  quote: string,
  validCcys: string[],
  token: CancelToken | null
): Promise<PriceResult | FetchErrorType | null> => {
  const lastFourDays = new Date();
  lastFourDays.setDate(lastFourDays.getDate() - 4);
  const period1 = toUnixTimestamp(lastFourDays, true);
  const period2 = toUnixTimestamp(new Date(), false);
  const url = `/assets/chart/${symbol}?startPeriod=${period1}&endPeriod=${period2}`;

  try {
    const response = await api.get<YahooChartResponse>(url, {
      cancelToken: token ?? undefined,
    });

    if (response.status !== 200) {
      console.error(
        `Response {status: ${response.status}, data: ${response.data}}`
      );
      return null;
    }

    if (!response.data.chart || response.data.chart.error) {
      console.error(response.data.chart?.error);
      return FetchError.BAD_DATA;
    }

    const result = response.data.chart.result;
    if (!Array.isArray(result) || result.length < 1) {
      console.error("Empty YF price result:", response.data, url);
      return FetchError.BAD_DATA;
    }

    const base = result[0].meta?.currency?.toLowerCase();
    if (!base) {
      console.error("Missing base currency:", response.data, url);
      return FetchError.BAD_DATA;
    }

    const isValidCcy = validCcys.find((ccy) => ccy === base);
    if (!isValidCcy) {
      console.warn("Unsupported currency:", response.data, url, validCcys);
      return FetchError.BAD_DATA;
    }

    const quotes = result[0].indicators?.quote;
    if (!Array.isArray(quotes) || quotes.length < 1) {
      console.error("Empty quotes:", response.data, url);
      return FetchError.BAD_DATA;
    }

    const closePrices = quotes[0].close;
    if (!Array.isArray(closePrices) || closePrices.length < 1) {
      console.error("Empty close prices:", response.data, url);
      return FetchError.BAD_DATA;
    }

    const price = closePrices
      .slice()
      .reverse()
      .find((p): p is number => Boolean(p));
    if (!price) {
      console.error("Missing valid close price:", response.data, url);
      return FetchError.BAD_DATA;
    }

    if (base === quote) {
      return [price, base];
    }

    const rate = await fetchPrice(base, quote, token);
    if (rate == null) return null;
    return [price * rate, base];
  } catch (error) {
    if (!axios.isCancel(error)) {
      console.error(error);
    }
    return FetchError.REQUEST_CANCELED;
  }
};

export const getFetcher = (
  provider: ProviderType,
  validCcys: string[]
): Fetcher => {
  if (provider === Provider.DCA_PAL) {
    return async (symbol: string, quote: string) => {
      return await fetchPrice(symbol, quote, null);
    };
  }

  return async (symbol: string, quote: string) => {
    const p = await fetchPriceYF(symbol, quote, validCcys, null);
    if (Array.isArray(p) && !isFetchError(p)) {
      const [px] = p;
      return px;
    }
    return null;
  };
};

export const fetchAssetsDcaPal = async (
  type: AssetType
): Promise<DcaPalAsset[]> => {
  const url = `/assets/${type}`;
  try {
    const response = await api.get<DcaPalAssetResponse[]>(url);

    if (response.status !== 200) {
      console.error(
        `Response {status: ${response.status}, data: ${response.data}`
      );
      return [];
    }

    const aclass = type === "fiat" ? ACLASS.CURRENCY : ACLASS.CRYPTO;

    return response.data.map((asset) => ({
      symbol: asset.id,
      name: asset.symbol,
      aclass,
    }));
  } catch (error) {
    if (!axios.isCancel(error)) {
      console.error(error);
    }
    return [];
  }
};
