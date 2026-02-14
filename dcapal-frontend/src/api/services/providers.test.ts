import axios from "axios";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../httpClient", () => ({
  api: {
    get: vi.fn(),
  },
}));

vi.mock("@/state/portfolioDomain", () => ({
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
  const mockedGet = vi.mocked(api.get);

  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, "error").mockImplementation(() => {});
    vi.spyOn(console, "warn").mockImplementation(() => {});
  });

  it("maps malformed chart payload to BAD_DATA", async () => {
    mockedGet.mockResolvedValue({
      status: 200,
      data: { chart: { error: { code: "bad-chart" } } },
      statusText: "OK",
      headers: {},
      config: { headers: {} },
    });

    const result = await fetchPriceYF("SPY", "usd", ["usd"], null);

    expect(result).toBe(FetchError.BAD_DATA);
  });

  it("maps canceled request to REQUEST_CANCELED", async () => {
    const canceledErr = { __CANCEL__: true };
    vi.spyOn(axios, "isCancel").mockReturnValue(true);
    mockedGet.mockRejectedValue(canceledErr);

    const result = await fetchPriceYF("SPY", "usd", ["usd"], null);

    expect(result).toBe(FetchError.REQUEST_CANCELED);
  });

  it("returns null for non-cancel fetchPrice failures", async () => {
    vi.spyOn(axios, "isCancel").mockReturnValue(false);
    const token = { token: "tok" };
    mockedGet.mockRejectedValue(new Error("boom"));

    const result = await fetchPrice("btc", "usd", token as never);

    expect(result).toBeNull();
    expect(mockedGet).toHaveBeenCalledWith("/price/btc?quote=usd", {
      cancelToken: token,
    });
  });

  it("returns [] for non-cancel fetchAssetsDcaPal failures", async () => {
    vi.spyOn(axios, "isCancel").mockReturnValue(false);
    mockedGet.mockRejectedValue(new Error("boom"));

    const result = await fetchAssetsDcaPal("fiat");

    expect(result).toStrictEqual([]);
  });
});
