// @ts-check
import { test, expect } from "@playwright/test";

test("it loads and fetches portfolio currencies", async ({ page }) => {
  await page.goto("/");

  await expect(page).toHaveTitle(
    /DcaPal - A smart assistant for your periodic investments | DcaPal/
  );

  await Promise.all([
    page.getByTestId("importStep.allocateYourSavings").first().click(),
    page.waitForResponse("/api/assets/fiat"),
  ]);

  const ccyGroup = page.getByTestId("ccyGroup");
  await ccyGroup.waitFor();

  const ccyRadios = ccyGroup.getByTestId("ccyRadio");
  expect(await ccyRadios.count()).toBeGreaterThan(0);

  await expect(ccyRadios.filter({ hasText: "usd" })).toBeVisible();
  await expect(ccyRadios.filter({ hasText: "eur" })).toBeVisible();
});
