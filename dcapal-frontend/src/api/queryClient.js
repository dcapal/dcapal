import { QueryClient } from "@tanstack/react-query";

export const SESSION_STALE_TIME = Infinity;
export const SEARCH_STALE_TIME = 30_000;
export const PRICE_STALE_TIME = 5 * 60_000;

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
