import { expect, test } from "../support/fixtures";
import { scenarios } from "../support/scenarios";

test.describe("successful portfolio import", () => {
  test.use({ scenario: scenarios.importSuccess });

  test("imports decimal values, fees, quantities, and weights", async ({
    page,
  }) => {
    /*
     * GIVEN an investor opens a shared portfolio link
     * WHEN the portfolio and fresh prices finish loading
     * THEN the allocation editor shows the imported decimal data and fee policy
     */
    const importResponse = page.waitForResponse(
      (response) =>
        response
          .url()
          .endsWith("/api/import/portfolio/fixture-import-portfolio") &&
        response.ok()
    );
    await page.goto("/import?p=fixture-import-portfolio");
    await importResponse;
    await expect(page.getByTestId("route-allocate")).toBeVisible();

    await expect(page.getByTestId("portfolio-editor")).toBeVisible({
      timeout: 3000,
    });
    await expect(page.getByText("Imported Fixture Portfolio")).toBeVisible();
    await expect(
      page.locator('[data-testid="asset-card"][data-symbol="VWCE.MI"]')
    ).toContainText("5");
    await expect(
      page.locator('[data-testid="asset-card"][data-symbol="AGGH.MI"]')
    ).toBeVisible();
    await expect(
      page
        .locator('[data-testid="asset-card"][data-symbol="AGGH.MI"]')
        .getByRole("spinbutton")
        .nth(2)
    ).toHaveValue("40");

    await page.getByTestId("portfolio-preferences").click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText("Fixed", { exact: true })).toBeVisible();
    await expect(dialog.getByText("Fee amount")).toBeVisible();
    await expect(dialog.getByRole("spinbutton").nth(1)).toHaveValue("1.25");
  });
});

test.describe("unresolved import prices", () => {
  test.use({ scenario: scenarios.importUnresolvedPrice });

  test("stops with an actionable error and allows going back", async ({
    page,
  }) => {
    /*
     * GIVEN an investor imports a portfolio with an unpriced asset
     * WHEN the price lookup fails
     * THEN the app shows a safe error state and the investor can go back
     */
    const importResponse = page.waitForResponse(
      (response) =>
        response
          .url()
          .endsWith("/api/import/portfolio/fixture-import-portfolio") &&
        response.ok()
    );
    await page.goto("/import?p=fixture-import-portfolio");
    await importResponse;
    await expect(page.getByTestId("route-allocate")).toBeVisible();

    await expect(page.getByTestId("import-error")).toBeVisible({
      timeout: 3000,
    });
    await expect(page.getByText("Oops! This is embarassing...")).toBeVisible();
    await page.getByRole("button", { name: "Go back" }).click();
    await expect(
      page.getByTestId("importStep.allocateYourSavings").first()
    ).toBeVisible();
  });
});
