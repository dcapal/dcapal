import { delay, http, HttpResponse } from "msw";
import { getDCAPalAPIsMock } from "@dcapal/api-client/mocks";

import assetsFiat from "./fixtures/assets-fiat.json";
import assetsCrypto from "./fixtures/assets-crypto.json";
import assetsSearch from "./fixtures/assets-search.json";
import assetsChart from "./fixtures/assets-chart.json";
import importPortfolio from "./fixtures/import-portfolio.json";
import importCreatedResponse from "./fixtures/import-created-response.json";
import priceConversions from "./fixtures/price-conversions.json";

const IMPORT_PORTFOLIO_ID = "fixture-import-portfolio";
const FIXED_TS_SECONDS = 1735689600;
// Fixtures intentionally allow malformed provider payloads because journeys
// cover the UI's handling of invalid backend responses.
type FixtureRecord = Record<string, any>;
type SyncPortfolio = FixtureRecord & {
  id?: string;
  lastUpdatedAt?: string;
};
type SyncRequestBody = {
  portfolios?: SyncPortfolio[];
  deletedPortfolios?: string[];
};

// Keep each scenario's remote state separate so tests can run in any order.
const syncStores = new Map<string, Map<string, SyncPortfolio>>();
const syncRequestCounts = new Map<string, number>();

const getScenarioName = (request: Request): string =>
  request.headers.get("x-e2e-scenario") || "default";

const getScenario = (request: Request): string =>
  getScenarioName(request).split(":")[0];

const getSyncStore = (scenarioName: string): Map<string, SyncPortfolio> => {
  if (!syncStores.has(scenarioName)) syncStores.set(scenarioName, new Map());
  return syncStores.get(scenarioName) as Map<string, SyncPortfolio>;
};

const getSyncRequestCount = (scenarioName: string): number => {
  const count = (syncRequestCounts.get(scenarioName) || 0) + 1;
  syncRequestCounts.set(scenarioName, count);
  return count;
};

const errorResponse = (message: string, status = 500) =>
  HttpResponse.json({ message }, { status });

const getSearchQuotes = (scenario: string): FixtureRecord[] => {
  if (scenario === "search-empty") return [];

  if (scenario === "search-yahoo-bad-price") {
    return [
      {
        quoteType: "ETF",
        longname: "Broken Yahoo price",
        shortname: "Broken Yahoo price",
        symbol: "BAD.YF",
        exchange: "NMS",
      },
    ];
  }

  if (scenario === "search-yahoo-quote-currency") {
    return [
      {
        quoteType: "ETF",
        longname: "Euro quoted fund",
        shortname: "Euro fund",
        symbol: "EUR.YF",
        exchange: "MIL",
      },
    ];
  }

  if (scenario === "search-yahoo-unsupported-currency") {
    return [
      {
        quoteType: "ETF",
        longname: "Unsupported currency fund",
        shortname: "Unsupported currency fund",
        symbol: "JPY.YF",
        exchange: "LSE",
      },
    ];
  }

  return assetsSearch.quotes as FixtureRecord[];
};

const getChart = (scenario: string, symbol: string): FixtureRecord => {
  if (scenario === "search-yahoo-bad-price") {
    return {
      ...(assetsChart as unknown as FixtureRecord),
      chart: {
        ...((assetsChart as unknown as FixtureRecord).chart as FixtureRecord),
        result: [
          {
            ...(
              (assetsChart as unknown as FixtureRecord).chart as FixtureRecord
            ).result[0],
            meta: { currency: "USD" },
            indicators: { quote: [{ close: [0, null] }] },
          },
        ],
      },
    };
  }

  if (scenario === "search-yahoo-quote-currency" && symbol === "EUR.YF") {
    return {
      ...(assetsChart as unknown as FixtureRecord),
      chart: {
        ...((assetsChart as unknown as FixtureRecord).chart as FixtureRecord),
        result: [
          {
            ...(
              (assetsChart as unknown as FixtureRecord).chart as FixtureRecord
            ).result[0],
            meta: { currency: "EUR" },
            indicators: { quote: [{ close: [12.34] }] },
          },
        ],
      },
    };
  }

  if (scenario === "search-yahoo-unsupported-currency" && symbol === "JPY.YF") {
    return {
      ...(assetsChart as unknown as FixtureRecord),
      chart: {
        ...((assetsChart as unknown as FixtureRecord).chart as FixtureRecord),
        result: [
          {
            ...(
              (assetsChart as unknown as FixtureRecord).chart as FixtureRecord
            ).result[0],
            meta: { currency: "JPY" },
            indicators: { quote: [{ close: [10.5] }] },
          },
        ],
      },
    };
  }

  return assetsChart as FixtureRecord;
};

