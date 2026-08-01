import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    coverage: {
      provider: "v8",
      reporter: ["json", "text"],
      reportsDirectory: "coverage",
      include: ["src/index.ts", "src/mutator/api-fetch.ts"],
      exclude: ["src/**/*.test.ts", "src/gen/**", "src/gen-mocks/**"],
    },
  },
});
