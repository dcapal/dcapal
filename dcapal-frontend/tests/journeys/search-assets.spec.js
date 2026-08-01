import { expect, test } from "../support/fixtures";
import { scenarios } from "../support/scenarios";

const openEditor = async (page, currency = "usd") => {
  await page.goto("/");
  await page.getByTestId("importStep.allocateYourSavings").first().click();
  const form = page.getByTestId("new-portfolio-form");
  await form.getByRole("textbox").fill("Search journey");
  await form.getByTestId("ccyRadio").filter({ hasText: currency }).click();
  await form.getByRole("button", { name: "Next" }).click();
  await expect(page.getByTestId("portfolio-search")).toBeVisible();
};

test.describe("normal asset search", () => {
  test.use({ scenario: scenarios.searchDefault });

  test("search groups cash, crypto, and Yahoo assets", async ({ page }) => {
    /*
     * GIVEN an investor is editing a portfolio
     * WHEN they search for cash, crypto, and market assets
     * THEN each result group shows only assets that have a real quote price
     */
    await openEditor(page);
    const search = page.getByTestId("portfolio-search");

    await search.fill("us");
    await expect(page.getByText("cash", { exact: true })).toBeVisible();
    await expect(
      page.locator('[data-testid="asset-result"][data-symbol="usd"]')
    ).toBeVisible();

    await search.fill("bt");
    await expect(page.getByText("crypto", { exact: true })).toBeVisible();
    await expect(
      page.locator('[data-testid="asset-result"][data-symbol="btc"]')
    ).toBeVisible();

    await search.fill("vw");
    await expect(page.getByText("equity", { exact: true })).toBeVisible();
    await expect(
      page.locator('[data-testid="asset-result"][data-symbol="VWCE.MI"]')
    ).toBeVisible();
  });

  test("a one-character search does not call the backend", async ({ page }) => {
    /*
     * GIVEN an investor has opened the asset search
     * WHEN they type fewer than two characters
     * THEN the search stays closed and no result is presented
     */
    await openEditor(page);
    const assetRequests = [];
    page.on("request", (request) => {
      if (request.url().includes("/api/assets/")) assetRequests.push(request);
    });
    await page.getByTestId("portfolio-search").fill("v");

    await expect(page.getByTestId("asset-result")).toHaveCount(0);
    expect(assetRequests).toHaveLength(0);
  });

  test("a Yahoo result converts its price into the quote currency", async ({
    page,
  }) => {
    /*
     * GIVEN an investor's portfolio is quoted in EUR
     * WHEN they select a Yahoo asset priced in USD
     * THEN the result displays the converted EUR price before it can be added
     */
    await openEditor(page, "eur");
    await page.getByTestId("portfolio-search").fill("vw");
    const result = page.locator(
      '[data-testid="asset-result"][data-symbol="VWCE.MI"]'
    );
    await expect(result).toContainText("93.15");
    await expect(result).toContainText("eur");
    await result.click();
    await expect(
      page.locator('[data-testid="asset-card"][data-symbol="VWCE.MI"]')
    ).toContainText("93.15");
  });
});

test.describe("search loading and failure states", () => {
  test.describe("slow search", () => {
    test.use({ scenario: scenarios.searchLoading });

    test("shows loading while the Yahoo search is pending", async ({
      page,
    }) => {
      /*
       * GIVEN an investor is editing a portfolio
       * WHEN the search backend is slow
       * THEN the search list communicates that results are still loading
       */
      await openEditor(page);
      await page.getByTestId("portfolio-search").fill("vw");
      await expect(page.getByTestId("search-loading")).toBeVisible();
      await expect(page.getByTestId("asset-result").first()).toBeVisible();
    });
  });

  for (const [label, scenario] of [
    ["empty", scenarios.searchEmpty],
    ["malformed", scenarios.searchMalformed],
    ["HTTP error", scenarios.searchHttpError],
  ]) {
    test.describe(`${label} search response`, () => {
      test.use({ scenario });

      test("shows a clear empty state", async ({ page }) => {
        /*
         * GIVEN an investor searches for an asset
         * WHEN the backend returns empty, malformed, or HTTP-error data
         * THEN the user sees a clear empty state instead of a broken result list
         */
        await openEditor(page);
        await page.getByTestId("portfolio-search").fill("zz");
        await expect(
          page.getByText("No asset found for 'ZZ'", { exact: true })
        ).toBeVisible();
      });
    });
  }
});

test.describe("DcaPal price failures", () => {
  test.use({ scenario: scenarios.searchDcaPalPriceError });

  test("does not add an unpriced DcaPal asset", async ({ page }) => {
    /*
     * GIVEN an investor searches for a cash asset
     * WHEN the DcaPal price request fails
     * THEN the result is marked unavailable and cannot be added
     */
    await openEditor(page);
    await page.getByTestId("portfolio-search").fill("us");
    const result = page.locator(
      '[data-testid="asset-result"][data-symbol="usd"]'
    );
    await expect(result).toContainText("Unavailable");
    await result.click();
    await expect(page.getByTestId("asset-card")).toHaveCount(0);
  });
});

test.describe("Yahoo chart validation", () => {
  test.use({ scenario: scenarios.searchYahooBadPrice });

  test("removes a Yahoo result whose chart has no valid close", async ({
    page,
  }) => {
    /*
     * GIVEN an investor searches for a Yahoo asset
     * WHEN its chart contains no valid closing price
     * THEN the invalid result disappears and no unusable asset is added
     */
    await openEditor(page);
    await page.getByTestId("portfolio-search").fill("bad");
    await expect(
      page.getByText("No asset found for 'BAD'", { exact: true })
    ).toBeVisible();
    await expect(page.getByTestId("asset-card")).toHaveCount(0);
  });
});

test.describe("Yahoo currency validation", () => {
  test.use({ scenario: scenarios.searchYahooUnsupportedCurrency });

  test("removes a Yahoo result quoted in an unsupported currency", async ({
    page,
  }) => {
    /*
     * GIVEN an investor searches for a Yahoo asset
     * WHEN its chart currency is not in the portfolio currency list
     * THEN the unusable result is removed from the search list
     */
    await openEditor(page);
    await page.getByTestId("portfolio-search").fill("jpy");
    await expect(
      page.getByText("No asset found for 'JPY'", { exact: true })
    ).toBeVisible();
  });
});

test.describe("Yahoo quote-currency prices", () => {
  test.use({ scenario: scenarios.searchYahooQuoteCurrency });

  test("accepts a Yahoo price already in the portfolio currency", async ({
    page,
  }) => {
    /*
     * GIVEN an investor's portfolio is quoted in EUR
     * WHEN a Yahoo chart is already quoted in EUR
     * THEN the result is added without a conversion request
     */
    await openEditor(page, "eur");
    await page.getByTestId("portfolio-search").fill("eur");
    const result = page.locator(
      '[data-testid="asset-result"][data-symbol="EUR.YF"]'
    );
    await expect(result).toContainText("12.34");
    await result.click();
    await expect(
      page.locator('[data-testid="asset-card"][data-symbol="EUR.YF"]')
    ).toContainText("12.34");
  });
});
