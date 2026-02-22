import { beforeEach, describe, expect, it, vi } from "vitest";

const { getSession } = vi.hoisted(() => ({
  getSession: vi.fn(),
}));

vi.mock("@app/config", () => ({
  DCAPAL_API: "/api",
  supabase: {
    auth: {
      getSession,
    },
  },
}));

vi.mock("@components/allocationFlow/portfolioSlice", () => ({
  FeeType: {
    ZERO_FEE: 10,
    FIXED: 20,
    VARIABLE: 30,
  },
  feeTypeToString: (type) => {
    if (type === 10) return "zeroFee";
    if (type === 20) return "fixed";
    if (type === 30) return "variable";
    return "undefined";
  },
  aclassToString: () => "EQUITY",
}));

import { syncPortfoliosAPI } from "./syncPortfolios";

describe("syncPortfoliosAPI", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    global.fetch = vi.fn();
    vi.spyOn(console, "error").mockImplementation(() => { });
  });

  it("injects bearer authorization header for sync endpoint", async () => {
    getSession.mockResolvedValue({
      data: { session: { access_token: "fixture-token" } },
    });
    global.fetch.mockResolvedValue({
      ok: true,
      json: async () => ({ updatedPortfolios: [], deletedPortfolios: [] }),
    });

    const result = await syncPortfoliosAPI({}, []);

    expect(result).toStrictEqual({
      updatedPortfolios: [],
      deletedPortfolios: [],
    });
    expect(global.fetch).toHaveBeenCalledTimes(1);
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/v1/sync/portfolios",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          "Content-Type": "application/json",
          Authorization: "Bearer fixture-token",
        }),
      })
    );
  });

  it("returns null and does not call fetch when session is missing", async () => {
    getSession.mockResolvedValue({ data: { session: null } });

    const result = await syncPortfoliosAPI({}, []);

    expect(result).toBeNull();
    expect(global.fetch).not.toHaveBeenCalled();
  });

  it("serializes non-empty portfolios and deletedPortfolios payload", async () => {
    getSession.mockResolvedValue({
      data: { session: { access_token: "fixture-token" } },
    });
    global.fetch.mockResolvedValue({
      ok: true,
      json: async () => ({ updatedPortfolios: [], deletedPortfolios: [] }),
    });

    const ts = Date.UTC(2026, 1, 13, 10, 0, 0);
    const portfolios = {
      "pfolio-1": {
        id: "pfolio-1",
        name: "Main",
        quoteCcy: "usd",
        fees: {
          feeStructure: {
            type: 20,
            feeAmount: 0.25,
            maxFeeImpact: 0.5,
          },
        },
        assets: {
          "asset-1": {
            symbol: "SPY",
            name: "SPDR S&P 500 ETF Trust",
            aclass: 10,
            baseCcy: "usd",
            provider: "YF",
            price: 609.73,
            averageBuyPrice: 601.25,
            qty: 2,
            targetWeight: 100,
            fees: {
              feeStructure: {
                type: 30,
                feeRate: 0.1,
                minFee: 0.01,
                maxFee: 0.2,
                maxFeeImpact: 0.5,
              },
            },
          },
        },
        lastUpdatedAt: ts,
      },
    };
    const deletedPortfolios = ["deleted-1"];

    await syncPortfoliosAPI(portfolios, deletedPortfolios);

    expect(global.fetch).toHaveBeenCalledTimes(1);
    const [url, options] = global.fetch.mock.calls[0];
    expect(url).toBe("/api/v1/sync/portfolios");
    expect(options.headers).toStrictEqual({
      "Content-Type": "application/json",
      Authorization: "Bearer fixture-token",
    });

    const body = JSON.parse(options.body);
    expect(body.deletedPortfolios).toStrictEqual(deletedPortfolios);
    expect(body.portfolios).toHaveLength(1);
    expect(body.portfolios[0]).toStrictEqual({
      id: "pfolio-1",
      name: "Main",
      quoteCcy: "usd",
      fees: {
        feeStructure: {
          type: "fixed",
          maxFeeImpact: 0.5,
          feeAmount: 0.25,
        },
      },
      assets: [
        {
          symbol: "spy",
          name: "SPDR S&P 500 ETF Trust",
          aclass: "EQUITY",
          baseCcy: "usd",
          provider: "YF",
          price: 609.73,
          averageBuyPrice: 601.25,
          qty: 2,
          targetWeight: 100,
          fees: {
            feeStructure: {
              type: "variable",
              maxFeeImpact: 0.5,
              feeRate: 0.1,
              minFee: 0.01,
              maxFee: 0.2,
            },
          },
        },
      ],
      lastUpdatedAt: new Date(ts).toISOString(),
    });
  });
});
