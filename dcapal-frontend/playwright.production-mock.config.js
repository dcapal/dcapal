// @ts-check
const { defineConfig, devices } = require("@playwright/test");

module.exports = defineConfig({
  testDir: "./tests",
  testMatch: "**/production-mock.smoke.spec.ts",
  fullyParallel: false,
  workers: 1,
  reporter: [["html", { open: "never" }]],
  use: {
    baseURL: "http://127.0.0.1:3000",
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "production-mock-chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    // This verifies the deployable bundle with only backend requests mocked.
    command: "pnpm run build && node scripts/serve-dist.mjs",
    url: "http://127.0.0.1:3000",
    timeout: 120000,
    reuseExistingServer: false,
  },
});
