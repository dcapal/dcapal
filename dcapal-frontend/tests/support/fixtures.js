import { test as base } from "@playwright/test";

import { writeBrowserCoverage } from "./coverage";

export const test = base.extend({
  scenario: ["default", { option: true }],

  scenarioHeader: [
    async ({ page, scenario }, use, testInfo) => {
      await page.setExtraHTTPHeaders({
        "x-e2e-scenario": `${scenario}:${testInfo.testId}`,
      });
      await use();
    },
    { auto: true },
  ],

  browserCoverage: [
    async ({ page }, use, testInfo) => {
      const coverageEnabled =
        process.env.FRONTEND_COVERAGE === "1" &&
        testInfo.project.name.startsWith("coverage-");

      if (!coverageEnabled) {
        await use();
        return;
      }

      await page.coverage.startJSCoverage({ resetOnNavigation: false });
      try {
        await use();
      } finally {
        const entries = await page.coverage.stopJSCoverage();
        await writeBrowserCoverage(entries, testInfo);
      }
    },
    { auto: true },
  ],
});

export { expect } from "@playwright/test";
