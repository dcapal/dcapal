import { expect, test } from "./support/fixtures";
import { seedAuthenticatedSession } from "./support/auth";
import { makePortfolio, seedPersistedState } from "./support/state";

test("a mocked production allocation loads the bundled optimizer", async ({
  page,
}) => {
  /*
   * GIVEN the production bundle uses the browser mock backend
   * WHEN an authenticated investor opens a valid allocation at the final step
   * THEN the bundled WASM optimizer returns an allocation without worker errors
   */
  const portfolio = makePortfolio({
    id: "production-mock-allocation",
    name: "Production mock allocation",
    budget: 100,
  });
  await seedAuthenticatedSession(page);
  await seedPersistedState(page, {
    portfolios: { [portfolio.id]: portfolio },
    selected: portfolio.id,
    step: 50,
  });
  await page.route("**/auth/v1/user", (route) =>
    route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        id: "fixture-user",
        aud: "authenticated",
        role: "authenticated",
        email: "fixture@example.com",
        user_metadata: { name: "Fixture User" },
      }),
    })
  );
  await page.route("**/api/v1/sync/portfolios", (route) =>
    route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ updatedPortfolios: [], deletedPortfolios: [] }),
    })
  );

  const runtimeErrors: string[] = [];
  page.on("pageerror", (error) => {
    runtimeErrors.push(`pageerror: ${error.message}`);
  });
  page.on("console", (message) => {
    if (
      message.type() === "error" &&
      /dcapal-optimizer|wasm|worker/i.test(message.text())
    ) {
      runtimeErrors.push(`console: ${message.text()}`);
    }
  });

  await page.goto("/allocate");

  await expect(page.getByTestId("route-allocate")).toBeVisible();
  await page.waitForTimeout(1000);
  expect(runtimeErrors).toEqual([]);
  await expect(
    page.getByText(/(?:allocation is ready|allocazione è pronta)/i)
  ).toBeVisible();
  await expect(
    page.getByText(/oops! (?:something bad happened|qualcosa è andato storto)/i)
  ).toHaveCount(0);
});
