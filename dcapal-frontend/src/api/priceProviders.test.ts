import { beforeEach, describe, expect, it, vi } from "vitest";

const { getPrice, getAssetsChart } = vi.hoisted(() => ({
  getPrice: vi.fn(),
  getAssetsChart: vi.fn(),
}));

vi.mock("@app/config", () => ({
  DCAPAL_API: "http://localhost/api",
  REFRESH_PRICE_INTERVAL_SEC: 60,
  supabase: { auth: { getSession: vi.fn() } },
}));

vi.mock("@dcapal/api-client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@dcapal/api-client")>()),
  getPrice,
  getAssetsChart,
}));

import { getPriceForProvider, Provider } from "./priceProviders";
import { queryClient } from "./queryClient";

const VALID_CCYS = ["eur", "usd", "gbp"];

const chartResponse = (currency: string, close: number) => ({
  data: {
    chart: {
      result: [
        { meta: { currency }, indicators: { quote: [{ close: [close] }] } },
      ],
    },
  },
});

describe("getPriceForProvider", () => {
  beforeEach(() => {
    queryClient.clear();
    vi.clearAllMocks();
    getPrice.mockResolvedValue({ data: { price: 1.25 } });
    getAssetsChart.mockResolvedValue(chartResponse("usd", 100));
  });

  /**
   * GIVEN a fiat cash asset stored with provider YF (current backend)
   * WHEN its price is resolved
   * THEN the DCA-Pal price API is used and Yahoo's chart is never called.
   */
  it("prices YF fiat cash through the DCA-Pal price API", async () => {
    const price = await getPriceForProvider(
      Provider.YF,
      VALID_CCYS,
      "EUR",
      "usd"
    );

    expect(price).toBe(1.25);
    expect(getPrice).toHaveBeenCalledWith(
      "EUR",
      { quote: "usd" },
      expect.anything()
    );
    expect(getAssetsChart).not.toHaveBeenCalled();
  });

  /**
   * GIVEN a YF fiat asset whose symbol case differs from the supported list
   * WHEN its price is resolved
   * THEN the match is case-insensitive and the DCA-Pal price API is used.
   */
  it("matches YF fiat symbols case-insensitively", async () => {
    const price = await getPriceForProvider(
      Provider.YF,
      ["EUR", "USD"],
      "usd",
      "eur"
    );

    expect(price).toBe(1.25);
    expect(getPrice).toHaveBeenCalledTimes(1);
    expect(getAssetsChart).not.toHaveBeenCalled();
  });

  /**
   * GIVEN a YF asset that is not a supported fiat currency (an equity)
   * WHEN its price is resolved
   * THEN the Yahoo chart is used and converted to the quote currency.
   */
  it("keeps pricing non-fiat YF assets through the Yahoo chart", async () => {
    const price = await getPriceForProvider(
      Provider.YF,
      VALID_CCYS,
      "AAPL",
      "eur"
    );

    expect(getAssetsChart).toHaveBeenCalledTimes(1);
    expect(getAssetsChart.mock.calls[0][0]).toBe("AAPL");
    // 100 USD converted via the mocked usd->eur rate of 1.25.
    expect(price).toBe(125);
    expect(getPrice).toHaveBeenCalledWith(
      "usd",
      { quote: "eur" },
      expect.anything()
    );
  });

  /**
   * GIVEN a fiat cash asset still stored with the legacy DCAPal provider
   * WHEN its price is resolved
   * THEN the DCA-Pal price API is used and Yahoo is not called.
   */
  it("keeps pricing legacy DCAPal assets through the price API", async () => {
    const price = await getPriceForProvider(
      Provider.DCA_PAL,
      VALID_CCYS,
      "eur",
      "usd"
    );

    expect(price).toBe(1.25);
    expect(getAssetsChart).not.toHaveBeenCalled();
  });

  /**
   * GIVEN a YF fiat asset and a price API that fails
   * WHEN its price is resolved
   * THEN the failure is swallowed and null is returned.
   */
  it("returns null when the price API fails for YF fiat", async () => {
    getPrice.mockRejectedValue(new Error("boom"));
    vi.spyOn(console, "error").mockImplementation(() => {});

    const price = await getPriceForProvider(
      Provider.YF,
      VALID_CCYS,
      "eur",
      "usd"
    );

    expect(price).toBeNull();
  });
});
