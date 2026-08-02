import React, { useCallback, useEffect, useMemo, useState } from "react";
import type { ChangeEvent } from "react";
import Fuse from "fuse.js";
import type { IFuseOptions } from "fuse.js";
import { useSelector } from "react-redux";
import {
  useGetAssetsData,
  useGetAssetsFiat,
  useGetAssetsCrypto,
  useGetPrice,
} from "@dcapal/api-client";

import { Spinner } from "@components/spinner/spinner";
import { currentPortfolio } from "@components/allocationFlow/portfolioSlice";
import { useTranslation } from "react-i18next";
import { Provider, useYahooPrice } from "@/api/priceProviders";
import { useAppStore } from "@/state/appStore";
import {
  PRICE_STALE_TIME,
  SEARCH_STALE_TIME,
  SESSION_STALE_TIME,
} from "@/api/queryClient";

type SearchState = {
  app: {
    currencies: string[];
  };
  pfolio: {
    selected: string | null;
    pfolios: Record<string, unknown>;
  };
};

type SearchAsset = {
  symbol: string;
  name: string;
  aclass: number;
  type?: string;
  exchange?: string;
};

type AssetToAdd = SearchAsset & {
  price: number;
  baseCcy?: string;
  provider: Provider;
};

type SearchBarProps = {
  text: string;
  setText: (text: string) => void;
  addAsset: (asset: AssetToAdd) => void;
};

type SearchResults = {
  fiat: SearchAsset[];
  crypto: SearchAsset[];
  yf: SearchAsset[];
};

type SearchItemProps = {
  data: SearchAsset;
  setText: (text: string) => void;
  addAsset: (asset: AssetToAdd) => void;
  closeSearchList: () => void;
};

type SearchItemYfProps = SearchItemProps & {
  removeAsset: (symbol: string) => void;
};

type DcaPalAsset = {
  id: string;
  symbol: string;
};

type YahooSearchQuote = {
  longname?: string | null;
  shortname?: string | null;
  symbol?: string | null;
  quoteType?: string | null;
  exchange?: string | null;
};

const useDebouncedValue = <T,>(value: T, delayMs: number): T => {
  const [debouncedValue, setDebouncedValue] = useState(value);

  useEffect(() => {
    const timeoutId = setTimeout(() => setDebouncedValue(value), delayMs);
    return () => clearTimeout(timeoutId);
  }, [value, delayMs]);

  return debouncedValue;
};

const searchOptions: IFuseOptions<SearchAsset> = {
  shouldSort: true,
  threshold: 0.1,
  keys: ["symbol", "name"],
};

const toDcaPalAsset = (asset: DcaPalAsset, aclass: number): SearchAsset => ({
  symbol: asset.id,
  name: asset.symbol,
  aclass,
});

const toYahooAsset = (quote: YahooSearchQuote): SearchAsset => ({
  name: quote.longname || quote.shortname || "",
  symbol: quote.symbol || "",
  type: quote.quoteType || "",
  exchange: quote.exchange || "",
  aclass: 10,
});

