import { expect, test } from "../support/fixtures";
import type { Page } from "@playwright/test";

const startNewPortfolio = async (
  page: Page,
  currency = "eur"
): Promise<void> => {
  await page.goto("/");
  const allocateButton = page
    .getByTestId("importStep.allocateYourSavings")
    .first();
  await expect(allocateButton).toHaveCSS("cursor", "pointer");
  await allocateButton.click();

  const form = page.getByTestId("new-portfolio-form");
  await expect(form).toBeVisible();
  await form.getByRole("textbox").fill("Monthly investments");
  await form.getByTestId("ccyRadio").filter({ hasText: currency }).click();
  await form.getByRole("button", { name: "Next" }).click();
  await expect(page.getByTestId("portfolio-editor")).toBeVisible();
};

test("an investor creates a portfolio and reaches investment", async ({
  page,
}) => {
  /*
   * GIVEN an investor is starting with no saved portfolio
   * WHEN they name a portfolio, choose EUR, add an asset, and enter its target weight
   * THEN the real allocation flow lets them continue to the investment step
   */
  await startNewPortfolio(page);

  const search = page.getByTestId("portfolio-search");
  await search.fill("usd");
  const cashResult = page.locator(
    '[data-testid="asset-result"][data-symbol="usd"]'
  );
  await expect(cashResult).toContainText("0.92");
  await cashResult.click();

  const asset = page.locator('[data-testid="asset-card"][data-symbol="usd"]');
  await expect(asset).toBeVisible();

  const fields = asset.getByRole("spinbutton");
  await fields.nth(0).fill("10");
  await fields.nth(0).blur();
  await expect(asset.getByText("Avg. Buy Price")).toBeVisible();
  await asset.getByRole("spinbutton").nth(2).fill("100");
  await asset.getByRole("spinbutton").nth(2).blur();

  const confirm = page.getByRole("button", { name: "Confirm weights" });
  await expect(confirm).toBeEnabled();
  await confirm.click();

  await expect(
    page.getByText("How much you would like to allocate?")
  ).toBeVisible();
});
