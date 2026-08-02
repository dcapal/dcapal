export const scenarios = Object.freeze({
  default: "default",
  importSuccess: "import-success",
  importUnresolvedPrice: "import-unresolved-price",
  searchDefault: "search-default",
  searchLoading: "search-loading",
  searchEmpty: "search-empty",
  searchMalformed: "search-malformed",
  searchHttpError: "search-http-error",
  searchDcaPalPriceError: "search-dcapal-price-error",
  searchYahooBadPrice: "search-yahoo-bad-price",
  searchYahooQuoteCurrency: "search-yahoo-quote-currency",
  searchYahooUnsupportedCurrency: "search-yahoo-unsupported-currency",
  syncLocalWins: "sync-local-wins",
  syncServerWins: "sync-server-wins",
  syncDeleted: "sync-deleted",
  syncRefresh: "sync-refresh",
  syncRefreshFailure: "sync-refresh-failure",
  syncSecond401: "sync-second-401",
  syncSignoutFailure: "sync-signout-failure",
} as const);

export type Scenario = (typeof scenarios)[keyof typeof scenarios];