/** Searches catalog and Yahoo assets and adds a selected result to the portfolio. */
export const SearchBar = (props: SearchBarProps) => {
  const { t } = useTranslation();
  const debouncedText = useDebouncedValue(props.text, 300);
  const isSearchEnabled = debouncedText.trim().length >= 2;
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [invalidYahooSymbols, setInvalidYahooSymbols] = useState(
    () => new Set()
  );

  const fiatQuery = useGetAssetsFiat({
    query: {
      enabled: isSearchEnabled,
      staleTime: SESSION_STALE_TIME,
    },
  });
  const cryptoQuery = useGetAssetsCrypto({
    query: {
      enabled: isSearchEnabled,
      staleTime: SESSION_STALE_TIME,
    },
  });
  const yahooQuery = useGetAssetsData(
    { name: debouncedText.toLowerCase() },
    {
      query: {
        enabled: isSearchEnabled,
        staleTime: SEARCH_STALE_TIME,
      },
    }
  );

  useEffect(() => {
    setInvalidYahooSymbols(new Set());
  }, [debouncedText]);

  const results = useMemo(() => {
    if (!isSearchEnabled) return null;

    const fuse = (assets: SearchAsset[]) =>
      new Fuse(assets, searchOptions)
        .search(debouncedText)
        .map((result) => result.item)
        .sort((a, b) => a.symbol.localeCompare(b.symbol));

    const fiatAssets = (fiatQuery.data?.data || []).map((asset) =>
      toDcaPalAsset(asset, 30)
    );
    const cryptoAssets = (cryptoQuery.data?.data || []).map((asset) =>
      toDcaPalAsset(asset, 20)
    );
    const yahooQuotes = yahooQuery.data?.data?.quotes;
    const yahooAssets = Array.isArray(yahooQuotes)
      ? yahooQuotes
          .filter((quote) => {
            const type = quote.quoteType?.toUpperCase();
            return type === "EQUITY" || type === "ETF" || type === "MUTUALFUND";
          })
          .map(toYahooAsset)
          .filter((asset) => asset.symbol)
      : [];

    return {
      fiat: fuse(fiatAssets),
      crypto: fuse(cryptoAssets),
      yf: yahooAssets,
    };
  }, [
    cryptoQuery.data,
    debouncedText,
    fiatQuery.data,
    isSearchEnabled,
    yahooQuery.data,
  ]);

  const removeAssetYf = useCallback((symbol: string) => {
    setInvalidYahooSymbols((current) => {
      const next = new Set(current);
      next.add(symbol);
      return next;
    });
  }, []);

  const visibleYahooAssets = results
    ? results.yf.filter((asset) => !invalidYahooSymbols.has(asset.symbol))
    : [];
  const isLoading =
    isSearchEnabled &&
    [fiatQuery, cryptoQuery, yahooQuery].some(
      (query) => query.isPending || query.isFetching
    );
  const isEmptyResult = Boolean(
    results &&
    !isLoading &&
    results.fiat.length === 0 &&
    results.crypto.length === 0 &&
    visibleYahooAssets.length === 0
  );

  const handleAddAssetInputChange = (event: ChangeEvent<HTMLInputElement>) => {
    setIsSearchOpen(event.target.value.length > 0);
    props.setText(event.target.value);
  };

  return (
    <div className="relative flex flex-col items-center justify-center">
      <input
        data-testid="portfolio-search"
        className="w-full h-12 px-6 pb-px border-2 rounded-3xl border-neutral-500/40 focus:border-neutral-500 focus-visible:outline-none uppercase placeholder:normal-case z-20"
        value={props.text}
        placeholder={t("searchBar.placeholder")}
        onChange={handleAddAssetInputChange}
      />
      {isSearchOpen && isLoading && (
        <div
          data-testid="search-loading"
          className="w-[calc(100%-2rem)] px-6 py-3 overflow-auto absolute inset-x-4 top-[3rem] bg-white rounded-sm ring-1 ring-slate-500/50 shadow-lg z-40 flex items-center justify-center font-light italic"
        >
          <Spinner width="2.5rem" height="2.5rem" />
        </div>
      )}
      {isSearchOpen && results && isEmptyResult && (
        <div className="w-[calc(100%-2rem)] px-6 py-4 overflow-auto absolute inset-x-4 top-[3rem] bg-white rounded-sm ring-1 ring-slate-500/50 shadow-lg z-40 flex items-center justify-center font-light italic">
          {t("searchBar.noAssetFoundFor")} '{props.text.toUpperCase()}'
        </div>
      )}
      {isSearchOpen && results && !isEmptyResult && (
        <ul className="w-[calc(100%-2rem)] max-h-72 min-h-[10rem] overflow-auto absolute inset-x-4 top-[3rem] bg-white rounded-sm ring-1 ring-slate-500/50 shadow-lg z-40">
          {results.fiat.length > 0 && <SearchHeader text="cash" />}
          {results.fiat.map((result) => (
            <SearchItemCW
              key={result.symbol}
              data={result}
              setText={props.setText}
              addAsset={props.addAsset}
              closeSearchList={() => setIsSearchOpen(false)}
            />
          ))}
          {results.crypto.length > 0 && <SearchHeader text="crypto" />}
          {results.crypto.map((result) => (
            <SearchItemCW
              key={result.symbol}
              data={result}
              setText={props.setText}
              addAsset={props.addAsset}
              closeSearchList={() => setIsSearchOpen(false)}
            />
          ))}
          {visibleYahooAssets.length > 0 && <SearchHeader text="equity" />}
          {visibleYahooAssets.map((result) => (
            <SearchItemYF
              key={result.symbol}
              data={result}
              setText={props.setText}
              addAsset={props.addAsset}
              removeAsset={removeAssetYf}
              closeSearchList={() => setIsSearchOpen(false)}
            />
          ))}
        </ul>
      )}
    </div>
  );
};

