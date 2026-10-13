import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

/**
 * GIVEN a representative journey story at a mobile or desktop viewport,
 * WHEN Storybook renders it and the journey's key interaction is exercised,
 * THEN the story has no browser errors, accessibility violations, or broken
 * responsive asset presentation.
 */
const journeys = [
  "portfolio overview",
  "add asset",
  "edit and remove asset",
  "portfolio settings",
  "allocate and rebalance",
];

type StoryIndexEntry = {
  id: string;
  name: string;
  title: string;
  type: string;
};

function normalize(value: string) {
  return value.toLowerCase().replace(/[^a-z0-9]/g, "");
}

async function findJourneyStory(page: Page, journey: string) {
  const index = await loadStoryIndex(page);
  const expectedTitle = normalize(`compositions ${journey}`);
  const expectedName = normalize(journey);
  const entries = Object.values(index.entries ?? {}) as StoryIndexEntry[];
  const story =
    entries.find(
      (entry) =>
        entry.type === "story" && normalize(entry.title).includes(expectedTitle)
    ) ??
    entries.find(
      (entry) =>
        entry.type === "story" &&
        normalize(entry.title).includes("compositionsportfoliojourneys") &&
        normalize(entry.name).includes(expectedName)
    );

  expect(story, `Storybook journey story is missing: ${journey}`).toBeTruthy();
  if (!story) throw new Error(`Storybook journey story is missing: ${journey}`);
  return story.id;
}

async function loadStoryIndex(page: Page) {
  const response = await page.request.get("/index.json");
  expect(response.ok()).toBeTruthy();
  return response.json();
}

async function findStory(
  page: Page,
  title: string,
  name: string
): Promise<string> {
  const index = await loadStoryIndex(page);
  const story = (Object.values(index.entries ?? {}) as StoryIndexEntry[]).find(
    (entry) =>
      entry.type === "story" &&
      normalize(entry.title) === normalize(title) &&
      normalize(entry.name) === normalize(name)
  );

  expect(story, `Storybook story is missing: ${title} / ${name}`).toBeTruthy();
  if (!story) throw new Error(`Storybook story is missing: ${title} / ${name}`);
  return story.id;
}

async function findDocs(page: Page, title: string): Promise<string> {
  const index = await loadStoryIndex(page);
  const docs = (Object.values(index.entries ?? {}) as StoryIndexEntry[]).find(
    (entry) =>
      entry.type === "docs" && normalize(entry.title) === normalize(title)
  );

  expect(docs, `Storybook docs entry is missing: ${title}`).toBeTruthy();
  if (!docs) throw new Error(`Storybook docs entry is missing: ${title}`);
  return docs.id;
}

async function expectDocsWithoutOpenDialogs(page: Page, storyId: string) {
  await page.goto(
    `/iframe.html?viewMode=docs&id=${encodeURIComponent(storyId)}`
  );
  await page.waitForLoadState("networkidle");
  const docsRoot = page.locator("#storybook-docs");
  await expect(docsRoot).toBeVisible();
  await expect(docsRoot).not.toBeEmpty();
  await expect(docsRoot.locator('[role="dialog"]:visible')).toHaveCount(0);
  await expect(docsRoot.getByRole("button", { name: "Cancel" })).toHaveCount(0);

  const storyFrames = docsRoot.locator("iframe");
  await expect.poll(() => storyFrames.count()).toBeGreaterThan(0);
  const frameHeights = await storyFrames.evaluateAll((frames) =>
    frames.map((frame) => {
      const attributeHeight = Number(frame.getAttribute("height"));
      const computedHeight = Number.parseFloat(
        window.getComputedStyle(frame).height
      );
      return Number.isFinite(attributeHeight) && attributeHeight > 0
        ? attributeHeight
        : computedHeight;
    })
  );
  expect(Math.max(...frameHeights)).toBeGreaterThanOrEqual(900);
}

