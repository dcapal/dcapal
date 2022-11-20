import axios from "axios";
import { api } from "./api";

export const Provider = Object.freeze({
  DCA_PAL: "DCAPal",
  YF: "YF",
});

export const fetchPrice = async (base, quote, token) => {
  const url = `/api/dcapal/price/${base}?quote=${quote}`;
  try {
    const response = await api.get(url, { cancelToken: token });

    if (response.status != 200) {
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

const toUnixTimestamp = (date) => {
  const d = new Date(date.getTime());
  d.setUTCHours(0, 0, 0, 0);
  return Math.floor(d.getTime() / 1000);
};

export const fetchPriceYF = async (symbol, quote, token) => {
  const lastThreeDays = new Date();
  lastThreeDays.setDate(lastThreeDays.getDate() - 3);
  const period1 = toUnixTimestamp(lastThreeDays);
  const period2 = toUnixTimestamp(new Date());
  const url = `/api/yfinance1/v8/finance/chart/${symbol}?interval=5m&period1=${period1}&period2=${period2}&close=adjusted`;
  try {
    const response = await api.get(url, {
      cancelToken: token,
    });

    if (response.status != 200) {
      console.error(
        `Response {status: ${response.status}, data: ${response.data}`
      );
      return null;
    }

    // On error, log and exit
    if (!response.data.chart || response.data.chart?.error) {
      console.error(response.data.chart?.error);
      return null;
    }

    const result = response.data.chart.result;
    if (!Array.isArray(result) || result.length < 1) {
      console.error("Empty YF price result:", response.data, url);
      return null;
    }

    const base = result[0].meta?.currency?.toLowerCase();
    if (!base) {
      console.error("Missing base currency:", response.data, url);
      return null;
    }

    const quotes = result[0].indicators?.quote;
    if (!Array.isArray(quotes) || quotes.length < 1) {
      console.error("Empty quotes:", response.data, url);
      return null;
    }

    const closePrices = quotes[0].close;
    if (!Array.isArray(closePrices) || closePrices.length < 1) {
      console.error("Empty close prices:", response.data, url);
      return null;
    }

    const price = closePrices
      .slice()
      .reverse()
      .find((p) => p);

    if (base === quote) {
      return [price, base];
    } else {
      const rate = await fetchPrice(base, quote, token);
      return [price * rate, base];
    }
  } catch (error) {
    if (!axios.isCancel(error)) {
      console.error(error);
    }
    return null;
  }
};

export const getFetcher = (provider) => {
  if (provider === Provider.DCA_PAL) {
    return async (symbol, quote) => {
      return await fetchPrice(symbol, quote, null);
    };
  }

  return async (symbol, quote) => {
    const p = await fetchPriceYF(symbol, quote, null);
    if (p) {
      const [px] = p;
      return px;
    }
    return null;
  };
};

export const fetchAssetsDcaPal = async (type) => {
  const url = `/api/dcapal/assets/${type}`;
  try {
    const response = await api.get(url);

    if (response.status != 200) {
      console.error(
        `Response {status: ${response.status}, data: ${response.data}`
      );
      return [];
    }

    return response.data.map((asset) => ({
      symbol: asset.id,
      name: asset.symbol,
    }));
  } catch (error) {
    if (!axios.isCancel(error)) {
      console.error(error);
    }
    return [];
  }
};
