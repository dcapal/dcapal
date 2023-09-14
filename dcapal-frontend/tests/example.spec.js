// @ts-check
const { test, expect } = require('@playwright/test');

test('has title', async ({ page }) => {
  await page.goto('http://localhost:8080/');

  // Expect a title "to contain" a substring.
  await expect(page).toHaveTitle(/DcaPal - A smart assistant for your periodic investments | DcaPal/);
});

test('get started link', async ({ page }) => {
  await page.goto('http://localhost:8080/');

  // Click the get started link.
  await page.getByRole('link', { name: 'Get started' }).click();

  // Expects page to have a heading with the name of Installation.
  await expect(page).toHaveTitle(/Allocate | Dcapal/);
  await expect(page).toHaveURL(/.*allocate/);
});
