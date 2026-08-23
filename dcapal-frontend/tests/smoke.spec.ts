import { expect, test } from "@playwright/test";

import { makePortfolio, seedPersistedState } from "./support/state";

const smokePortfolio = makePortfolio({
  id: "11111111-1111-4111-8111-111111111111",
  name: "Smoke portfolio",
  lastUpdatedAt: "2026-01-01T00:00:00.000Z",
});

const isPath = (url: string, pathname: string): boolean => {
  return new URL(url).pathname === pathname;
};

test("a production allocation completes with the bundled optimizer", async ({
  page,
}) => {
  /*
   * GIVEN an investor has a valid portfolio and a non-zero investment budget
   * WHEN the production allocation route opens at its final step
   * THEN the bundled WASM optimizer produces an allocation instead of the
   * generic error state, without reporting browser runtime errors
   */
  const portfolio = makePortfolio({
    id: "allocation-smoke-portfolio",
    name: "Allocation smoke portfolio",
    budget: 100,
  });
  await seedPersistedState(page, {
    portfolios: { [portfolio.id]: portfolio },
    selected: portfolio.id,
    step: 50,
  });

  const runtimeErrors: string[] = [];
  page.on("pageerror", (error) => {
    runtimeErrors.push(`pageerror: ${error.message}`);
  });
  page.on("console", (message) => {
    if (message.type() === "error") {
      runtimeErrors.push(`console: ${message.text()}`);
    }
  });

  await page.goto("/allocate");

  await expect(page.getByTestId("route-allocate")).toBeVisible();
  await expect(
    page.getByText(/(?:allocation is ready|allocazione è pronta)/i)
  ).toBeVisible();
  await expect(
    page.getByText(/oops! (?:something bad happened|qualcosa è andato storto)/i)
  ).toHaveCount(0);
  expect(runtimeErrors).toEqual([]);
});

/*
 * GIVEN the global setup has prepared a real Supabase session and the browser
 * has a seeded portfolio
 * WHEN /allocate opens
 * THEN Supabase returns the seeded user, the frontend sends an authenticated
 * POST /api/v1/sync/portfolios request, and the expected portfolio is included
 */
test("syncs a Supabase session through the frontend", async ({ page }) => {
  await seedPersistedState(page, {
    portfolios: { [smokePortfolio.id]: smokePortfolio },
    selected: smokePortfolio.id,
    step: 10,
  });

  const authUserResponse = page.waitForResponse((response) => {
    return (
      response.request().method() === "GET" &&
      isPath(response.url(), "/auth/v1/user")
    );
  });
  const syncRequest = page.waitForRequest((request) => {
    return (
      request.method() === "POST" &&
      isPath(request.url(), "/api/v1/sync/portfolios")
    );
  });
  const syncResponse = page.waitForResponse((response) => {
    return (
      response.request().method() === "POST" &&
      isPath(response.url(), "/api/v1/sync/portfolios")
    );
  });

  await page.goto("/allocate");

  const [authResponse, request, response] = await Promise.all([
    authUserResponse,
    syncRequest,
    syncResponse,
  ]);

  expect(authResponse.status()).toBe(200);
  await expect(authResponse.json()).resolves.toMatchObject({
    email: process.env.SMOKE_USER_EMAIL || "smoke@example.com",
  });

  expect(request.headers().authorization).toMatch(/^Bearer\s+\S+/);
  expect(request.postDataJSON()).toMatchObject({
    portfolios: [
      expect.objectContaining({
        id: smokePortfolio.id,
        name: smokePortfolio.name,
        assets: [expect.objectContaining({ symbol: "vwce.mi" })],
      }),
    ],
    deletedPortfolios: [],
  });
  expect(response.status()).toBe(200);
  await expect(page.getByTestId("route-allocate")).toBeVisible();
});
