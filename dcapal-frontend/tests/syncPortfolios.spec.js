// tests/syncPortfolios.spec.js
import { expect, test } from "@playwright/test";
import jwt from "jsonwebtoken";

test("sync portfolios e2e", async ({ request }) => {
  const token = await generateJWT();

  const lastUpdatedAtInPast = "2021-08-01T00:00:00Z";
  const lastUpdatedAtNow = new Date().toISOString();

  const portfolioId = crypto.randomUUID();

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

  // First sync request with current timestamp
  const response1 = await request.post("/api/v1/sync/portfolios", {
    data: clientPayload1,
    headers: {
      authorization: `Bearer ${token}`,
    },
  });
  expect(response1.ok()).toBeTruthy();

  const body1 = await response1.json();
  expect(body1).toStrictEqual({
    updatedPortfolios: [],
    deletedPortfolios: [],
  });

  // Second sync request with past timestamp
  const response2 = await request.post("/api/v1/sync/portfolios", {
    data: clientPayload2,
    headers: {
      authorization: `Bearer ${token}`,
    },
  });
  expect(response2.ok()).toBeTruthy();

  const body2 = await response2.json();
  expect(body2).toStrictEqual({
    updatedPortfolios: [
      {
        ...clientPayload1.portfolios[0],
        lastUpdatedAt: lastUpdatedAtNow,
      },
    ],
    deletedPortfolios: [],
  });
});

// Helper function to generate JWT token
async function generateJWT() {
  const jwtSecret = "super-secret-jwt-token-with-at-least-32-characters-long";

  const expiration = Math.floor(Date.now() / 1000) + 24 * 60 * 60;
  const sub = crypto.randomUUID();
  const sessionId = crypto.randomUUID();
  const randEmail = crypto.randomUUID() + "@example.com";

  const claims = {
    iat: 0,
    sub: sub,
    session_id: sessionId,
    role: "",
    aud: "authenticated",
    exp: expiration,
    user_metadata: {
      email: randEmail,
      full_name: null,
    },
  };

  return jwt.sign(claims, jwtSecret, { algorithm: "HS256" });
}
