import { beforeEach, describe, expect, it, vi } from "vitest";

const { syncPortfoliosAPI } = vi.hoisted(() => ({
  syncPortfoliosAPI: vi.fn(),
}));

vi.mock("@/api", () => ({
  syncPortfoliosAPI,
}));

vi.mock("@app/config", () => ({
  REFRESH_PRICE_INTERVAL_SEC: 60,
}));

import {
  ACLASS,
  currentPortfolio,
  resetPortfolioStoreForTests,
  setPortfolioStoreStorageForTests,
  usePortfolioStore,
} from "./portfolioStore";

type MockStorage = {
  getItem: (name: string) => string | null;
  setItem: (name: string, value: string) => void;
  removeItem: (name: string) => void;
};

const createStorageMock = (): MockStorage => {
  const values = new Map<string, string>();
  return {
    getItem: (name) => values.get(name) ?? null,
    setItem: (name, value) => values.set(name, value),
    removeItem: (name) => values.delete(name),
  };
};

const createPortfolio = (id: string, name: string) => {
  return {
    id,
    name,
    assets: {},
    quoteCcy: "usd",
    nextIdx: 0,
    totalAmount: 0,
    budget: 0,
    fees: null,
    lastPriceRefresh: 1,
    lastUpdatedAt: 1,
  };
};

describe("portfolioStore", () => {
  const storage = createStorageMock();

  beforeEach(() => {
    vi.clearAllMocks();
    setPortfolioStoreStorageForTests(storage);
    resetPortfolioStoreForTests();
  });

  it("returns selected current portfolio with selector helper", () => {
    const pfolio = createPortfolio("pfolio-1", "Main");
    usePortfolioStore.getState().addPortfolio({ pfolio });
    usePortfolioStore.getState().selectPortfolio({ id: "pfolio-1" });

    const selected = currentPortfolio(usePortfolioStore.getState());
    expect(selected?.id).toBe("pfolio-1");
    expect(selected?.name).toBe("Main");
  });

  it("applies sync fulfilled-equivalent merge and deletions", async () => {
    usePortfolioStore.setState({
      selected: "local-1",
      pfolios: {
        "local-1": createPortfolio("local-1", "Local"),
        "deleted-local": createPortfolio("deleted-local", "To delete"),
      },
      deletedPortfolios: ["deleted-local"],
    });

    syncPortfoliosAPI.mockResolvedValue({
      updatedPortfolios: [
        {
          id: "remote-1",
          name: "Remote",
          quoteCcy: "usd",
          fees: {
            feeStructure: {
              type: "zeroFee",
            },
          },
          assets: [
            {
              symbol: "VWCE",
              name: "VWCE",
              aclass: "EQUITY",
              baseCcy: "usd",
              provider: "YF",
              qty: 1,
              targetWeight: 100,
              price: 100,
              fees: null,
            },
          ],
          nextIdx: 1,
          totalAmount: 100,
          budget: 0,
          lastUpdatedAt: "2026-02-14T10:00:00.000Z",
        },
      ],
      deletedPortfolios: ["deleted-local"],
    });

    await usePortfolioStore.getState().syncPortfoliosNow();

    expect(syncPortfoliosAPI).toHaveBeenCalledTimes(1);
    const state = usePortfolioStore.getState();

    expect(state.pfolios["deleted-local"]).toBeUndefined();
    expect(state.pfolios["remote-1"]).toBeDefined();
    expect(state.pfolios["remote-1"].assets.VWCE.aclass).toBe(ACLASS.EQUITY);
    expect(state.pfolios["remote-1"].assets.VWCE.weight).toBe(100);
    expect(state.pfolios["remote-1"].assets.VWCE.amount).toBe(0);
  });

  it("keeps state unchanged when sync response is null", async () => {
    const initial = {
      selected: "local-1",
      pfolios: {
        "local-1": createPortfolio("local-1", "Local"),
      },
      deletedPortfolios: [],
    };

    usePortfolioStore.setState(initial);
    syncPortfoliosAPI.mockResolvedValue(null);

    await usePortfolioStore.getState().syncPortfoliosNow();

    expect(usePortfolioStore.getState().pfolios).toStrictEqual(initial.pfolios);
    expect(usePortfolioStore.getState().selected).toBe(initial.selected);
  });
});
