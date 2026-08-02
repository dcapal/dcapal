import { defineConfig } from "orval";

/** Keeps production hooks and MSW handlers generated from the same OpenAPI document. */
export default defineConfig({
  api: {
    input: "../../dcapal-backend/docs/openapi.json",
    output: {
      target: "./src/gen/index.ts",
      schemas: "./src/gen/model",
      client: "react-query",
      mode: "single",
      override: {
        fetch: {
          forceSuccessResponse: true,
        },
        query: {
          version: 5,
        },
        mutator: {
          path: "./src/mutator/api-fetch.ts",
          name: "apiFetch",
        },
      },
    },
  },
  apiMocks: {
    input: "../../dcapal-backend/docs/openapi.json",
    output: {
      target: "./src/gen-mocks/index.ts",
      schemas: "./src/gen-mocks/model",
      client: "react-query",
      mode: "single",
      mock: {
        generators: [{ type: "msw" }],
      },
      override: {
        fetch: {
          forceSuccessResponse: true,
        },
        query: {
          version: 5,
        },
      },
    },
  },
});
