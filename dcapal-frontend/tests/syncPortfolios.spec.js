import { expect, test } from "./support/fixtures";
import { seedAuthenticatedSession } from "./support/auth";
import { scenarios } from "./support/scenarios";
import { makePortfolio, seedPersistedState } from "./support/state";

const seedSyncPortfolio = async (page, { authenticated = true, name } = {}) => {
  const portfolio = makePortfolio({
    id: "sync-portfolio",
    name: name || "Sync portfolio",
    lastUpdatedAt: "2026-01-01T00:00:00.000Z",
  });
  await seedPersistedState(page, {
    portfolios: { [portfolio.id]: portfolio },
    selected: null,
    step: 10,
  });
  if (authenticated) await seedAuthenticatedSession(page);

  const syncRequests = [];
  page.on("request", (request) => {
    if (request.url().includes("/api/v1/sync/portfolios")) {
      syncRequests.push(request);
    }
  });
  await page.goto("/allocate");
  return { portfolio, syncRequests };
};

const waitForSyncCount = async (requests, count) => {
  await expect.poll(() => requests.length).toBeGreaterThanOrEqual(count);
};

test.describe("unauthenticated synchronization", () => {
  test.use({ scenario: scenarios.syncLocalWins });

  test("does not send local portfolios without a session", async ({ page }) => {
    /*
     * GIVEN an investor is not authenticated
     * WHEN the app loads a local portfolio
     * THEN the synchronization boundary remains untouched
     */
    const { syncRequests } = await seedSyncPortfolio(page, {
      authenticated: false,
    });
    await page.waitForTimeout(100);
    expect(syncRequests).toHaveLength(0);
  });
});

test.describe("authenticated synchronization", () => {
  test.describe("initial and local-wins sync", () => {
    test.use({ scenario: scenarios.syncLocalWins });

    test("sends decimal-string portfolio data and keeps a newer local rename", async ({
      page,
    }) => {
      /*
       * GIVEN an authenticated investor has a local portfolio
       * WHEN the app performs its initial sync and the investor renames it locally
       * THEN the request contains REST decimal strings and the newer local name wins
       */
      const { syncRequests } = await seedSyncPortfolio(page);
      await waitForSyncCount(syncRequests, 1);
      const initialPayload = JSON.parse(syncRequests[0].postData());
      expect(initialPayload.portfolios[0].assets[0].price).toBe("100");
      await syncRequests[0].response();
      await page.waitForTimeout(100);

      const card = page.getByTestId("portfolio-card");
      await card.getByTestId("portfolio-edit").click();
      await card.getByRole("textbox").fill("Local winner");
      await card.getByRole("button", { name: "Save" }).click();
      await expect
        .poll(() =>
          syncRequests.some((request) => {
            const payload = request.postData();
            return (
              payload &&
              JSON.parse(payload).portfolios[0].name === "Local winner"
            );
          })
        )
        .toBe(true);
    });

    test("runs the five-second sync interval without a wall-clock wait", async ({
      page,
    }) => {
      /*
       * GIVEN an authenticated investor has completed the initial sync
       * WHEN five virtual seconds pass
       * THEN the coordinator performs its scheduled synchronization
       */
      await page.clock.install({ time: Date.now() });
      const { syncRequests } = await seedSyncPortfolio(page);
      await waitForSyncCount(syncRequests, 1);
      await syncRequests[0].response();
      await page.waitForTimeout(100);
      await page.clock.runFor(5000);
      await waitForSyncCount(syncRequests, 2);
    });
  });

  test.describe("server conflict", () => {
    test.use({ scenario: scenarios.syncServerWins });

    test("applies a newer server portfolio visibly", async ({ page }) => {
      /*
       * GIVEN an authenticated investor has a local portfolio
       * WHEN the server returns a newer conflicting portfolio during sync
       * THEN the visible portfolio card uses the server version
       */
      await seedSyncPortfolio(page, { name: "Local portfolio" });
      await expect(page.getByTestId("portfolio-card")).toContainText(
        "Server portfolio"
      );
    });
  });

  test.describe("server deletion", () => {
    test.use({ scenario: scenarios.syncDeleted });

    test("removes a portfolio deleted by the server", async ({ page }) => {
      /*
       * GIVEN an authenticated investor has a locally saved portfolio
       * WHEN synchronization reports that the portfolio was deleted remotely
       * THEN it disappears from the visible portfolio list
       */
      await seedSyncPortfolio(page);
      await expect(page.getByTestId("portfolio-card")).toHaveCount(0);
      await expect(page.getByTestId("new-portfolio-form")).toBeVisible();
    });
  });
});

test.describe("token refresh", () => {
  test.use({ scenario: scenarios.syncRefresh });

  test("refreshes an expired token and retries the sync", async ({ page }) => {
    /*
     * GIVEN an authenticated investor's first sync receives an expired-token response
     * WHEN Supabase refreshes the session
     * THEN the real client retries synchronization with the refreshed bearer token
     */
    const { syncRequests } = await seedSyncPortfolio(page);
    await waitForSyncCount(syncRequests, 2);
    expect(
      syncRequests.some(
        (request) =>
          request.headers().authorization === "Bearer refreshed-fixture-token"
      )
    ).toBe(true);
  });
});

test.describe("refresh failure", () => {
  test.use({ scenario: scenarios.syncRefreshFailure });

  test("signs out safely when token refresh fails", async ({ page }) => {
    /*
     * GIVEN an authenticated investor's sync token is expired
     * WHEN Supabase rejects the refresh token
     * THEN the expired session is cleared and the application remains rendered
     */
    const requests = [];
    page.on("request", (request) => requests.push(request));
    await seedSyncPortfolio(page);
    await expect(page.getByTestId("route-allocate")).toBeVisible();
    await expect
      .poll(
        () =>
          requests.filter((request) => request.url().includes("/auth/v1/token"))
            .length
      )
      .toBeGreaterThanOrEqual(1);
  });
});

test.describe("second unauthorized response", () => {
  test.use({ scenario: scenarios.syncSecond401 });

  test("signs out after the refreshed token is also rejected", async ({
    page,
  }) => {
    /*
     * GIVEN an authenticated investor has an expired token
     * WHEN the retried request is rejected again
     * THEN the auth failure callback signs the session out
     */
    const requests = [];
    page.on("request", (request) => requests.push(request));
    await seedSyncPortfolio(page);
    await expect
      .poll(
        () =>
          requests.filter((request) =>
            request.url().includes("/auth/v1/logout")
          ).length
      )
      .toBeGreaterThanOrEqual(1);
  });
});

test.describe("sign-out failure", () => {
  test.use({ scenario: scenarios.syncSignoutFailure });

  test("does not blank the app when sign-out itself fails", async ({
    page,
  }) => {
    /*
     * GIVEN an authenticated investor reaches an auth failure
     * WHEN Supabase sign-out also returns an error
     * THEN the application remains available for the investor to recover
     */
    await seedSyncPortfolio(page);
    await expect(page.getByTestId("route-allocate")).toBeVisible();
    await expect(page.getByText("Sync portfolio")).toBeVisible();
  });
});
