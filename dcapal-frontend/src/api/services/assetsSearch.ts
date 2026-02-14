import axios from "axios";
import { api } from "../httpClient";
import { ACLASS } from "@components/allocationFlow/portfolioSlice";

interface YahooSearchQuote {
  quoteType?: string;
  longname?: string;
  shortname?: string;
  symbol?: string;
  exchange?: string;
}

interface YahooSearchResponse {
  quotes?: YahooSearchQuote[];
}

export interface SearchAsset {
  name: string;
  symbol: string;
  type: string;
  exchange: string;
  aclass: number;
}

export const fetchAssetsYF = async (query: string): Promise<SearchAsset[]> => {
  const url = `/assets/search?name=${query}`;
  try {
    const response = await api.get<YahooSearchResponse>(url);

    if (response.status !== 200) {
      console.error(
        `Response {status: ${response.status}, data: ${response.data}}`
      );
      return [];
    }

    const quotes = response.data.quotes;
    if (!Array.isArray(quotes)) return [];

    return quotes
      .filter((quote) => {
        const type = quote.quoteType?.toUpperCase();
        return type === "EQUITY" || type === "ETF" || type === "MUTUALFUND";
      })
      .map((quote) => ({
        name: quote.longname || quote.shortname || "",
        symbol: quote.symbol || "",
        type: quote.quoteType || "",
        exchange: quote.exchange || "",
        aclass: ACLASS.EQUITY,
      }));
  } catch (error) {
    if (!axios.isCancel(error)) {
      console.error(error);
    }
    return [];
  }
};
