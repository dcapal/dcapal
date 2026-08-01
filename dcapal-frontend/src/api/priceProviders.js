import { useQuery } from "@tanstack/react-query";
import {
  getAssetsChart,
  getGetPriceQueryKey,
  getPrice,
} from "@dcapal/api-client";

import { queryClient, PRICE_STALE_TIME } from "./queryClient";

export const Provider = Object.freeze({
  DCA_PAL: "DCAPal",
  YF: "YF",
});

export const FetchError = Object.freeze({
  BAD_DATA: "BAD_DATA",
  REQUEST_CANCELED: "REQUEST_CANCELED",
});

const toUnixTimestamp = (date, startOfDay) => {
  const value = new Date(date.getTime());
  if (startOfDay) value.setUTCHours(0, 0, 0, 0);
  return Math.floor(value.getTime() / 1000);
};

const getLastFourDays = () => {
  const date = new Date();
  date.setDate(date.getDate() - 4);
  return {
    startPeriod: toUnixTimestamp(date, true),
    endPeriod: toUnixTimestamp(new Date(), false),
  };
};

const isValidClosePrice = (value) =>
  typeof value === "number" && Number.isFinite(value) && value !== 0;

export const fetchDcaPalPrice = async ({ symbol, quote, signal }) => {
  const response = await getPrice(symbol, { quote }, { signal });
  return response.data.price;
};

export const fetchYahooPrice = async ({ symbol, quote, validCcys, signal }) => {
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

export const getYahooPriceQueryKey = (symbol, quote, validCcys) => [
  "price-provider",
  Provider.YF,
  symbol,
  quote,
  [...validCcys].sort(),
];

export const useYahooPrice = ({ symbol, quote, validCcys, enabled = true }) =>
  useQuery({
    queryKey: getYahooPriceQueryKey(symbol, quote, validCcys),
    queryFn: ({ signal }) =>
      fetchYahooPrice({ symbol, quote, validCcys, signal }),
    enabled:
      enabled &&
      Boolean(symbol) &&
      Boolean(quote) &&
      Array.isArray(validCcys) &&
      validCcys.length > 0,
    staleTime: PRICE_STALE_TIME,
  });

export const getDcaPalPrice = (symbol, quote) =>
  queryClient.fetchQuery({
    queryKey: getGetPriceQueryKey(symbol, { quote }),
    queryFn: ({ signal }) => fetchDcaPalPrice({ symbol, quote, signal }),
    staleTime: PRICE_STALE_TIME,
  });

export const getYahooPrice = (symbol, quote, validCcys) =>
  queryClient.fetchQuery({
    queryKey: getYahooPriceQueryKey(symbol, quote, validCcys),
    queryFn: ({ signal }) =>
      fetchYahooPrice({ symbol, quote, validCcys, signal }),
    staleTime: PRICE_STALE_TIME,
  });

export const getPriceForProvider = async (
  provider,
  validCcys,
  symbol,
  quote
) => {
  try {
    if (provider === Provider.DCA_PAL) {
      return await getDcaPalPrice(symbol, quote);
    }

    const result = await getYahooPrice(symbol, quote, validCcys);
    return result.price;
  } catch (error) {
    if (error?.message === FetchError.BAD_DATA) return null;
    console.error(error);
    return null;
  }
};