test.describe("Storybook journey smoke", () => {
  for (const journey of journeys) {
    test(`renders ${journey} without console errors or accessibility violations`, async ({
      page,
    }, testInfo) => {
      const consoleErrors: string[] = [];
      page.on("console", (message) => {
        if (message.type() === "error") consoleErrors.push(message.text());
      });
      page.on("pageerror", (error) => consoleErrors.push(error.message));

      const storyId = await findJourneyStory(page, journey);
      await page.goto(
        `/iframe.html?id=${encodeURIComponent(storyId)}&viewMode=story`
      );
      await page.waitForLoadState("networkidle");

      const storyRoot = page.locator("#storybook-root");
      await expect(storyRoot).toBeVisible();
      await expect(storyRoot).not.toBeEmpty();
      expect(consoleErrors).toEqual([]);

      if (journey === "add asset") {
        const isMobile = testInfo.project.name === "storybook-mobile";
        const navigationFab = page.getByRole("button", {
          name: "Open portfolio navigation",
        });
        if (isMobile) {
          await expect(navigationFab).toBeVisible();
          await navigationFab.click();
          await expect(
            page.getByRole("heading", { name: "Portfolio navigation" })
          ).toBeVisible();
          const mobileNavigation = page.locator(".ds-workspace-nav--mobile");
          await expect(
            mobileNavigation.getByRole("link", {
              name: "Add asset",
            })
          ).toHaveCount(0);
          await page.keyboard.press("Escape");
          await expect(
            page.getByRole("heading", { name: "Portfolio navigation" })
          ).toBeHidden();
        } else {
          await expect(navigationFab).toHaveCount(0);
        }

        await page.getByRole("button", { name: "Add asset" }).click();
        await expect(
          page.getByRole("heading", { name: "Add an asset" })
        ).toBeVisible();
        await expect(
          page.getByRole("textbox", {
            name: /Search by name or ticker/i,
          })
        ).toBeVisible();
        if (isMobile) {
          await expect(page.getByRole("dialog")).toBeVisible();
          await expect(page.locator(".ds-add-asset-drawer")).toBeVisible();
        } else {
          await expect(page.getByRole("dialog")).toBeVisible();
          await expect(
            page.locator(".ds-dialog-popup.ds-add-asset-dialog")
          ).toBeVisible();
        }

        await page
          .getByRole("textbox", {
            name: /Search by name or ticker/i,
          })
          .fill("VW");
        await expect(
          page.getByRole("region", {
            name: "Asset search results",
          })
        ).toBeVisible();
        await page.getByRole("button", { name: /VWCE/ }).click();
        await expect(
          page.getByRole("region", {
            name: "Asset details",
          })
        ).toBeVisible();
        await expect(page.getByLabel("Target weight")).toBeVisible();
        await page
          .getByRole("button", { name: "Choose another asset" })
          .click();
        await expect(
          page.getByRole("region", { name: "Asset search results" })
        ).toBeVisible();
        await page.getByRole("button", { name: /VWCE/ }).click();
        await page.getByRole("button", { name: "Add asset" }).click();
        await expect(page.getByRole("dialog")).toHaveCount(0);
        await expect(
          page.getByRole("heading", { name: "Core portfolio" })
        ).toBeVisible();

        const pageResults = await new AxeBuilder({ page }).analyze();
        expect(pageResults.violations).toEqual([]);
      }

      if (journey === "portfolio overview") {
        await expect(
          page.locator('button[aria-label="Edit VWCE"]:visible')
        ).toBeVisible();
        await expect(
          page.locator('button[aria-label="More actions for VWCE"]:visible')
        ).toBeVisible();

        const navigationFab = page.getByRole("button", {
          name: "Open portfolio navigation",
        });
        if (testInfo.project.name === "storybook-mobile") {
          await expect(navigationFab).toBeVisible();
          await navigationFab.click();
          await expect(
            page.getByRole("heading", { name: "Portfolio navigation" })
          ).toBeVisible();
          await expect(
            page.getByRole("button", { name: "Close portfolio navigation" })
          ).toBeVisible();
          await page.keyboard.press("Escape");
          await expect(
            page.getByRole("heading", { name: "Portfolio navigation" })
          ).toBeHidden();
        } else {
          await expect(navigationFab).toBeHidden();
        }

        const cards = page.locator(".ds-asset-collection__cards");
        const table = page.locator(".ds-asset-collection__table");
        if (testInfo.project.name === "storybook-mobile") {
          await expect(cards).toBeVisible();
          await expect(table).toBeHidden();
          await expect(
            cards.getByText(/^Quantity(?: owned today)?$/i).first()
          ).toBeVisible();
          await expect(
            cards.getByText("Average cost basis", { exact: true }).first()
          ).toBeVisible();
          await expect(cards.locator(".ds-asset-card__ticker")).toHaveCount(5);
          await expect(cards.locator(".ds-asset-ticker-chip")).toHaveCount(0);
          const visibleCards = cards.locator(".ds-asset-card:visible");
          await expect(visibleCards).toHaveCount(5);
          const firstCard = await visibleCards.nth(0).boundingBox();
          const secondCard = await visibleCards.nth(1).boundingBox();
          expect(firstCard).not.toBeNull();
          expect(secondCard).not.toBeNull();
          if (firstCard && secondCard) {
            expect(
              secondCard.y - (firstCard.y + firstCard.height)
            ).toBeGreaterThan(0);
          }
        } else {
          await expect(cards).toBeHidden();
          await expect(table).toBeVisible();
          await expect(
            table.getByRole("columnheader", {
              name: /^Quantity(?: owned today)?$/i,
            })
          ).toBeVisible();
          await expect(
            table.getByRole("columnheader", { name: "Average cost basis" })
          ).toBeVisible();
          await expect(table.locator(".ds-table__asset-ticker")).toHaveCount(5);
          await expect(table.locator(".ds-asset-ticker-chip")).toHaveCount(0);
        }
      }

      if (journey === "edit and remove asset") {
        await page.getByRole("button", { name: "Edit VWCE" }).click();
        if (testInfo.project.name === "storybook-mobile") {
          await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(1);
          await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(0);
        } else {
          await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(1);
          await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(0);
        }
        await expect(
          page.getByRole("heading", { name: "Edit VWCE" })
        ).toBeVisible();
        await page.keyboard.press("Escape");
        await expect(
          page.getByRole("heading", { name: "Edit VWCE" })
        ).toBeHidden();
      }

      if (journey === "portfolio settings") {
        await expect(
          page.getByRole("heading", { name: "Portfolio settings" })
        ).toBeVisible();
        await expect(page.getByRole("dialog")).toHaveCount(0);
        await page.getByRole("radio", { name: "Strategic" }).click();
        await page
          .getByRole("button", {
            name: "Review strategic allocation",
          })
          .click();
        await expect(
          page.getByRole("heading", { name: "Strategic allocation" })
        ).toBeVisible();
        await expect(page.getByRole("dialog")).toHaveCount(0);
      }

      if (journey === "allocate and rebalance") {
        await page.getByRole("button", { name: /Allocate budget/ }).click();
        await expect(
          page.getByRole("heading", { name: "Your allocation is ready" })
        ).toBeVisible();
        await page.getByRole("button", { name: "Back to options" }).click();
        await expect(
          page.getByRole("heading", {
            name: "How much do you want to allocate?",
          })
        ).toBeVisible();
      }

      const results = await new AxeBuilder({ page }).analyze();
      expect(results.violations).toEqual([]);
    });
  }
});

