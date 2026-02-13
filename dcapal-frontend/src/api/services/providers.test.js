import axios from "axios";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../httpClient", () => ({
  api: {
    get: vi.fn(),
  },
}));

vi.mock("@components/allocationFlow/portfolioSlice", () => ({
  ACLASS: {
    CURRENCY: 30,
    CRYPTO: 20,
  },
}));

import { api } from "../httpClient";
import {
  FetchError,
  fetchAssetsDcaPal,
  fetchPrice,
  fetchPriceYF,
} from "./providers";

describe("providers service error mapping", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, "error").mockImplementation(() => {});
    vi.spyOn(console, "warn").mockImplementation(() => {});
  });

  it("maps malformed chart payload to BAD_DATA", async () => {
    api.get.mockResolvedValue({
      status: 200,
      data: { chart: { error: { code: "bad-chart" } } },
    });

    const result = await fetchPriceYF("SPY", "usd", ["usd"], null);

    expect(result).toBe(FetchError.BAD_DATA);
  });

  it("maps canceled request to REQUEST_CANCELED", async () => {
    const canceledErr = { __CANCEL__: true };
    vi.spyOn(axios, "isCancel").mockReturnValue(true);
    api.get.mockRejectedValue(canceledErr);

    const result = await fetchPriceYF("SPY", "usd", ["usd"], null);

    expect(result).toBe(FetchError.REQUEST_CANCELED);
  });

  it("returns null for non-cancel fetchPrice failures", async () => {
    vi.spyOn(axios, "isCancel").mockReturnValue(false);
    const token = { token: "tok" };
    api.get.mockRejectedValue(new Error("boom"));

    const result = await fetchPrice("btc", "usd", token);

    expect(result).toBeNull();
    expect(api.get).toHaveBeenCalledWith("/price/btc?quote=usd", {
      cancelToken: token,
    });
  });

  it("returns [] for non-cancel fetchAssetsDcaPal failures", async () => {
    vi.spyOn(axios, "isCancel").mockReturnValue(false);
    api.get.mockRejectedValue(new Error("boom"));

    const result = await fetchAssetsDcaPal("fiat");

    expect(result).toStrictEqual([]);
  });
});
