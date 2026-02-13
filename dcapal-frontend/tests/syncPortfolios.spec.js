import { expect, test } from "@playwright/test";

test("sync portfolios restores conflict-resolution coverage using MSW", async ({
  page,
}) => {
  await page.goto("/");

  const lastUpdatedAtInPast = "2021-08-01T00:00:00Z";
  const lastUpdatedAtNow = "2026-02-13T10:00:00.000Z";
  const portfolioId = "f2479b20-a873-48fd-84c3-12fd979afebd";

  const clientPayload1 = {
    portfolios: [
      {
        id: portfolioId,
        name: "p2",
        quoteCcy: "usd",
        fees: {
          feeStructure: {
            type: "zeroFee",
          },
        },
        assets: [
          {
            symbol: "vwenx",
            name: "Vanguard Wellington Admiral",
            aclass: "EQUITY",
            baseCcy: "usd",
            provider: "YF",
            price: 76.38,
            qty: 10,
            targetWeight: 90,
            fees: {
              feeStructure: {
                type: "variable",
                feeRate: 0.005,
                minFee: 0.01,
                maxFee: 0.1,
              },
            },
          },
          {
            symbol: "spy",
            name: "SPDR S&P 500 ETF Trust",
            aclass: "EQUITY",
            baseCcy: "usd",
            provider: "YF",
            price: 609.73,
            qty: 1,
            targetWeight: 10,
            fees: {
              feeStructure: {
                type: "fixed",
                feeAmount: 0.01,
              },
            },
          },
        ],
        lastUpdatedAt: lastUpdatedAtNow,
      },
    ],
    deletedPortfolios: [],
  };

  const clientPayload2 = {
    ...clientPayload1,
    portfolios: [
      {
        ...clientPayload1.portfolios[0],
        lastUpdatedAt: lastUpdatedAtInPast,
      },
    ],
  };

  const response1 = await page.evaluate(async (body) => {
    const response = await fetch("/api/v1/sync/portfolios", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: "Bearer fixture-token",
      },
      body: JSON.stringify(body),
    });

    return {
      status: response.status,
      body: await response.json(),
    };
  }, clientPayload1);

  expect(response1.status).toBe(200);
  expect(response1.body).toStrictEqual({
    updatedPortfolios: [],
    deletedPortfolios: [],
  });

  const response2 = await page.evaluate(async (body) => {
    const response = await fetch("/api/v1/sync/portfolios", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: "Bearer fixture-token",
      },
      body: JSON.stringify(body),
    });

    return {
      status: response.status,
      body: await response.json(),
    };
  }, clientPayload2);

  expect(response2.status).toBe(200);
  expect(response2.body).toStrictEqual({
    updatedPortfolios: [
      {
        ...clientPayload1.portfolios[0],
      },
    ],
    deletedPortfolios: [],
  });

  const missingAuthResponse = await page.evaluate(async () => {
    const response = await fetch("/api/v1/sync/portfolios", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ portfolios: [], deletedPortfolios: [] }),
    });

    return {
      status: response.status,
      body: await response.json(),
    };
  });

  expect(missingAuthResponse.status).toBe(401);
  expect(missingAuthResponse.body).toStrictEqual({
    message: "Missing bearer token",
  });
});
