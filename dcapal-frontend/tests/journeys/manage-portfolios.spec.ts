import { expect, test } from "../support/fixtures";
import type { Page } from "@playwright/test";
import { makePortfolio, seedPersistedState } from "../support/state";

const seedList = async (page: Page): Promise<void> => {
  const portfolio = makePortfolio({
    id: "managed-portfolio",
    name: "Managed portfolio",
  });
  await seedPersistedState(page, {
    portfolios: { [portfolio.id]: portfolio },
    selected: null,
    step: 10,
  });
  await page.goto("/allocate");
  await expect(page.getByTestId("portfolio-card")).toHaveCount(1);
};

test("an investor can cancel a new portfolio and then create one", async ({
  page,
}) => {
  /*
   * GIVEN an investor already has a saved portfolio
   * WHEN they open and cancel a new portfolio form, then create a named EUR portfolio
   * THEN the list remains usable and the new portfolio can be saved
   */
  await seedList(page);
  await page.getByRole("button", { name: "New portfolio" }).click();
  const form = page.getByTestId("new-portfolio-form");
  await form.getByRole("button", { name: "Cancel" }).click();
  await expect(form).toBeHidden();

  await page.getByRole("button", { name: "New portfolio" }).click();
  await form.getByRole("textbox").fill("Retirement EUR");
  await form.getByTestId("ccyRadio").filter({ hasText: "eur" }).click();
  await form.getByRole("button", { name: "Next" }).click();
  await expect(page.getByTestId("portfolio-editor")).toBeVisible();
  await page.getByText("Go back", { exact: true }).click();
  await expect(page.getByText("Retirement EUR")).toBeVisible();
});

test("an investor can rename, cancel, duplicate, delete, and reload", async ({
  page,
  browser,
}) => {
  /*
   * GIVEN an investor has a saved portfolio
   * WHEN they edit its name, cancel an edit, duplicate it, delete the copy, and reload
   * THEN the visible portfolio list preserves the saved outcome
   */
  await seedList(page);
  const card = page.getByTestId("portfolio-card").first();

  await card.getByTestId("portfolio-edit").click();
  await card.getByRole("textbox").fill("Renamed portfolio");
  await card.getByRole("button", { name: "Save" }).click();
  await expect(card).toContainText("Renamed portfolio");

  await card.getByTestId("portfolio-edit").click();
  await card.getByRole("textbox").fill("Discarded name");
  await card.getByTestId("portfolio-edit").click();
  await expect(card).toContainText("Renamed portfolio");

  await card.getByTestId("portfolio-edit").click();
  await card.getByRole("button", { name: "Duplicate" }).click();
  await expect(page.getByTestId("portfolio-card")).toHaveCount(2);
  const copy = page.getByTestId("portfolio-card").nth(1);
  await expect(copy).toContainText("(copy)");

  await copy.getByTestId("portfolio-edit").click();
  await copy.getByRole("button", { name: "Delete" }).click();
  await expect(page.getByTestId("portfolio-card")).toHaveCount(1);

  await page.waitForTimeout(250);
  const reopenedContext = await browser.newContext({
    storageState: await page.context().storageState(),
  });
  const reopenedPage = await reopenedContext.newPage();
  try {
    await reopenedPage.goto("/allocate");
    await expect(reopenedPage.getByTestId("portfolio-card")).toContainText(
      "Renamed portfolio"
    );
  } finally {
    await reopenedContext.close();
  }
});
