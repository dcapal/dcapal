import { expect, test } from "@playwright/test";

const seedPersistedState = async (page) => {
  await page.addInitScript(() => {
    const persistedRoot = {
      app: JSON.stringify({
        allocationFlowStep: 10,
        currencies: ["usd", "eur", "gbp", "chf"],
        preferredCurrency: "",
        pfolioFile: "",
      }),
      pfolio: JSON.stringify({
        selected: null,
        pfolios: {},
        deletedPortfolios: [],
      }),
      _persist: JSON.stringify({ version: 5, rehydrated: true }),
    };

    localStorage.setItem("persist:root", JSON.stringify(persistedRoot));
  });
};

test("route '/' renders Root", async ({ page }) => {
  await page.goto("/");

  await expect(
    page.getByTestId("importStep.allocateYourSavings").first()
  ).toBeVisible();
});

test("route '/allocate' renders lazy App", async ({ page }) => {
  await page.goto("/allocate");

  await expect(page.getByTestId("route-allocate")).toBeVisible();
  await expect(page.getByTestId("ccyGroup")).toBeVisible();
});

test("route '/import' renders and redirects to allocate", async ({ page }) => {
  await seedPersistedState(page);

  const importedPortfolioResponse = page.waitForResponse((response) =>
    response
      .url()
      .includes("/api/import/portfolio/fixture-import-portfolio")
  );

  await page.goto("/import?p=fixture-import-portfolio");

  await importedPortfolioResponse;
  await page.waitForURL("**/allocate");
  await expect(page.getByTestId("route-allocate")).toBeVisible();
});

test("route '/login' renders login page", async ({ page }) => {
  await page.goto("/login");

  await expect(page.getByTestId("route-login")).toBeVisible();
});

test("route '/demo/60-40' redirects and loads the demo portfolio", async ({
  page,
}) => {
  await seedPersistedState(page);

  await page.goto("/demo/60-40");

  await page.waitForURL("**/allocate");
  await expect(page.getByTestId("route-allocate")).toBeVisible();
  await expect(page.getByText("VWCE.MI")).toBeVisible();
});
