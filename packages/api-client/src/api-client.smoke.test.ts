import { QueryClient } from "@tanstack/react-query";
import { setupServer } from "msw/node";
import { afterAll, afterEach, beforeAll, describe, expect, it } from "vitest";

import { getGetPriceQueryOptions } from "./gen";
import { getGetPriceMockHandler } from "./gen-mocks";
import {
  configureApiClientBaseUrl,
  resetApiClientConfiguration,
} from ".";

const server = setupServer(
  getGetPriceMockHandler({ price: 123.45, ts: 1_700_000_000 }),
);

describe("generated API client", () => {
  beforeAll(() => server.listen());
  afterEach(() => {
    server.resetHandlers();
    resetApiClientConfiguration();
  });
  afterAll(() => server.close());

  it("fetches a generated query through TanStack Query and MSW", async () => {
    configureApiClientBaseUrl("http://localhost/api");
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    const result = await queryClient.fetchQuery(
      getGetPriceQueryOptions("btc", { quote: "usd" }),
    );

    expect(result.data).toEqual({ price: 123.45, ts: 1_700_000_000 });
    expect(result.status).toBe(200);
  });
});