test("new asset edit keeps select labels and popup geometry aligned", async ({
  page,
}) => {
  const storyId = await findStory(
    page,
    "Compositions / Add asset",
    "New asset edit"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  const select = page.locator(".ds-select-trigger:visible").first();
  await expect(select).toContainText("Default");
  await select.click();
  const popup = page.locator(".ds-select-popup:visible").last();
  await expect(popup).toBeVisible();
  const triggerBox = await select.boundingBox();
  const popupBox = await popup.boundingBox();
  expect(triggerBox).not.toBeNull();
  expect(popupBox).not.toBeNull();
  if (triggerBox && popupBox) {
    expect(Math.abs(popupBox.x - triggerBox.x)).toBeLessThanOrEqual(2);
    expect(Math.abs(popupBox.width - triggerBox.width)).toBeLessThanOrEqual(2);
  }
  const popupZIndex = await popup.evaluate((element) =>
    Number.parseInt(window.getComputedStyle(element).zIndex, 10)
  );
  expect(popupZIndex).toBeGreaterThan(0);
  const tightOption = page
    .locator(".ds-select-item:visible")
    .filter({ hasText: "Tight · ±2 pp" });
  await expect(tightOption).toBeVisible();
  await tightOption.click();
  await expect(select).toContainText("Tight · ±2 pp");
  await page.keyboard.press("Escape");
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations).toEqual([]);
});

test("overview keeps the retired performance chart out of the workspace", async ({
  page,
}) => {
  const storyId = await findStory(
    page,
    "Compositions / Portfolio overview",
    "Populated"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  await expect(page.locator(".ds-chart__currency")).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "Investments" })
  ).toBeVisible();
});

test("first-use overview stages expose placeholders and setup progress", async ({
  page,
}) => {
  const emptyId = await findStory(
    page,
    "Compositions / Portfolio overview",
    "First use"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(emptyId)}`);
  await page.waitForLoadState("networkidle");
  await expect(
    page.locator('.ds-metric-card[data-placeholder="true"]')
  ).toHaveCount(3);
  await expect(
    page.locator('.ds-portfolio-setup__step[data-status="complete"]')
  ).toHaveCount(0);

  const assetStageId = await findStory(
    page,
    "Compositions / Portfolio overview",
    "First use asset added"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(assetStageId)}`);
  await page.waitForLoadState("networkidle");
  await expect(
    page.locator('.ds-metric-card[data-placeholder="true"]')
  ).toHaveCount(0);
  await expect(
    page.locator('.ds-portfolio-setup__step[data-status="complete"]')
  ).toHaveCount(2);
  await expect(page.getByText("1 asset", { exact: true })).toBeVisible();
});