const SearchHeader = ({ text }: { text: string }) => (
  <div className="sticky top-0 pl-2 pt-1 pb-1 bg-slate-200 text-xs font-semibold">
    <div className="uppercase">{text}</div>
  </div>
);

const SearchItemCW = ({
  data,
  setText,
  addAsset,
  closeSearchList,
}: SearchItemProps) => {
  const { i18n, t } = useTranslation();
  const quoteCcy = useSelector(
    (state) => currentPortfolio(state as SearchState)?.quoteCcy || ""
  );
  const priceQuery = useGetPrice(
    data.symbol,
    { quote: quoteCcy },
    {
      query: {
        enabled: Boolean(quoteCcy),
        staleTime: PRICE_STALE_TIME,
      },
    }
  );
  const price = priceQuery.data?.data?.price;
  const isPriceUnavailable = priceQuery.isError;

  const handleResultClick = () => {
    if (price == null) return;

    setText(data.symbol);
    addAsset({
      symbol: data.symbol,
      name: data.name,
      aclass: data.aclass,
      price,
      baseCcy: data.symbol,
      provider: Provider.DCA_PAL,
    });
    closeSearchList();
  };

  return (
    <li
      data-testid="asset-result"
      data-symbol={data.symbol}
      className="pl-2 pt-1 pb-1 hover:bg-slate-400/50 cursor-pointer"
    >
      <div
        className="flex items-center justify-between h-10 uppercase"
        onClick={handleResultClick}
      >
        <div className="grow flex flex-col min-w-0">
          <div className="font-medium uppercase">{data.symbol}</div>
          <div className="text-xs font-light capitalize truncate">
            {data.name}
          </div>
        </div>
        <div className="mr-2">
          {price != null ? (
            <div className="flex items-center">
              <div className="text-base font-medium m-1">
                {price.toLocaleString(i18n.language, {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 2,
                })}
              </div>
              <div className="flex justify-start w-10 uppercase">
                {quoteCcy}
              </div>
            </div>
          ) : (
            <div className="text-base font-medium">
              {isPriceUnavailable ? t("searchBar.unavailable") : "Loading..."}
            </div>
          )}
        </div>
      </div>
    </li>
  );
};

const SearchItemYF = ({
  data,
  setText,
  addAsset,
  removeAsset,
  closeSearchList,
}: SearchItemYfProps) => {
  const quoteCcy = useSelector(
    (state) => currentPortfolio(state as SearchState)?.quoteCcy || ""
  );
  const validCcys = useAppStore((state) => state.currencies);
  const { t, i18n } = useTranslation();
  const priceQuery = useYahooPrice({
    symbol: data.symbol,
    quote: quoteCcy,
    validCcys,
  });
  const price = priceQuery.data?.price;
  const baseCcy = priceQuery.data?.baseCcy;

  useEffect(() => {
    if (priceQuery.isError) removeAsset(data.symbol);
  }, [data.symbol, priceQuery.isError, removeAsset]);

  const handleResultClick = () => {
    if (price == null) return;

    setText(data.symbol);
    addAsset({
      symbol: data.symbol,
      name: data.name,
      aclass: data.aclass,
      price,
      baseCcy,
      provider: Provider.YF,
    });
    closeSearchList();
  };

  return (
    <li
      data-testid="asset-result"
      data-symbol={data.symbol}
      className="pl-2 pt-1 pb-1 hover:bg-slate-400/50 cursor-pointer"
    >
      <div
        className="flex items-center justify-between h-10 uppercase"
        onClick={handleResultClick}
      >
        <div className="grow flex flex-col min-w-0">
          <div className="font-medium uppercase">{data.symbol}</div>
          <div className="text-xs font-light capitalize truncate">
            {data.name}
          </div>
        </div>
        <div className="mr-2">
          {price != null ? (
            <div className="flex items-center">
              <div className="text-base font-medium m-1">
                {price.toLocaleString(i18n.language, {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 2,
                })}
              </div>
              <div className="flex justify-start w-10 uppercase">
                {quoteCcy}
              </div>
            </div>
          ) : (
            <div className="text-base font-medium">
              {t("searchBar.loading")}...
            </div>
          )}
        </div>
      </div>
    </li>
  );
};
