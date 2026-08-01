const clone = (value) => JSON.parse(JSON.stringify(value));

export const makeAsset = (overrides = {}) => ({
  idx: 0,
  symbol: "VWCE.MI",
  name: "Vanguard FTSE All-World UCITS ETF",
  aclass: 10,
  baseCcy: "usd",
  price: 100,
  provider: "YF",
  qty: 1,
  amount: 100,
  weight: 100,
  targetWeight: 100,
  averageBuyPrice: 100,
  fees: null,
  ...overrides,
});

export const makePortfolio = (overrides = {}) => {
  const assets = overrides.assets || {
    "VWCE.MI": makeAsset(),
  };
  const totalAmount =
    overrides.totalAmount ??
    Object.values(assets).reduce((total, asset) => total + asset.amount, 0);

  return {
    id: "portfolio-fixture",
    name: "Fixture portfolio",
    assets,
    quoteCcy: "usd",
    nextIdx: Object.keys(assets).length,
    totalAmount,
    budget: 0,
    fees: {
      feeStructure: {
        type: 10,
      },
    },
    lastPriceRefresh: Date.now(),
    lastUpdatedAt: Date.now(),
    ...overrides,
  };
};

export const persistedRoot = ({
  portfolios = {},
  selected = null,
  step = 10,
  currencies = ["usd", "eur", "gbp", "chf"],
  preferredCurrency = "",
  pfolioFile = "",
} = {}) => ({
  app: JSON.stringify({
    allocationFlowStep: step,
    currencies,
    preferredCurrency,
    pfolioFile,
  }),
  pfolio: JSON.stringify({
    selected,
    pfolios: portfolios,
    deletedPortfolios: [],
  }),
  _persist: JSON.stringify({ version: 6, rehydrated: true }),
});

export const seedPersistedState = async (page, options = {}) => {
  const root = persistedRoot(options);
  await page.addInitScript((value) => {
    if (!window.localStorage.getItem("persist:root")) {
      window.localStorage.setItem("persist:root", JSON.stringify(value));
    }
  }, root);
};

export const portfolioState = (portfolio, options = {}) => {
  const copy = clone(portfolio);
  return {
    portfolios: { [copy.id]: copy },
    selected: options.selected ?? copy.id,
    ...options,
  };
};

export const createPortfolioFixtures = () => ({
  positive: makePortfolio({
    id: "portfolio-positive",
    name: "Positive gain",
    assets: {
      VWCE: makeAsset({
        symbol: "VWCE",
        name: "Positive asset",
        price: 110,
        amount: 110,
        averageBuyPrice: 100,
      }),
    },
  }),
  negative: makePortfolio({
    id: "portfolio-negative",
    name: "Negative gain",
    assets: {
      AGGH: makeAsset({
        symbol: "AGGH",
        name: "Negative asset",
        price: 90,
        amount: 90,
        averageBuyPrice: 100,
      }),
    },
  }),
  zero: makePortfolio({
    id: "portfolio-zero",
    name: "Zero gain",
    assets: {
      CASH: makeAsset({
        symbol: "CASH",
        name: "Zero asset",
      }),
    },
  }),
});
