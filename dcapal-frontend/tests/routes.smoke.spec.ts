import { expect, test } from "./support/fixtures";

test("the home page offers the allocation journey", async ({ page }) => {
  /*
   * GIVEN an investor visits DcaPal
   * WHEN the home page finishes loading
   * THEN the investor can start describing a portfolio
   */
  await page.goto("/");

  await expect(page).toHaveTitle(
    /DcaPal - A smart assistant for your periodic investments | DcaPal/
  );
  await expect(
    page.getByTestId("importStep.allocateYourSavings").first()
  ).toBeVisible();
});

test("the allocate route loads the currency step", async ({ page }) => {
  /*
   * GIVEN an investor has chosen to allocate savings
   * WHEN the allocate route opens
   * THEN the portfolio flow displays the available quote currencies
   */
  await page.goto("/allocate");

  await expect(page.getByTestId("route-allocate")).toBeVisible();
  await expect(page.getByTestId("ccyGroup")).toBeVisible();
});

test("a valid import link enters the real import step", async ({ page }) => {
  /*
   * GIVEN an investor opens a valid shared portfolio link
   * WHEN the backend returns the portfolio
   * THEN the app enters the allocation route and shows import progress
   */
  await page.goto("/import?p=fixture-import-portfolio");

  await expect(page.getByTestId("route-allocate")).toBeVisible();
  await expect(page.getByText("Import Portfolio")).toBeVisible();
});

test("a missing import link safely returns to portfolio selection", async ({
  page,
}) => {
  /*
   * GIVEN an investor opens an expired or unknown shared portfolio link
   * WHEN the import request returns not found
   * THEN the app returns to the new-portfolio screen without a permanent loader
   */
  await page.goto("/import?p=missing-portfolio");

  await page.waitForURL("**/allocate");
  await expect(page.getByTestId("route-allocate")).toBeVisible();
  await expect(page.getByTestId("new-portfolio-form")).toBeVisible();
});

test("the login route renders authentication controls", async ({ page }) => {
  /*
   * GIVEN an investor is not signed in
   * WHEN the login route opens
   * THEN the authentication screen shows visible, usable OAuth buttons
   */
  await page.goto("/login");

  const loginRoute = page.getByTestId("route-login");
  await expect(loginRoute).toBeVisible();

  const oauthButtons = loginRoute.getByRole("button");
  await expect(oauthButtons).toHaveCount(2);
  for (const provider of ["Google", "GitHub"]) {
    const button = oauthButtons.filter({ hasText: provider });
    await expect(button).toBeVisible();
    await expect(button).toHaveCSS("border-width", "1px");
    await expect(button).toHaveCSS("background-color", "rgb(255, 255, 255)");
    await expect(button).toHaveCSS("padding", "10px 15px");
    await expect
      .poll(() =>
        button.evaluate((element) => element.getBoundingClientRect().height)
      )
      .toBeGreaterThan(0);
  }
});

test("an unknown route renders the not-found page", async ({ page }) => {
  /*
   * GIVEN an investor follows an unknown link
   * WHEN the router cannot match the path
   * THEN the app shows a not-found page with a way home
   */
  await page.goto("/does-not-exist");

  await expect(page.getByText("Page not found")).toBeVisible();
});
