import { expect, test } from "../support/fixtures";
import {
  createPortfolioFixtures,
  makeAsset,
  makePortfolio,
  seedPersistedState,
} from "../support/state";

const editablePortfolio = makePortfolio({
  id: "editable-portfolio",
  name: "Editable portfolio",
  assets: {
    VWCE: makeAsset({
      symbol: "VWCE",
      name: "Equity asset",
      qty: 1,
      amount: 100,
      weight: 50,
      targetWeight: 40,
      averageBuyPrice: 90,
    }),
    AGGH: makeAsset({
      idx: 1,
      symbol: "AGGH",
      name: "Bond asset",
      price: 50,
      qty: 2,
      amount: 100,
      weight: 50,
      targetWeight: 60,
      averageBuyPrice: 50,
    }),
  },
  totalAmount: 200,
});

test("an investor edits quantities, prices, and allocation weights", async ({
  page,
}) => {
  /*
   * GIVEN an investor has an existing two-asset portfolio
   * WHEN they change quantity, average buy price, and target weights
   * THEN the editor recalculates amounts and permits the exact 100% allocation
   */
  await seedPersistedState(page, {
    portfolios: { [editablePortfolio.id]: editablePortfolio },
    selected: editablePortfolio.id,
    step: 30,
    preferredCurrency: "usd",
  });
  await page.goto("/allocate");

  await expect(page.getByTestId("portfolio-search")).toHaveCSS(
    "background-color",
    "rgb(255, 255, 255)"
  );
  await expect(page.getByTestId("portfolio-search")).toHaveCSS(
    "font-size",
    "16px"
  );
  await expect(page.getByTestId("portfolio-search")).toHaveCSS(
    "padding-top",
    "0px"
  );
  await expect(page.getByTestId("portfolio-search")).toHaveCSS(
    "padding-bottom",
    "0px"
  );
  await expect(page.getByTestId("portfolio-preferences")).toHaveCSS(
    "cursor",
    "pointer"
  );
  await expect(page.locator("button:has(svg.lucide-save)")).toHaveCSS(
    "cursor",
    "pointer"
  );
  await expect(page.getByRole("button", { name: "Go back" })).toHaveCSS(
    "cursor",
    "pointer"
  );
  await expect(page.getByRole("button", { name: "Confirm weights" })).toHaveCSS(
    "cursor",
    "pointer"
  );

  const mwrHelp = page.getByRole("button", { name: "MWR" });
  await expect(mwrHelp).toHaveCSS("cursor", "pointer");
  if ((page.viewportSize()?.width ?? 1024) < 768) {
    const helpDrawer = page.getByRole("dialog");
    await mwrHelp.click();
    await expect(helpDrawer).toBeVisible();
    await expect(helpDrawer).toContainText(
      /Money-Weighted Return|Rendimento ponderato/
    );
    await page.keyboard.press("Escape");
    await expect(helpDrawer).toBeHidden();
  } else {
    const tooltip = page.getByRole("tooltip");
    await mwrHelp.click();
    await expect(tooltip).toBeVisible({ timeout: 500 });
    await expect(mwrHelp).toHaveAttribute("aria-expanded", "true");
    await mwrHelp.click();
    await expect(mwrHelp).toHaveAttribute("aria-expanded", "false");
    await expect(tooltip).toBeHidden();
    await page.mouse.move(40, 120);
    await mwrHelp.hover();
    await expect(tooltip).toBeVisible({ timeout: 500 });
    await page.keyboard.press("Escape");
    await expect(mwrHelp).toHaveAttribute("aria-expanded", "false");
    await expect(tooltip).toBeHidden();
    await mwrHelp.click();
    await expect(tooltip).toBeVisible({ timeout: 500 });
    await page.mouse.click(40, 120);
    await expect(mwrHelp).toHaveAttribute("aria-expanded", "false");
    await expect(tooltip).toBeHidden();
  }

  const equity = page.locator('[data-testid="asset-card"][data-symbol="VWCE"]');
  const bond = page.locator('[data-testid="asset-card"][data-symbol="AGGH"]');
  await expect(equity).toBeVisible();
  await expect(bond).toBeVisible();

  await equity.getByRole("spinbutton").nth(0).fill("2");
  await equity.getByRole("spinbutton").nth(0).blur();
  await equity.getByRole("spinbutton").nth(1).fill("95");
  await equity.getByRole("spinbutton").nth(1).blur();
  await equity.getByRole("spinbutton").nth(2).fill("50");
  await equity.getByRole("spinbutton").nth(2).blur();
  await bond.getByRole("spinbutton").nth(2).fill("50");
  await bond.getByRole("spinbutton").nth(2).blur();

  await expect(
    page.getByRole("button", { name: "Confirm weights" })
  ).toBeEnabled();
});

