import { expect, test } from "./support/fixtures";

test("an investor can choose a quote currency", async ({ page }) => {
  /*
   * GIVEN an investor has opened the allocation flow
   * WHEN they start a new portfolio
   * THEN the fiat currency catalog is loaded and EUR and USD can be chosen
   */
  await page.goto("/");
  await page.getByTestId("importStep.allocateYourSavings").first().click();

  const ccyGroup = page.getByTestId("ccyGroup");
  await expect(ccyGroup).toBeVisible();
  await expect(
    ccyGroup.getByTestId("ccyRadio").filter({ hasText: "usd" })
  ).toBeVisible();
  await expect(
    ccyGroup.getByTestId("ccyRadio").filter({ hasText: "eur" })
  ).toBeVisible();
});
