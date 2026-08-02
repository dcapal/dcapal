import { QueryClient } from "@tanstack/react-query";

/** Session-backed catalog data remains valid for the lifetime of the client. */
export const SESSION_STALE_TIME = Infinity;

/** Asset-search results may be reused briefly while the user is typing. */
export const SEARCH_STALE_TIME = 30_000;

/** Price data is considered fresh for five minutes. */
export const PRICE_STALE_TIME = 5 * 60_000;

/** Shared TanStack Query client for all frontend server-state requests. */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
      refetchOnWindowFocus: false,
    },
    mutations: {
      retry: false,
    },
  },
});
