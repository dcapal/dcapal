// @ts-check
const { defineConfig, devices } = require("@playwright/test");

const storybookBaseUrl =
  process.env.STORYBOOK_BASE_URL ?? "http://127.0.0.1:6006";

module.exports = defineConfig({
  testDir: "./tests",
  testMatch: "**/storybook.smoke.spec.ts",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: [["html", { open: "never" }]],
  use: {
    baseURL: storybookBaseUrl,
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "storybook-desktop",
      use: { ...devices["Desktop Chrome"] },
    },
    {
      name: "storybook-mobile",
      use: { ...devices["Pixel 5"] },
    },
  ],
  webServer: {
    command: "pnpm run storybook:build && node scripts/serve-storybook.mjs",
    url: storybookBaseUrl,
    timeout: 120000,
    reuseExistingServer: !process.env.CI,
  },
});