test("the editor distinguishes under, exact, and over allocation", async ({
  page,
}) => {
  /*
   * GIVEN an investor is reviewing target weights
   * WHEN the weights are under, exactly, and over 100%
   * THEN the editor clearly marks the invalid totals and enables the valid one
   */
  await seedPersistedState(page, {
    portfolios: { [editablePortfolio.id]: editablePortfolio },
    selected: editablePortfolio.id,
    step: 30,
  });
  await page.goto("/allocate");

  const equity = page.locator('[data-testid="asset-card"][data-symbol="VWCE"]');
  const bond = page.locator('[data-testid="asset-card"][data-symbol="AGGH"]');
  const message = page.getByText(/Target weights must sum up to 100%/);

  await equity.getByRole("spinbutton").nth(2).fill("30");
  await equity.getByRole("spinbutton").nth(2).blur();
  await expect(message).toContainText("90");
  await expect(
    page.getByRole("button", { name: "Confirm weights" })
  ).toBeDisabled();
  await expect(page.getByRole("button", { name: "Confirm weights" })).toHaveCSS(
    "cursor",
    "not-allowed"
  );

  await equity.getByRole("spinbutton").nth(2).fill("40");
  await equity.getByRole("spinbutton").nth(2).blur();
  await expect(message).toHaveCount(0);

  await bond.getByRole("spinbutton").nth(2).fill("70");
  await bond.getByRole("spinbutton").nth(2).blur();
  await expect(message).toContainText("110");
});

test("portfolio cards show positive, negative, and zero gain", async ({
  page,
}) => {
  /*
   * GIVEN an investor has portfolios with different market performance
   * WHEN the portfolio list opens
   * THEN positive, negative, and zero gains use the matching visible values
   */
  const fixtures = createPortfolioFixtures();
  const portfolios = Object.fromEntries(
    Object.values(fixtures).map((portfolio) => [portfolio.id, portfolio])
  );
  await seedPersistedState(page, {
    portfolios,
    selected: null,
    step: 10,
  });
  await page.goto("/allocate");

  await expect(
    page.getByTestId("portfolio-card").filter({ hasText: "Positive gain" })
  ).toContainText("+10.00%");
  await expect(
    page.getByTestId("portfolio-card").filter({ hasText: "Negative gain" })
  ).toContainText("-10.00%");
  await expect(
    page.getByTestId("portfolio-card").filter({ hasText: "Zero gain" })
  ).toContainText("0.00%");
});

test("an investor can remove the last asset and return to portfolios", async ({
  page,
}) => {
  /*
   * GIVEN an investor is editing a one-asset portfolio
   * WHEN they remove the last asset and go back
   * THEN the portfolio list is restored without an empty editor stuck on screen
   */
  const portfolio = makePortfolio({ id: "remove-last", name: "Remove last" });
  await seedPersistedState(page, {
    portfolios: { [portfolio.id]: portfolio },
    selected: portfolio.id,
    step: 30,
  });
  await page.goto("/allocate");

  await page.getByRole("button", { name: "Remove VWCE.MI" }).click();
  await expect(page.getByTestId("asset-card")).toHaveCount(0);
  await page.getByText("Go back", { exact: true }).click();
  await expect(page.getByTestId("portfolio-card")).toHaveCount(1);
});
