import { expect, test } from "@playwright/test";

const seedPersistedState = async (page) => {
  await page.addInitScript(() => {
    const persistedRoot = {
      app: JSON.stringify({
        allocationFlowStep: 10,
        currencies: ["usd", "eur", "gbp", "chf"],
        preferredCurrency: "",
        pfolioFile: "",
      }),
      pfolio: JSON.stringify({
        selected: null,
        pfolios: {},
        deletedPortfolios: [],
      }),
      _persist: JSON.stringify({ version: 5, rehydrated: true }),
    };

    localStorage.setItem("persist:root", JSON.stringify(persistedRoot));
  });
};

const importFixture = async (page, fixtureId) => {
  await seedPersistedState(page);

  const importedPortfolioResponse = page.waitForResponse((response) =>
    response.url().includes(`/api/import/portfolio/${fixtureId}`)
  );

  await page.goto(`/import?p=${fixtureId}`);
  await importedPortfolioResponse;
  await page.waitForURL("**/allocate");
  await expect(page.getByTestId("route-allocate")).toBeVisible();
};

const goToInvestStep = async (page) => {
  await page.getByRole("button", { name: "Confirm weights" }).click();
  await expect(page.getByTestId("invest.recommendation.text")).toBeVisible();
  await expect(page.getByTestId("invest.runAllocation.button")).toBeVisible();
};

const setInvestCash = async (page, value) => {
  const cashInput = page.getByTestId("invest.cash.input");
  await cashInput.fill(String(value));
  await cashInput.blur();
};

const runAllocation = async (page) => {
  await expect(page.getByTestId("invest.recommendation.text")).toBeVisible();
  await expect(page.getByTestId("invest.runAllocation.button")).toBeEnabled();
  await page.getByTestId("invest.runAllocation.button").click();
  await expect(page.getByTestId("end.allocationReady")).toBeVisible({
    timeout: 15000,
  });
  await expect(page.getByTestId("end.allocationError")).toHaveCount(0);
};

const openPreferences = async (page) => {
  await page.getByTestId("portfolio.preferences.button").click();
  await expect(page.getByTestId("portfolio.preferences.dialog")).toBeVisible();
};

const closePreferences = async (page) => {
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("portfolio.preferences.dialog")).not.toBeVisible();
};

const setFixedFees = async (page, feeAmount) => {
  const dialog = page.getByTestId("portfolio.preferences.dialog");
  await dialog.getByTestId("portfolio.fees.type.fixed").click();
  const feeInput = dialog.getByTestId("portfolio.fees.fixed.amount");
  await feeInput.fill(String(feeAmount));
  await feeInput.blur();
};

const setVariableFees = async (page, feeRate, minFee, maxFee) => {
  const dialog = page.getByTestId("portfolio.preferences.dialog");
  await dialog.getByTestId("portfolio.fees.type.variable").click();

  const feeRateInput = dialog.getByTestId("portfolio.fees.variable.rate");
  await feeRateInput.fill(String(feeRate));
  await feeRateInput.blur();

  const minFeeInput = dialog.getByTestId("portfolio.fees.variable.min");
  await minFeeInput.fill(String(minFee));
  await minFeeInput.blur();

  const maxFeeInput = dialog.getByTestId("portfolio.fees.variable.max");
  await maxFeeInput.fill(String(maxFee));
  await maxFeeInput.blur();
};

const getFeeValues = async (page) => {
  return await page.evaluate(() => {
    const rows = Array.from(document.querySelectorAll("div")).filter(
      (el) => el.textContent && el.textContent.trim() === "Fees"
    );

    const values = [];
    for (const rowLabel of rows) {
      const row = rowLabel.parentElement;
      if (!row) continue;

      const valueNode = row.querySelector("span, p");
      const text = valueNode?.textContent || "";
      const match = text.match(/([0-9]+(?:[.,][0-9]+)?)/);
      if (!match) continue;
      values.push(Number(match[1].replace(",", ".")));
    }

    return values;
  });
};

test("allocation with zero fees keeps all fee rows at zero", async ({ page }) => {
  await importFixture(page, "fixture-allocate-zero-fee");

  await goToInvestStep(page);
  await setInvestCash(page, 100);
  await runAllocation(page);

  const feeValues = await getFeeValues(page);
  expect(feeValues.length).toBeGreaterThan(0);
  expect(feeValues.every((v) => v === 0)).toBeTruthy();
});

test("allocation with fixed fees applies 19 EUR per executed trade", async ({
  page,
}) => {
  await importFixture(page, "fixture-allocate-fixed-fee-19");

  await openPreferences(page);
  await setFixedFees(page, 19);
  await closePreferences(page);

  await goToInvestStep(page);
  await setInvestCash(page, 100);
  await runAllocation(page);

  const feeValues = await getFeeValues(page);
  expect(feeValues.length).toBeGreaterThan(0);
  expect(feeValues.some((v) => v === 19)).toBeTruthy();
});

test("allocation with variable fees uses configured min/max bounds", async ({
  page,
}) => {
  await importFixture(page, "fixture-allocate-variable-fee");

  await openPreferences(page);
  await setVariableFees(page, 10, 2, 3);
  await closePreferences(page);

  await goToInvestStep(page);
  await setInvestCash(page, 100);
  await runAllocation(page);

  const feeValues = await getFeeValues(page);
  expect(feeValues.length).toBeGreaterThan(0);
  expect(feeValues.some((v) => v > 0)).toBeTruthy();
  expect(feeValues.every((v) => v <= 3)).toBeTruthy();
  expect(feeValues.some((v) => v >= 2)).toBeTruthy();
});

test("disabling tax efficient algorithm allows both buys and sells", async ({
  page,
}) => {
  await importFixture(page, "fixture-allocate-tax-efficient-off");

  await goToInvestStep(page);
  await setInvestCash(page, 0);
  await page.getByTestId("invest.taxEfficient.checkbox").uncheck();
  await runAllocation(page);

  await expect(page.getByText(/\bBuy\b/)).toBeVisible();
  await expect(page.getByText(/\bSell\b/)).toBeVisible();
});
