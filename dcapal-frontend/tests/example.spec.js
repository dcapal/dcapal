// @ts-check
const { test, expect } = require('@playwright/test');

test('has title', async ({ page }) => {
  await page.goto('http://127.0.0.1:8080/');

  // Expect a title "to contain" a substring.
  await expect(page).toHaveTitle(/DcaPal - A smart assistant for your periodic investments | DcaPal/);
});