test("portfolio selector keeps overview content and navigation synchronized", async ({
  page,
}, testInfo) => {
  const storyId = await findStory(
    page,
    "Compositions / Portfolio overview",
    "Populated"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  await page
    .getByRole("button", {
      name: /Select portfolio, current portfolio Core portfolio/i,
    })
    .click();
  await page.getByRole("menuitem", { name: "Emergency fund" }).click();

  await expect(
    page.getByRole("heading", { name: "Emergency fund" })
  ).toBeVisible();
  await expect(
    page.locator(".ds-portfolio-workspace-sidebar__heading strong")
  ).toHaveText("Emergency fund");
  const assetNames = page.locator(
    testInfo.project.name === "storybook-mobile"
      ? ".ds-asset-card__name"
      : ".ds-table__asset-name"
  );
  await expect(assetNames).toHaveText(["Cash"]);
});

test("asset collection landmarks follow their visible heading", async ({
  page,
}) => {
  const storyId = await findStory(
    page,
    "Patterns / Portfolio",
    "Portfolio workspace shell"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  const collection = page.locator("section.ds-asset-collection").first();
  const headingId = await collection.getAttribute("aria-labelledby");
  expect(headingId).toBeTruthy();
  if (headingId) {
    await expect(page.locator(`[id="${headingId}"]`)).toHaveText("Holdings");
  }
});

test("first-use setup unlocks steps in order and does not restore step one", async ({
  page,
}) => {
  const storyId = await findStory(
    page,
    "Compositions / Portfolio overview",
    "First use"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  await expect(
    page.getByRole("button", { name: /Add your first asset/ })
  ).toBeDisabled();
  await expect(
    page.getByRole("button", { name: /Set target weights/ })
  ).toBeDisabled();

  await page.getByRole("button", { name: /Name and currency/ }).click();
  await page.getByRole("textbox", { name: "Portfolio name" }).fill("Starter");
  await page.getByRole("combobox", { name: "Base currency" }).click();
  await page.getByRole("option", { name: /EUR · Euro/ }).click();
  await page.getByRole("button", { name: "Continue" }).click();
  await expect(
    page.getByRole("heading", { name: "Add your first asset" })
  ).toBeVisible();

  await page.getByRole("button", { name: "Close new portfolio" }).click();
  await expect(page.getByText("Step 1 of 3", { exact: true })).toBeHidden();
  await expect(
    page.getByRole("button", { name: /Add your first asset/ })
  ).toBeEnabled();
  await expect(
    page.getByRole("button", { name: /Set target weights/ })
  ).toBeDisabled();
});

test("strategic allocation exposes human-readable drift labels", async ({
  page,
}) => {
  const storyId = await findStory(
    page,
    "Compositions / Portfolio settings",
    "Strategic Allocation"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  const driftTriggers = page.locator(
    ".ds-strategic-allocation__row .ds-select-trigger:visible"
  );
  await expect(driftTriggers).toHaveCount(6);
  await expect(driftTriggers.first()).toContainText("±5 pp");
  const driftLabels = await driftTriggers.allTextContents();
  expect(driftLabels.join(" ")).not.toContain("five");
});

test("allocation review exposes amount, fees, weight, and signed deltas", async ({
  page,
}) => {
  const storyId = await findStory(
    page,
    "Compositions / Allocate and rebalance",
    "Allocation Result"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  for (const label of [
    "Amount",
    "Weight after allocation",
    "Commission estimate",
    "Estimated first-year fee impact",
  ]) {
    await expect(page.getByText(label, { exact: true })).toBeVisible();
  }
  await expect(page.getByText("Buy 195 AGGH", { exact: true })).toBeVisible();
  await expect(page.getByText("€998.40", { exact: true })).toBeVisible();
  await expect(page.getByText("10.2%", { exact: true })).toBeVisible();
  await expect(page.getByText("+8.3 pp", { exact: true })).toBeVisible();
  await expect(page.getByText("€1.00", { exact: true })).toBeVisible();
  await expect(page.getByText("€0.18", { exact: true })).toBeVisible();
  await expect(page.getByText("€0.60", { exact: true })).toBeVisible();
  await expect(page.getByText("€3,842.75", { exact: true })).toBeVisible();
});

// GIVEN the displayed holdings and a €1,000 contribution, WHEN the buy-only
// rebalance review is rendered, THEN purchases, fees, and cash reconcile.
test("rebalance review reconciles buy-only trades and cash", async ({
  page,
}) => {
  const storyId = await findStory(
    page,
    "Compositions / Allocate and rebalance",
    "Rebalance Result"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  await expect(page.getByText("Sell", { exact: true })).toHaveCount(0);
  await expect(page.getByText("Buy", { exact: true }).first()).toBeVisible();
  await expect(page.getByText("€747.52", { exact: true })).toBeVisible();
  await expect(page.getByText("€188.70", { exact: true })).toBeVisible();
  await expect(
    page.getByText("Cash balance after", { exact: true })
  ).toBeVisible();
  await expect(page.getByText("€3,903.93", { exact: true })).toBeVisible();
  await expect(
    page.getByText("Unallocated budget", { exact: true })
  ).toBeVisible();
  await expect(page.getByText("€61.78", { exact: true })).toBeVisible();
  await expect(
    page.getByText(
      /€1,000.00 budget · €936.22 invested · €2.00 estimated commission/
    )
  ).toBeVisible();
});

// GIVEN a rebalance journey at either viewport, WHEN the investor reviews the
// proposed changes, THEN weight comparison and estimated transactions are clear
// and they can return to the options before confirming.
test("rebalance options progress to a responsive review and return", async ({
  page,
}, testInfo) => {
  const mobile = testInfo.project.name === "storybook-mobile";
  const storyId = await findStory(
    page,
    "Compositions / Allocate and rebalance",
    mobile ? "Rebalance Options Mobile" : "Rebalance Options"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  await expect(
    page.getByRole("heading", { name: "Rebalance to your target allocation" })
  ).toBeVisible();
  await expect(page.getByLabel("Amount to invest")).toHaveValue("1,000");
  await expect(
    page.getByText("No existing assets will be sold.", { exact: false })
  ).toBeVisible();
  if (mobile) {
    const primaryButton = await page
      .getByRole("button", { name: "Review rebalance" })
      .boundingBox();
    const navigationFab = await page
      .locator(".ds-portfolio-workspace__navigation-fab")
      .boundingBox();
    expect(primaryButton).not.toBeNull();
    expect(navigationFab).not.toBeNull();
    const overlaps =
      primaryButton!.x < navigationFab!.x + navigationFab!.width &&
      primaryButton!.x + primaryButton!.width > navigationFab!.x &&
      primaryButton!.y < navigationFab!.y + navigationFab!.height &&
      primaryButton!.y + primaryButton!.height > navigationFab!.y;
    expect(overlaps).toBe(false);
  }
  await page.getByRole("button", { name: "Review rebalance" }).click();

  await expect(
    page.getByRole("heading", { name: "Review your rebalance" })
  ).toBeVisible();
  await expect(
    page.getByRole("table", { name: "Rebalance weight comparison" })
  ).toBeVisible();
  await expect(
    page.getByText("Proposed purchases", { exact: true })
  ).toBeVisible();
  await expect(
    page.getByText("Quantity", { exact: true }).first()
  ).toBeVisible();
  await expect(
    page.getByText("Commission estimate", { exact: true }).first()
  ).toBeVisible();

  if (mobile) {
    await expect(
      page.getByRole("button", { name: "Back to options" })
    ).toBeVisible();
    const documentWidth = await page
      .locator("body")
      .evaluate((node) => node.scrollWidth);
    const viewportWidth = await page.evaluate(() => window.innerWidth);
    expect(documentWidth).toBeLessThanOrEqual(viewportWidth);
  }
  await page.getByRole("button", { name: "Back to options" }).click();
  await expect(
    page.getByRole("heading", { name: "Rebalance to your target allocation" })
  ).toBeVisible();

  await page.getByRole("button", { name: "Review rebalance" }).click();
  await page.getByRole("button", { name: "Confirm rebalance" }).click();
  await expect(
    page.getByRole("status").getByText("Rebalance confirmed", { exact: true })
  ).toBeVisible();

  if (mobile) {
    const primaryButton = await page
      .getByRole("button", { name: "Rebalance confirmed" })
      .boundingBox();
    const navigationFab = await page
      .locator(".ds-portfolio-workspace__navigation-fab")
      .boundingBox();
    expect(primaryButton).not.toBeNull();
    expect(navigationFab).not.toBeNull();
    const overlaps =
      primaryButton!.x < navigationFab!.x + navigationFab!.width &&
      primaryButton!.x + primaryButton!.width > navigationFab!.x &&
      primaryButton!.y < navigationFab!.y + navigationFab!.height &&
      primaryButton!.y + primaryButton!.height > navigationFab!.y;
    expect(overlaps).toBe(false);
  }

  await page
    .getByRole("button", { name: /Select portfolio, current portfolio/ })
    .click();
  await page.getByRole("menuitem", { name: "Long-term growth" }).click();
  await expect(
    page.getByRole("status").getByText("Rebalance confirmed", { exact: true })
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Confirm rebalance" })
  ).toBeEnabled();
});

// GIVEN buy-only is disabled and a larger budget is entered, WHEN the investor
// reviews the rebalance, THEN overweight assets can be sold and the resulting
// cash summary describes the combined purchases and sales.
test("rebalance review supports sell-capable trades when buy-only is disabled", async ({
  page,
}) => {
  const storyId = await findStory(
    page,
    "Compositions / Allocate and rebalance",
    "Rebalance Options"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  await page.getByLabel("Amount to invest").fill("500");
  await page.getByRole("checkbox", { name: /Buy only/ }).uncheck();
  await page.getByRole("button", { name: "Review rebalance" }).click();

  await expect(
    page.getByRole("heading", { name: "Proposed transactions" })
  ).toBeVisible();
  await expect(page.getByText("Sell", { exact: true })).toBeVisible();
  await expect(
    page.getByText("Cash after rebalance", { exact: true })
  ).toBeVisible();
  await expect(page.getByText("€1,091.87", { exact: true })).toBeVisible();
});

// GIVEN an editable allocation budget, WHEN it is raised to €2,000, THEN the
// whole-unit purchase, commission, and cash remainder reflect the entered value.
test("allocation preview recalculates for the entered investment budget", async ({
  page,
}) => {
  const storyId = await findStory(
    page,
    "Compositions / Allocate and rebalance",
    "Allocation options"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");
  await page.getByLabel("Amount to invest").fill("2,000");
  await page.getByRole("button", { name: "Allocate budget" }).click();

  await expect(
    page.getByText("Buy 390 AGGH @ €5.12", { exact: true })
  ).toBeVisible();
  await expect(page.getByText("€1,996.80", { exact: true })).toBeVisible();
  await expect(page.getByText("€2.20", { exact: true })).toBeVisible();
});

// GIVEN a new-budget allocation, WHEN its result is reviewed on mobile, THEN
// the proposal remains stacked and buy-only, with costs and remaining cash.
test("allocation result remains readable on mobile without selling holdings", async ({
  page,
}, testInfo) => {
  test.skip(
    testInfo.project.name !== "storybook-mobile",
    "The stacked allocation result is a mobile-only contract."
  );
  const storyId = await findStory(
    page,
    "Compositions / Allocate and rebalance",
    "Allocation Result Mobile"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  await expect(
    page.getByRole("heading", { name: "Your allocation is ready" })
  ).toBeVisible();
  await expect(
    page.getByText("Commission estimate", { exact: true })
  ).toBeVisible();
  await expect(
    page.getByText("Unallocated cash", { exact: false })
  ).toBeVisible();
  const documentWidth = await page
    .locator("body")
    .evaluate((node) => node.scrollWidth);
  const viewportWidth = await page.evaluate(() => window.innerWidth);
  expect(documentWidth).toBeLessThanOrEqual(viewportWidth);
});

test.describe("latest design-system polish", () => {
  test("fixed fee configuration uses one nested responsive overlay and restores focus", async ({
    page,
  }, testInfo) => {
    const storyId = await findStory(
      page,
      "Compositions / Add asset",
      "New asset edit"
    );
    await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
    await page.waitForLoadState("networkidle");

    const feeTrigger = page.getByRole("button", {
      name: "Edit transaction fees",
    });

    // Dismissal should discard a policy choice that has not been saved.
    await feeTrigger.click();
    await page.getByRole("radio", { name: "Fixed" }).click();
    await page.keyboard.press("Escape");
    await expect(page.locator(".ds-fee-summary:visible")).toContainText(
      "Default"
    );

    await feeTrigger.click();
    await page.getByRole("radio", { name: "Fixed" }).click();
    await page.getByLabel("Maximum impact").fill("9");
    await page.getByLabel("Amount").fill("99");
    await page.getByRole("radio", { name: "Default" }).click();
    await expect(page.locator(".ds-fee-configuration:visible")).toContainText(
      "maximum impact 0.5% · amount €2.50"
    );
    await page.keyboard.press("Escape");
    await expect(page.locator(".ds-fee-summary:visible")).toContainText(
      "Default"
    );

    await feeTrigger.click();
    await page.getByRole("radio", { name: "Fixed" }).click();

    if (testInfo.project.name === "storybook-mobile") {
      await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(0);
      await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(1);
    } else {
      await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(1);
      await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(0);
    }
    await expect(page.locator(".ds-fee-configuration:visible")).toContainText(
      "Maximum impact"
    );
    await expect(
      page.locator(".ds-fee-configuration:visible .ds-input-with-suffix__chip")
    ).toHaveText(["%", "EUR"]);
    await expect(
      page.locator(".ds-fee-configuration:visible").getByRole("button", {
        name: "Save",
      })
    ).toBeVisible();

    await page
      .locator(".ds-fee-configuration:visible")
      .getByRole("button", { name: "Save" })
      .click();
    await expect(page.locator(".ds-fee-configuration:visible")).toHaveCount(0);
    await expect(feeTrigger).toBeFocused();

    await feeTrigger.click();
    await page.getByRole("radio", { name: "Variable" }).click();
    await expect(page.locator(".ds-fee-configuration:visible")).toContainText(
      "Variable fees are estimated"
    );
    await page.getByLabel("Min fee").fill("50");
    await page.getByLabel("Max fee").fill("25");
    await expect(page.getByLabel("Min fee")).toHaveValue("50");
    await expect(page.getByLabel("Max fee")).toHaveValue("25");
    await expect(page.getByRole("alert")).toContainText(
      "Minimum fee must not exceed maximum fee"
    );
    await page
      .locator(".ds-fee-configuration:visible")
      .getByRole("button", { name: "Save" })
      .click();
    await expect(page.locator(".ds-fee-configuration:visible")).toHaveCount(1);
    await page.keyboard.press("Escape");
    await expect(page.locator(".ds-fee-configuration:visible")).toHaveCount(0);
    await expect(feeTrigger).toBeFocused();
    await expect(page.locator(".ds-fee-summary:visible")).toContainText(
      "Fixed"
    );
  });

  test("allocation footers adapt navigation and action layout to mobile", async ({
    page,
  }, testInfo) => {
    const mobile = testInfo.project.name === "storybook-mobile";
    const optionsId = await findStory(
      page,
      "Compositions / Allocate and rebalance",
      "Allocation options"
    );
    await page.goto(`/iframe.html?id=${encodeURIComponent(optionsId)}`);
    await page.waitForLoadState("networkidle");
    const optionsFooter = page.locator(".ds-allocation-footer:visible");
    await expect(optionsFooter).toHaveCount(1);
    const optionsButtons = optionsFooter.getByRole("button");
    if (mobile) {
      await expect(optionsButtons).toHaveCount(1);
      await expect(optionsButtons.first()).not.toHaveClass(/ds-button--ghost/);
    } else {
      await expect(optionsButtons).toHaveCount(2);
      await expect(optionsButtons.first()).toHaveClass(/ds-button--ghost/);
      const optionsLayout = await optionsButtons.evaluateAll((buttons) =>
        buttons.map((button) => button.getBoundingClientRect().x)
      );
      expect(optionsLayout[0]).toBeLessThan(optionsLayout[1]);
    }

    const resultId = await findStory(
      page,
      "Compositions / Allocate and rebalance",
      "Allocation result"
    );
    await page.goto(`/iframe.html?id=${encodeURIComponent(resultId)}`);
    await page.waitForLoadState("networkidle");
    const resultFooter = page.locator(".ds-allocation-footer:visible");
    const resultButtons = resultFooter.getByRole("button");
    if (mobile) {
      await expect(resultButtons).toHaveCount(1);
      await expect(resultButtons.first()).not.toHaveClass(/ds-button--ghost/);
    } else {
      await expect(resultButtons).toHaveCount(2);
      await expect(resultButtons.first()).toHaveClass(/ds-button--ghost/);
      const resultLayout = await resultButtons.evaluateAll((buttons) =>
        buttons.map((button) => button.getBoundingClientRect().x)
      );
      expect(resultLayout[0]).toBeLessThan(resultLayout[1]);
    }
  });

  test("strategic mobile rows expose independent target and drift labels", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name !== "storybook-mobile",
      "The stacked labels are a mobile-only contract."
    );
    const storyId = await findStory(
      page,
      "Compositions / Portfolio settings",
      "Strategic allocation mobile"
    );
    await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
    await page.waitForLoadState("networkidle");
    await expect(
      page
        .locator(".ds-strategic-allocation__field-label:visible")
        .filter({ hasText: "Target weight" })
    ).toHaveCount(6);
    await expect(
      page
        .locator(".ds-strategic-allocation__field-label:visible")
        .filter({ hasText: "Drift band" })
    ).toHaveCount(6);
  });

  test("allocation method uses a selected radio card and fee controls stay segmented", async ({
    page,
  }) => {
    const storyId = await findStory(
      page,
      "Compositions / Portfolio settings",
      "Allocation method open"
    );
    await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
    await page.waitForLoadState("networkidle");
    const selected = page.locator(
      ".ds-radio-card-group__option:has(.ds-radio-card-group__radio[data-checked])"
    );
    await expect(selected).toHaveCount(1);
    await expect(selected).toContainText("Simple");
    await expect(page.locator(".ds-segmented-control:visible")).toContainText(
      "Zero"
    );
    await expect(page.locator(".ds-segmented-control:visible")).toContainText(
      "Variable"
    );
  });

  test("portfolio fee policies reveal the legacy forms inline", async ({
    page,
  }) => {
    const storyId = await findStory(
      page,
      "Compositions / Portfolio settings",
      "Allocation method open"
    );
    await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
    await page.waitForLoadState("networkidle");

    await page.getByRole("radio", { name: "Fixed" }).click();
    await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(0);
    await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(0);
    await expect(page.getByLabel("Maximum impact")).toBeVisible();
    await expect(page.getByLabel("Amount")).toBeVisible();

    await page.getByRole("radio", { name: "Variable" }).click();
    for (const label of [
      "Maximum impact",
      "Fee percentage",
      "Min fee",
      "Max fee",
    ]) {
      await expect(page.getByLabel(label)).toBeVisible();
    }
    await expect(
      page.getByText("Variable fees are estimated", { exact: false })
    ).toBeVisible();
    await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(0);
    await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(0);
  });

  test("portfolio floating action opens navigation on mobile and is hidden on desktop", async ({
    page,
  }, testInfo) => {
    const isMobile = testInfo.project.name === "storybook-mobile";
    const storyId = await findStory(
      page,
      "Patterns / Portfolio",
      isMobile ? "Floating Actions Mobile" : "Floating Actions Desktop"
    );
    await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
    await page.waitForLoadState("networkidle");

    const navigationFab = page.getByRole("button", {
      name: "Open portfolio navigation",
    });
    if (isMobile) {
      await expect(navigationFab).toBeVisible();
      await navigationFab.click();
      await expect(
        page.getByRole("heading", { name: "Portfolio navigation" })
      ).toBeVisible();
      await expect(
        page.locator(".ds-workspace-nav--mobile").getByRole("link", {
          name: "Add asset",
        })
      ).toHaveCount(0);
    } else {
      await expect(navigationFab).toHaveCount(0);
    }
    await expect(
      page.getByRole("button", { name: "Open portfolio actions" })
    ).toHaveCount(0);
  });

  test("disabled switches keep a colored track and thumb", async ({ page }) => {
    const storyId = await findStory(page, "UI / Primitives", "Disabled switch");
    await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
    await page.waitForLoadState("networkidle");
    const disabledSwitch = page.getByRole("switch", { name: "Enable alerts" });
    const colors = await disabledSwitch.evaluate((element) => {
      const style = window.getComputedStyle(element);
      const thumb = window.getComputedStyle(element, "::after");
      return {
        track: style.backgroundColor,
        thumb: thumb.backgroundColor,
        border: style.borderColor,
      };
    });
    expect(colors.track).not.toBe("rgba(0, 0, 0, 0)");
    expect(colors.thumb).not.toBe("rgb(255, 255, 255)");
    expect(colors.border).not.toBe("rgba(0, 0, 0, 0)");
  });

  test("untoggled switches use the violet thumb treatment", async ({
    page,
  }) => {
    const storyId = await findStory(page, "UI / Primitives", "Form states");
    await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
    await page.waitForLoadState("networkidle");
    const toggle = page.getByRole("switch", { name: "Enable alerts" });
    const thumb = await toggle.evaluate(
      (element) => window.getComputedStyle(element, "::after").backgroundColor
    );
    expect(thumb).not.toBe("rgb(255, 255, 255)");
  });

  test("every button variant exposes a persistent pressed state", async ({
    page,
  }) => {
    const storyId = await findStory(
      page,
      "UI / Primitives",
      "Button pressed states"
    );
    await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
    await page.waitForLoadState("networkidle");
    const pressedButtons = page.locator('button[data-pressed="true"]');
    await expect(pressedButtons).toHaveCount(6);
    expect(
      await pressedButtons.evaluateAll((buttons) =>
        buttons.every(
          (button) => button.getAttribute("aria-pressed") === "true"
        )
      )
    ).toBe(true);
  });
});

test.describe("Storybook docs and overlay smoke", () => {
  test("journey docs show closed overview states without stacked dialogs", async ({
    page,
  }) => {
    const docsStories = [
      "Compositions / Portfolio overview",
      "Compositions / Add asset",
      "Compositions / Edit and remove asset",
      "Compositions / Portfolio settings",
      "Compositions / Allocate and rebalance",
    ] as const;

    for (const title of docsStories) {
      const docsId = await findDocs(page, title);
      await expectDocsWithoutOpenDialogs(page, docsId);
    }
  });

  for (const storyName of ["Transaction fees", "Remove confirmation"]) {
    test(`${storyName} opens only its active overlay`, async ({
      page,
    }, testInfo) => {
      const storyId = await findStory(
        page,
        "Compositions / Edit and remove asset",
        storyName
      );
      await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
      await page.waitForLoadState("networkidle");

      if (storyName === "Remove confirmation") {
        await expect(page.getByRole("dialog")).toHaveCount(1);
      } else if (testInfo.project.name === "storybook-mobile") {
        await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(1);
        await expect(page.getByRole("dialog")).toHaveCount(0);
      } else {
        await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(1);
      }
      await expect(page.getByRole("button", { name: "Cancel" })).toHaveCount(0);
      await page.keyboard.press("Escape");
      await expect(page.getByRole("dialog")).toBeHidden();
      await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(0);
    });
  }
});
