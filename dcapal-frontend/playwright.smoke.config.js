// @ts-check
const path = require("node:path");
const { defineConfig, devices } = require("@playwright/test");

module.exports = defineConfig({
  testDir: "./tests",
  testMatch: "**/smoke.spec.ts",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [["html", { open: "never" }]],
  globalSetup: require.resolve("./tests/support/smoke.globalSetup.js"),
  use: {
    baseURL: "http://127.0.0.1:3000",
    storageState: path.resolve(
      __dirname,
      "test-results/smoke/storage-state.json"
    ),
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "smoke-chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command: "REACT_APP_E2E_MSW=0 pnpm run dev:ci",
    url: "http://127.0.0.1:3000",
    timeout: 120000,
    reuseExistingServer: false,
  },
});
