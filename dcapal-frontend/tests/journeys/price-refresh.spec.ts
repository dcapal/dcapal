import { expect, test } from "../support/fixtures";
import { makePortfolio, seedPersistedState } from "../support/state";

test("an investor sees a styled success toast after stale prices refresh", async ({
  page,
}) => {
  /*
   * GIVEN an investor has a portfolio whose prices were refreshed more than five minutes ago
   * WHEN the portfolio editor opens and refreshes its prices
   * THEN the translated success toast is visible with a readable, elevated presentation
   */
  const stalePortfolio = makePortfolio({
    lastPriceRefresh: Date.now() - 6 * 60 * 1000,
  });
  await seedPersistedState(page, {
    portfolios: { [stalePortfolio.id]: stalePortfolio },
    selected: stalePortfolio.id,
    step: 30,
  });

  await page.goto("/allocate");

  const toast = page
    .getByRole("status")
    .filter({ hasText: "Refreshed prices!" });
  await expect(toast).toBeVisible();
  const toastBar = toast.locator("..");
  await expect(toastBar).toHaveCSS("background-color", "rgb(255, 255, 255)");
  await expect(toastBar).toHaveCSS("color", "rgb(54, 54, 54)");
  await expect(toastBar).toHaveCSS("padding", "8px 10px");
  await expect(toastBar).toHaveCSS("border-radius", "8px");
  await expect(toastBar).toHaveCSS("box-shadow", /rgba/);
});
