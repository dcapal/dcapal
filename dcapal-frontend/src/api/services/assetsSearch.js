import axios from "axios";
import { api } from "../httpClient";
import { ACLASS } from "@components/allocationFlow/portfolioSlice";

export const fetchAssetsYF = async (query) => {
  const url = `/assets/search?name=${query}`;
  try {
    const response = await api.get(url);

    if (response.status !== 200) {
      console.error(
        `Response {status: ${response.status}, data: ${response.data}}`
      );
      return [];
    }

    return response.data.quotes
      .filter((quote) => {
        const type = quote.quoteType.toUpperCase();
        return type === "EQUITY" || type === "ETF" || type === "MUTUALFUND";
      })
      .map((quote) => ({
        name: quote.longname || quote.shortname || "",
        symbol: quote.symbol,
        type: quote.quoteType,
        exchange: quote.exchange,
        aclass: ACLASS.EQUITY,
      }));
  } catch (error) {
    if (!axios.isCancel(error)) {
      console.error(error);
    }
    return [];
  }
};
