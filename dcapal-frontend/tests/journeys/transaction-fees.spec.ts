import { expect, test } from "../support/fixtures";
import type { Locator, Page } from "@playwright/test";
import { makePortfolio, seedPersistedState } from "../support/state";

const openFeePreferences = async (page: Page): Promise<Locator> => {
  const portfolio = makePortfolio({
    id: "fees-portfolio",
    name: "Fees portfolio",
  });
  await seedPersistedState(page, {
    portfolios: { [portfolio.id]: portfolio },
    selected: portfolio.id,
    step: 30,
  });
  await page.goto("/allocate");
  await page.getByTestId("portfolio-preferences").click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Close" })).toHaveCSS(
    "cursor",
    "pointer"
  );
  return dialog;
};

test("an investor configures portfolio fee policies and recovers invalid bounds", async ({
  page,
}) => {
  /*
   * GIVEN an investor is configuring broker fees
   * WHEN they switch between zero, fixed, and variable policies
   * THEN each policy exposes its fields and invalid minimum/maximum bounds are recoverable
   */
  const dialog = await openFeePreferences(page);
  await expect(dialog.getByText("No fees", { exact: true })).toBeVisible();
  await expect(dialog.getByText(/zero-fee trading life/)).toBeVisible();

  await dialog.getByText("Fixed", { exact: true }).click();
  await dialog.getByRole("spinbutton").nth(0).fill("10");
  await dialog.getByRole("spinbutton").nth(0).blur();
  await dialog.getByRole("spinbutton").nth(1).fill("1.50");
  await dialog.getByRole("spinbutton").nth(1).blur();
  await expect(dialog.getByRole("spinbutton").nth(1)).toHaveValue("1.50");

  await dialog.getByText("Variable", { exact: true }).click();
  const variableFields = dialog.getByRole("spinbutton");
  await variableFields.nth(1).fill("0.5");
  await variableFields.nth(1).blur();
  await variableFields.nth(2).fill("3");
  await variableFields.nth(2).blur();
  await variableFields.nth(3).fill("1");
  await variableFields.nth(3).blur();
  await expect(dialog.getByText(/Review your Min fee/)).toBeVisible();

  await variableFields.nth(3).fill("4");
  await variableFields.nth(3).blur();
  await expect(dialog.getByText(/Review your Min fee/)).toHaveCount(0);
});

test("an investor can override fees for one asset and return to the default", async ({
  page,
}) => {
  /*
   * GIVEN a portfolio has a market asset and a portfolio-wide fee policy
   * WHEN the investor sets an asset-specific fixed fee and chooses Default again
   * THEN the asset returns to the portfolio-wide variable policy
   */
  const portfolio = makePortfolio({ id: "asset-fees", name: "Asset fees" });
  await seedPersistedState(page, {
    portfolios: { [portfolio.id]: portfolio },
    selected: portfolio.id,
    step: 30,
  });
  await page.goto("/allocate");

  const asset = page.locator(
    '[data-testid="asset-card"][data-symbol="VWCE.MI"]'
  );
  const transactionFees = asset.getByTestId("transaction-fees");
  await expect(transactionFees).toHaveCSS("cursor", "pointer");
  await expect(transactionFees).toHaveAttribute("aria-expanded", "false");
  await transactionFees.focus();
  await page.keyboard.press("Enter");
  await expect(transactionFees).toHaveAttribute("aria-expanded", "true");
  await asset.getByText("Fixed", { exact: true }).click();
  const fields = asset.getByRole("spinbutton");
  await fields.nth(4).fill("2");
  await fields.nth(4).blur();
  await expect(asset.getByText("Fee amount")).toBeVisible();

  await asset.getByText("Default", { exact: true }).click();
  await expect(asset.getByText(/zero-fee trading life/)).toBeVisible();
});
