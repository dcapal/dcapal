import { delay, http, HttpResponse } from "msw";

import assetsFiat from "./fixtures/assets-fiat.json";
import assetsCrypto from "./fixtures/assets-crypto.json";
import assetsSearch from "./fixtures/assets-search.json";
import assetsChart from "./fixtures/assets-chart.json";
import importPortfolio from "./fixtures/import-portfolio.json";
import allocateFlowZeroFee from "./fixtures/allocate-flow/zero-fee.json";
import allocateFlowFixedFee from "./fixtures/allocate-flow/fixed-fee-19.json";
import allocateFlowVariableFee from "./fixtures/allocate-flow/variable-fee.json";
import allocateFlowTaxEfficientOff from "./fixtures/allocate-flow/tax-efficient-off.json";
import importCreatedResponse from "./fixtures/import-created-response.json";
import priceConversions from "./fixtures/price-conversions.json";

const importPortfoliosById = {
  "fixture-import-portfolio": importPortfolio,
  "fixture-allocate-zero-fee": allocateFlowZeroFee,
  "fixture-allocate-fixed-fee-19": allocateFlowFixedFee,
  "fixture-allocate-variable-fee": allocateFlowVariableFee,
  "fixture-allocate-tax-efficient-off": allocateFlowTaxEfficientOff,
};
const FIXED_TS_SECONDS = 1735689600;
const syncStore = new Map();

const getConversionPrice = (base, quote) => {
  const baseRates = priceConversions[String(base).toLowerCase()] || {};
  return baseRates[String(quote).toLowerCase()] || 1;
};

const toMillis = (isoTs) => {
  const ts = Date.parse(isoTs);
  return Number.isNaN(ts) ? -1 : ts;
};

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
    await delay(30);
    const url = new URL(request.url);
    const query = url.searchParams.get("name");

    if (!query) {
      return HttpResponse.json({ quotes: [] });
    }

    return HttpResponse.json(assetsSearch);
  }),

  http.get("/api/assets/chart/:symbol", async () => {
    await delay(40);
    return HttpResponse.json(assetsChart);
  }),

  http.get("/api/price/:asset", async ({ params, request }) => {
    await delay(20);
    const base = String(params.asset || "").toLowerCase();
    const url = new URL(request.url);
    const quote = String(url.searchParams.get("quote") || "").toLowerCase();
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

  http.get("/api/import/portfolio/:id", async ({ params }) => {
    await delay(200);

    const portfolio = importPortfoliosById[params.id];
    if (!portfolio) {
      return new HttpResponse(null, { status: 404 });
    }

    return HttpResponse.json(portfolio);
  }),

  http.post("/api/v1/sync/portfolios", async ({ request }) => {
    await delay(30);

    const auth = request.headers.get("authorization") || "";
    if (!auth.toLowerCase().startsWith("bearer ")) {
      return HttpResponse.json(
        { message: "Missing bearer token" },
        { status: 401 }
      );
    }

    const req = await request.json();
    const clientPortfolios = Array.isArray(req.portfolios)
      ? req.portfolios
      : [];
    const deletedPortfolios = Array.isArray(req.deletedPortfolios)
      ? req.deletedPortfolios
      : [];

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

  // Stabilize auth-ui/supabase calls during login smoke tests.
  http.all(/https?:\/\/127\.0\.0\.1:54321\/.*/, async () => {
    await delay(20);
    return HttpResponse.json({});
  }),
];
