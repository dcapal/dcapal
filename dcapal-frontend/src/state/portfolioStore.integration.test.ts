import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { http, HttpResponse } from "msw";
import { setupServer } from "msw/node";
import { readFileSync } from "node:fs";

const { getSession } = vi.hoisted(() => ({
  getSession: vi.fn(),
}));

vi.mock("@app/config", () => ({
  DCAPAL_API: "http://localhost/api",
  REFRESH_PRICE_INTERVAL_SEC: 60,
  supabase: {
    auth: {
      getSession,
    },
  },
}));

import { resetPortfolioStoreForTests, usePortfolioStore } from "./portfolioStore";

const server = setupServer();
const syncFixture = JSON.parse(
  readFileSync(
    new URL("../mocks/fixtures/sync-portfolios-response.json", import.meta.url),
    "utf-8"
  )
);

describe("portfolioStore integration sync with MSW", () => {
  beforeAll(() => server.listen({ onUnhandledRequest: "error" }));

  afterEach(() => {
    server.resetHandlers();
  });

  afterAll(() => server.close());

  beforeEach(() => {
    vi.clearAllMocks();
    resetPortfolioStoreForTests();
    getSession.mockResolvedValue({
      data: { session: { access_token: "fixture-token" } },
    });
  });

  it("syncs state using real API service and fixture payload", async () => {
    let capturedAuthorization = "";
    let capturedBody: Record<string, unknown> | null = null;

    server.use(
      http.post("http://localhost/api/v1/sync/portfolios", async ({ request }) => {
        capturedAuthorization = request.headers.get("authorization") || "";
        capturedBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json(syncFixture);
      })
    );

    usePortfolioStore.setState({
      selected: "local-1",
      pfolios: {
        "local-1": {
          id: "local-1",
          name: "Local",
          assets: {},
          quoteCcy: "usd",
          nextIdx: 0,
          totalAmount: 0,
          budget: 0,
          fees: null,
          lastPriceRefresh: Date.now(),
          lastUpdatedAt: Date.now(),
        },
        "deleted-local": {
          id: "deleted-local",
          name: "Deleted local",
          assets: {},
          quoteCcy: "usd",
          nextIdx: 0,
          totalAmount: 0,
          budget: 0,
          fees: null,
          lastPriceRefresh: Date.now(),
          lastUpdatedAt: Date.now(),
        },
      },
      deletedPortfolios: ["deleted-local"],
    });

    await usePortfolioStore.getState().syncPortfoliosNow();

    expect(capturedAuthorization).toBe("Bearer fixture-token");
    expect((capturedBody as any)?.deletedPortfolios).toStrictEqual([
      "deleted-local",
    ]);

    const state = usePortfolioStore.getState();
    expect(state.pfolios["deleted-local"]).toBeUndefined();
    expect(state.pfolios["remote-1"]).toBeDefined();
    expect(state.pfolios["remote-1"].quoteCcy).toBe("usd");
    expect(state.pfolios["remote-1"].assets.VWCE.symbol).toBe("VWCE");
  });
});
