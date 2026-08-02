import { useQuery } from "@tanstack/react-query";
import type { QueryFunctionContext } from "@tanstack/react-query";
import {
  getAssetsChart,
  getGetPriceQueryKey,
  getPrice,
} from "@dcapal/api-client";

import { queryClient, PRICE_STALE_TIME } from "./queryClient";

/** Identifies the backend or market-data provider used for an asset price. */
export const Provider = Object.freeze({
  DCA_PAL: "DCAPal",
  YF: "YF",
});

/** Classifies the recoverable failures returned by price-provider adapters. */
export const FetchError = Object.freeze({
  BAD_DATA: "BAD_DATA",
  REQUEST_CANCELED: "REQUEST_CANCELED",
});

/** The provider values accepted by the frontend price adapters. */
export type Provider = (typeof Provider)[keyof typeof Provider];

/** The fetch-error values understood by the frontend price adapters. */
export type FetchError = (typeof FetchError)[keyof typeof FetchError];

type PriceRequest = {
  symbol: string;
  quote: string;
  signal?: AbortSignal;
};

type YahooPriceRequest = PriceRequest & {
  validCcys: string[];
};

type YahooPrice = {
  price: number;
  baseCcy: string;
};

type PriceQueryContext = Pick<QueryFunctionContext, "signal">;

const toUnixTimestamp = (date: Date, startOfDay: boolean) => {
  const value = new Date(date.getTime());
  if (startOfDay) value.setUTCHours(0, 0, 0, 0);
  return Math.floor(value.getTime() / 1000);
};

const getLastFourDays = () => {
  // Yahoo's chart endpoint needs a window because the latest close is not
  // guaranteed to be present at the current timestamp.
  const date = new Date();
  date.setDate(date.getDate() - 4);
  return {
    startPeriod: toUnixTimestamp(date, true),
    endPeriod: toUnixTimestamp(new Date(), false),
  };
};

const isValidClosePrice = (value: number | null): value is number =>
  typeof value === "number" && Number.isFinite(value) && value !== 0;

/** Fetches an asset price directly from the DCA-Pal API. */
export const fetchDcaPalPrice = async ({
  symbol,
  quote,
  signal,
}: PriceRequest): Promise<number> => {
  const response = await getPrice(symbol, { quote }, { signal });
  return response.data.price;
};

/** Fetches a Yahoo close price and converts it to the requested quote currency. */
export const fetchYahooPrice = async ({
  symbol,
  quote,
  validCcys,
  signal,
}: YahooPriceRequest): Promise<YahooPrice> => {
  const response = await getAssetsChart(symbol, getLastFourDays(), { signal });
  const chart = response.data.chart;
  const result = chart?.result?.[0];
  const baseCcy = result?.meta?.currency?.toLowerCase();
  const supportedCurrencies = validCcys.map((ccy) => ccy.toLowerCase());

  if (!result || !baseCcy || !supportedCurrencies.includes(baseCcy)) {
    throw new Error(FetchError.BAD_DATA);
  }

  const closePrices = result.indicators?.quote?.[0]?.close;
  const price = Array.isArray(closePrices)
    ? [...closePrices].reverse().find(isValidClosePrice)
    : undefined;

  if (price === undefined) {
    throw new Error(FetchError.BAD_DATA);
  }

  if (baseCcy === quote.toLowerCase()) {
    return { price, baseCcy };
  }

  const conversion = await getPrice(baseCcy, { quote }, { signal });
  return {
    price: price * conversion.data.price,
    baseCcy,
  };
};

/** Builds the cache key for a Yahoo price request. */
export const getYahooPriceQueryKey = (
  symbol: string,
  quote: string,
  validCcys: string[]
) => ["price-provider", Provider.YF, symbol, quote, [...validCcys].sort()];

/** Reads a Yahoo price through the shared TanStack Query cache. */
export const useYahooPrice = ({
  symbol,
  quote,
  validCcys,
  enabled = true,
}: YahooPriceRequest & { enabled?: boolean }) =>
  useQuery<YahooPrice>({
    queryKey: getYahooPriceQueryKey(symbol, quote, validCcys),
    queryFn: ({ signal }: PriceQueryContext) =>
      fetchYahooPrice({ symbol, quote, validCcys, signal }),
    enabled:
      enabled &&
      Boolean(symbol) &&
      Boolean(quote) &&
      Array.isArray(validCcys) &&
      validCcys.length > 0,
    staleTime: PRICE_STALE_TIME,
  });

/** Reads or fetches a DCA-Pal price through the shared query cache. */
export const getDcaPalPrice = (symbol: string, quote: string) =>
  queryClient.fetchQuery({
    queryKey: getGetPriceQueryKey(symbol, { quote }),
    queryFn: ({ signal }: PriceQueryContext) =>
      fetchDcaPalPrice({ symbol, quote, signal }),
    staleTime: PRICE_STALE_TIME,
  });

/** Reads or fetches a converted Yahoo price through the shared query cache. */
export const getYahooPrice = (
  symbol: string,
  quote: string,
  validCcys: string[]
) =>
  queryClient.fetchQuery({
    queryKey: getYahooPriceQueryKey(symbol, quote, validCcys),
    queryFn: ({ signal }: PriceQueryContext) =>
      fetchYahooPrice({ symbol, quote, validCcys, signal }),
    staleTime: PRICE_STALE_TIME,
  });

/** Resolves a provider price and turns unusable provider data into `null`. */
export const getPriceForProvider = async (
  provider: Provider,
  validCcys: string[],
  symbol: string,
  quote: string
): Promise<number | null> => {
  try {
    if (provider === Provider.DCA_PAL) {
      return await getDcaPalPrice(symbol, quote);
    }

    const result = await getYahooPrice(symbol, quote, validCcys);
    return result.price;
  } catch (error: unknown) {
    if (error instanceof Error && error.message === FetchError.BAD_DATA) {
      return null;
    }
    console.error(error);
    return null;
  }
};