const getImportedPortfolio = (scenario: string): FixtureRecord => {
  if (scenario !== "import-unresolved-price") return importPortfolio;

  return {
    ...importPortfolio,
    assets: [
      {
        symbol: "broken",
        name: "Asset without a price",
        aclass: "EQUITY",
        baseCcy: "usd",
        provider: "DCAPal",
        price: "0",
        averageBuyPrice: "0",
        qty: "1",
        targetWeight: "100",
      },
    ],
  };
};

const getConversionPrice = (base: string, quote: string): number => {
  const rates = priceConversions as Record<string, Record<string, number>>;
  const baseRates = rates[base.toLowerCase()] || {};
  return baseRates[quote.toLowerCase()] || 1;
};

const toMillis = (isoTs: string | undefined): number => {
  const ts = Date.parse(isoTs || "");
  return Number.isNaN(ts) ? -1 : ts;
};

/** MSW handlers that model the backend responses used by frontend journeys. */
export const handlers = [
  http.get("/api/assets/fiat", async () => {
    await delay(30);
    return HttpResponse.json(assetsFiat);
  }),

  http.get("/api/assets/crypto", async () => {
    await delay(30);
    return HttpResponse.json(assetsCrypto);
  }),

  http.get("/api/assets/search", async ({ request }) => {
    const scenario = getScenario(request);
    await delay(scenario === "search-loading" ? 1000 : 30);
    const url = new URL(request.url);
    const query = url.searchParams.get("name");

    if (scenario === "search-http-error") {
      return errorResponse("Yahoo search service unavailable", 502);
    }

    if (scenario === "search-malformed") {
      return HttpResponse.json({ quotes: { malformed: true } });
    }

    if (!query) {
      return HttpResponse.json({ quotes: [] });
    }

    return HttpResponse.json({ quotes: getSearchQuotes(scenario) });
  }),

  http.get("/api/assets/chart/:symbol", async ({ params, request }) => {
    await delay(40);
    return HttpResponse.json(
      getChart(getScenario(request), String(params.symbol || ""))
    );
  }),

  http.get("/api/price/:asset", async ({ params, request }) => {
    await delay(20);
    const base = String(params.asset || "").toLowerCase();
    const scenario = getScenario(request);
    const url = new URL(request.url);
    const quote = String(url.searchParams.get("quote") || "").toLowerCase();

    if (
      scenario === "search-dcapal-price-error" ||
      scenario === "import-unresolved-price"
    ) {
      const failingAssets =
        scenario === "import-unresolved-price" ? ["broken"] : ["usd", "btc"];
      if (failingAssets.includes(base)) {
        return errorResponse(`Price unavailable for ${base}`, 502);
      }
    }

    const price = getConversionPrice(base, quote);

    return HttpResponse.json({
      price,
      ts: FIXED_TS_SECONDS,
    });
  }),

  http.post("/api/import/portfolio", async () => {
    await delay(40);
    return HttpResponse.json(importCreatedResponse, { status: 201 });
  }),

  http.get("/api/import/portfolio/:id", async ({ params, request }) => {
    await delay(200);

    if (params.id !== IMPORT_PORTFOLIO_ID) {
      return new HttpResponse(null, { status: 404 });
    }

    return HttpResponse.json(getImportedPortfolio(getScenario(request)));
  }),

  http.post("/api/v1/sync/portfolios", async ({ request }) => {
    await delay(30);

    const scenarioName = getScenarioName(request);
    const scenario = getScenario(request);
    const requestCount = getSyncRequestCount(scenarioName);

    const auth = request.headers.get("authorization") || "";
    if (!auth.toLowerCase().startsWith("bearer ")) {
      return HttpResponse.json(
        { message: "Missing bearer token" },
        { status: 401 }
      );
    }

    if (
      (scenario === "sync-refresh" ||
        scenario === "sync-refresh-failure" ||
        scenario === "sync-second-401" ||
        scenario === "sync-signout-failure") &&
      requestCount === 1
    ) {
      return errorResponse("Expired bearer token", 401);
    }

    const req = (await request.json()) as SyncRequestBody;
    const clientPortfolios = Array.isArray(req.portfolios)
      ? req.portfolios
      : [];
    const deletedPortfolios = Array.isArray(req.deletedPortfolios)
      ? req.deletedPortfolios
      : [];

    if (
      (scenario === "sync-second-401" || scenario === "sync-signout-failure") &&
      requestCount === 2
    ) {
      return errorResponse("Bearer token still rejected", 401);
    }

    if (scenario === "sync-server-wins" && requestCount === 1) {
      return HttpResponse.json({
        updatedPortfolios: clientPortfolios.map((portfolio) => ({
          ...portfolio,
          name: "Server portfolio",
          lastUpdatedAt: "2099-01-01T00:00:00.000Z",
        })),
        deletedPortfolios: [],
      });
    }

    if (scenario === "sync-deleted") {
      return HttpResponse.json({
        updatedPortfolios: [],
        deletedPortfolios: clientPortfolios.map((portfolio) => portfolio.id),
      });
    }

    const syncStore = getSyncStore(scenarioName);
    const updatedPortfolios = [];

    for (const clientPf of clientPortfolios) {
      if (!clientPf?.id) continue;

      const serverPf = syncStore.get(clientPf.id);
      if (!serverPf) {
        syncStore.set(clientPf.id, clientPf);
        continue;
      }

      const serverTs = toMillis(serverPf.lastUpdatedAt);
      const clientTs = toMillis(clientPf.lastUpdatedAt);

      if (clientTs > serverTs) {
        syncStore.set(clientPf.id, clientPf);
      } else if (serverTs > clientTs) {
        updatedPortfolios.push(serverPf);
      }
    }

    for (const id of deletedPortfolios) {
      syncStore.delete(id);
    }

    return HttpResponse.json({
      updatedPortfolios,
      deletedPortfolios,
    });
  }),

  http.all(/https?:\/\/127\.0\.0\.1:54321\/.*/, async ({ request }) => {
    await delay(20);

    const scenario = getScenario(request);
    const url = new URL(request.url);

    if (url.pathname.endsWith("/auth/v1/token")) {
      if (scenario === "sync-refresh-failure") {
        return errorResponse("Refresh token rejected", 400);
      }

      return HttpResponse.json({
        access_token: "refreshed-fixture-token",
        refresh_token: "refreshed-fixture-refresh-token",
        token_type: "bearer",
        expires_in: 3600,
        expires_at: Math.floor(Date.now() / 1000) + 3600,
        user: {
          id: "fixture-user",
          aud: "authenticated",
          role: "authenticated",
          email: "fixture@example.com",
          user_metadata: { name: "Fixture User" },
        },
      });
    }

    if (url.pathname.endsWith("/auth/v1/logout")) {
      if (scenario === "sync-signout-failure") {
        return errorResponse("Sign out failed", 500);
      }
      return new HttpResponse(null, { status: 204 });
    }

    if (url.pathname.endsWith("/auth/v1/user")) {
      return HttpResponse.json({
        id: "fixture-user",
        aud: "authenticated",
        role: "authenticated",
        email: "fixture@example.com",
        user_metadata: { name: "Fixture User" },
      });
    }

    return HttpResponse.json({});
  }),

  ...getDCAPalAPIsMock(),
];
