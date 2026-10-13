import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";
import { formatCurrency } from "../src/design-system/fixtures/portfolio";
import {
  addMoneyProposal,
  addMoneyResult,
  rebalanceProposal,
  suggestedAmountStates,
} from "../src/design-system/fixtures/invest";
import { formatPoints } from "../src/design-system/components/patterns/invest-format";

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
  "invest",
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

      if (journey === "invest") {
        await page.getByRole("button", { name: "Add money" }).click();
        await page.getByRole("button", { name: "Continue" }).click();
        await expect(
          page.getByRole("button", { name: "Update portfolio" })
        ).toBeVisible();
        await page.getByRole("button", { name: "Back to rules" }).click();
        await expect(
          page.getByRole("button", { name: "Continue" })
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

const INVEST_TITLE = "Compositions / Invest";

/** Opens an Invest composition story, using the mobile sibling on the phone project. */
async function openInvest(
  page: Page,
  name: string,
  options: { mobileVariant?: boolean } = {}
) {
  const mobile = options.mobileVariant ?? false;
  const storyId = await findStory(
    page,
    INVEST_TITLE,
    mobile ? `${name} Mobile` : name
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");
}

/** The first rendered match of an exact text, ignoring responsive duplicates that are hidden. */
function visibleText(page: Page, text: string): Locator {
  return page.getByText(text, { exact: true }).locator("visible=true").first();
}

async function expectNoHorizontalOverflow(page: Page) {
  const documentWidth = await page
    .locator("body")
    .evaluate((node) => node.scrollWidth);
  const viewportWidth = await page.evaluate(() => window.innerWidth);
  expect(documentWidth).toBeLessThanOrEqual(viewportWidth);
}

async function overlaps(first: Locator, second: Locator) {
  const a = await first.boundingBox();
  const b = await second.boundingBox();
  expect(a).not.toBeNull();
  expect(b).not.toBeNull();
  return (
    a!.x < b!.x + b!.width &&
    a!.x + a!.width > b!.x &&
    a!.y < b!.y + b!.height &&
    a!.y + a!.height > b!.y
  );
}

test.describe("Invest journey", () => {
  // GIVEN the Invest journey on its drift view, WHEN the investor adds money,
  // keeps their amount while switching path, continues to the proposal, goes
  // back to the rules and updates the portfolio, THEN each step shows the
  // fixture figures, the amount survives going back, and an updated summary
  // lists the changed quantities.
  test("drift leads through plan and proposal to an updated portfolio", async ({
    page,
  }, testInfo) => {
    await openInvest(page, "Drift overview", {
      mobileVariant: testInfo.project.name === "storybook-mobile",
    });

    await expect(
      page.getByRole("heading", { level: 1, name: "Invest" })
    ).toBeVisible();
    await expect(
      visibleText(page, formatPoints(addMoneyProposal.driftBefore))
    ).toBeVisible();

    // The path actions sit in the page header, outside the comparison card.
    const paths = page
      .getByRole("group", { name: "Investment paths" })
      .locator("visible=true");
    await expect(
      paths.getByRole("button", { name: "Rebalance" })
    ).toBeVisible();
    await expect(page.locator(".ds-path-card")).toHaveCount(0);
    await paths.getByRole("button", { name: "Add money" }).click();
    const amount = page.getByLabel("Amount to invest");
    await expect(amount).toBeVisible();

    // The path switch lives inside the Budget and rules card.
    await expect(
      page
        .locator(".ds-plan-form__rules")
        .getByRole("radiogroup", { name: "Investment path" })
    ).toBeVisible();

    // Switching path keeps what was typed.
    await amount.fill("750");
    await page.getByRole("radio", { name: "Rebalance" }).click();
    await expect(page.getByLabel("New money to add (optional)")).toHaveValue(
      "750"
    );
    await page.getByRole("radio", { name: "Add money" }).click();
    await expect(amount).toHaveValue("750");

    const suggested = suggestedAmountStates.amount;
    if (suggested.status !== "amount") throw new Error("fixture changed");
    await page.getByRole("button", { name: "Use suggested" }).click();
    await expect(amount).toHaveValue(String(suggested.amount));

    await page.getByRole("button", { name: "Continue" }).click();
    await expect(
      page.getByRole("status", { name: "Calculating your proposal" })
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Update portfolio" })
    ).toBeVisible();
    for (const transaction of addMoneyProposal.transactions) {
      await expect(
        visibleText(page, formatCurrency(transaction.amount))
      ).toBeVisible();
    }
    await expect(page.getByText(/Funded by/)).toContainText(
      formatCurrency(addMoneyProposal.fundedBy?.newMoney ?? 0)
    );

    // Going back to the rules keeps the amount.
    await page.getByRole("button", { name: "Back to rules" }).click();
    await expect(amount).toHaveValue(String(suggested.amount));

    await page.getByRole("button", { name: "Continue" }).click();
    await page.getByRole("button", { name: "Update portfolio" }).click();
    await expect(
      page.getByRole("heading", { name: "Portfolio updated" })
    ).toBeVisible();
    for (const change of addMoneyResult.changes) {
      await expect(visibleText(page, change.ticker)).toBeVisible();
    }

    await page.getByRole("button", { name: "Back to portfolio" }).click();
    await expect(
      page.getByRole("heading", { name: "Current vs target" })
    ).toBeVisible();
  });

  // GIVEN the Rebalance proposal, WHEN it is reviewed, THEN overweight assets
  // are sold, the funding note names the sale proceeds and cash used, sale
  // commissions are said not to be estimated, and no Buy only option exists on
  // the Rebalance plan.
  test("rebalance sells overweight assets and offers no buy-only toggle", async ({
    page,
  }) => {
    await openInvest(page, "Plan rebalance");
    await expect(page.getByRole("checkbox", { name: /Buy only/i })).toHaveCount(
      0
    );
    await expect(page.getByLabel("New money to add (optional)")).toBeVisible();

    await openInvest(page, "Proposal rebalance");
    const sells = rebalanceProposal.transactions.filter(
      (transaction) => transaction.action === "sell"
    );
    expect(sells.length).toBeGreaterThan(0);
    await expect(visibleText(page, "Sell")).toBeVisible();
    for (const sell of sells) {
      await expect(visibleText(page, sell.ticker)).toBeVisible();
    }
    const fundedBy = rebalanceProposal.fundedBy;
    expect(fundedBy).not.toBeNull();
    const note = page.getByText(/Funded by/);
    await expect(note).toContainText(
      formatCurrency(fundedBy?.saleProceeds ?? 0)
    );
    await expect(note).toContainText(formatCurrency(fundedBy?.cashUsed ?? 0));
    await expect(note).toContainText("Sale commissions are not estimated.");
  });

  // GIVEN the plan step, WHEN the investor types negative or non-numeric
  // input, THEN only the digits are kept, and WHEN they accept the suggested
  // amount, THEN the field is filled with it.
  test("plan ignores invalid input and fills the suggested amount", async ({
    page,
  }) => {
    await openInvest(page, "Plan add money");
    const amount = page.getByLabel("Amount to invest");
    const suggested = suggestedAmountStates.amount;
    if (suggested.status !== "amount") throw new Error("fixture changed");

    await amount.fill("-5abc00");
    await expect(amount).toHaveValue("500");
    await amount.fill("abc");
    await expect(amount).toHaveValue("");

    await page.getByRole("button", { name: "Use suggested" }).click();
    await expect(amount).toHaveValue(String(suggested.amount));
    await expect(page.getByRole("button", { name: "Continue" })).toBeEnabled();
  });

  // GIVEN the plan step, WHEN the investor opens the commissions editor and
  // presses Escape, THEN it closes and focus returns to the Edit button.
  test("commissions editor closes with Escape and restores focus", async ({
    page,
  }) => {
    await openInvest(page, "Plan add money");
    const edit = page.getByRole("button", { name: "Edit commissions" });
    await edit.click();
    await expect(page.locator(".ds-fee-configuration:visible")).toHaveCount(1);
    await page.keyboard.press("Escape");
    await expect(page.locator(".ds-fee-configuration:visible")).toHaveCount(0);
    await expect(edit).toBeFocused();

    // The Canvas story opens the same editor immediately.
    await openInvest(page, "Commissions dialog");
    await expect(page.locator(".ds-fee-configuration:visible")).toHaveCount(1);
    await page.keyboard.press("Escape");
    await expect(page.locator(".ds-fee-configuration:visible")).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Edit commissions" })
    ).toBeFocused();
  });

  // GIVEN the drift view on a phone, WHEN it first loads, THEN the Add money and
  // Rebalance buttons are inside the first viewport without scrolling and the
  // Portfolio value is not broken across lines.
  test("mobile drift shows its path actions without scrolling", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name !== "storybook-mobile",
      "The first-viewport contract is mobile-only."
    );
    await openInvest(page, "Drift overview", { mobileVariant: true });
    const viewport = page.viewportSize();
    for (const name of ["Add money", "Rebalance"]) {
      const box = await page.getByRole("button", { name }).boundingBox();
      expect(box).not.toBeNull();
      expect(box!.y).toBeGreaterThanOrEqual(0);
      expect(box!.y + box!.height).toBeLessThanOrEqual(viewport!.height);
    }
    await expectNoHorizontalOverflow(page);
  });

  // GIVEN the drift metric strip, WHEN it renders at any width, THEN each
  // figure stays on one line.
  test("drift metric values never wrap mid-number", async ({
    page,
  }, testInfo) => {
    await openInvest(page, "Drift overview", {
      mobileVariant: testInfo.project.name === "storybook-mobile",
    });
    const values = page.locator(
      ".ds-metric-strip .ds-metric-card__value:visible"
    );
    expect(await values.count()).toBeGreaterThan(0);
    for (const value of await values.all()) {
      const lines = await value.evaluate((node) => {
        const lineHeight = parseFloat(getComputedStyle(node).lineHeight);
        return Math.round(node.getBoundingClientRect().height / lineHeight);
      });
      expect(lines).toBe(1);
    }
  });

  // GIVEN the drift, plan and proposal screens beside the portfolio sidebar,
  // WHEN the viewport is 1024 or 1180 pixels wide, THEN the page does not
  // scroll sideways, no metric value is clipped by its card, the plan's path
  // switch spans the card content, and the drift screen shows Add money
  // (primary) and Rebalance (secondary) outside any card, in the first viewport.
  for (const width of [1024, 1180]) {
    test(`invest screens fit the main column at ${width}px`, async ({
      page,
    }, testInfo) => {
      test.skip(
        testInfo.project.name === "storybook-mobile",
        "Mid-width desktop layouts are not exercised on the phone project."
      );
      await page.setViewportSize({ width, height: 800 });

      const screens = [
        "Drift overview",
        "Plan add money",
        "Proposal add money",
        "Proposal rebalance",
      ];
      for (const story of screens) {
        await openInvest(page, story);
        await expectNoHorizontalOverflow(page);
        const clipped = await page
          .locator(".ds-metric-card__value:visible")
          .evaluateAll((nodes) =>
            nodes
              .filter((node) => node.scrollWidth > node.clientWidth)
              .map((node) => node.textContent)
          );
        expect(clipped, `${story} at ${width}px clips metric values`).toEqual(
          []
        );
      }

      await openInvest(page, "Plan add money");
      const strategy = page.getByRole("radiogroup", {
        name: "Investment path",
      });
      const content = page
        .locator(".ds-plan-form__rules .ds-card__content")
        .first();
      const switchBox = await strategy.boundingBox();
      const contentBox = await content.boundingBox();
      expect(switchBox).not.toBeNull();
      expect(contentBox).not.toBeNull();
      expect(switchBox!.width).toBeGreaterThanOrEqual(contentBox!.width - 48);

      await openInvest(page, "Drift overview");
      const addMoney = page.getByRole("button", { name: "Add money" });
      const rebalance = page.getByRole("button", { name: "Rebalance" });
      await expect(addMoney).toHaveClass(/ds-button--primary/);
      await expect(rebalance).toHaveClass(/ds-button--secondary/);
      for (const button of [addMoney, rebalance]) {
        expect(await button.evaluate((node) => node.closest(".ds-card"))).toBe(
          null
        );
        const box = await button.boundingBox();
        expect(box).not.toBeNull();
        expect(box!.y + box!.height).toBeLessThanOrEqual(800);
      }
    });
  }

  // GIVEN each Invest step on a phone, WHEN it is shown, THEN nothing scrolls
  // sideways, the pinned action bar does not overlap the navigation button,
  // and the primary action is reachable above both.
  test("mobile steps keep actions reachable without overlap", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name !== "storybook-mobile",
      "The pinned action bar is a mobile-only contract."
    );
    const steps = [
      { story: "Drift overview", primary: "Add money" },
      { story: "Plan add money", primary: "Continue" },
      { story: "Proposal add money", primary: "Update portfolio" },
      { story: "Proposal rebalance", primary: "Update portfolio" },
      { story: "Updated", primary: "Back to portfolio" },
    ];
    for (const step of steps) {
      await openInvest(page, step.story, { mobileVariant: true });
      await expectNoHorizontalOverflow(page);

      const primary = page.getByRole("button", { name: step.primary });
      await primary.scrollIntoViewIfNeeded();
      await expect(primary).toBeVisible();
      const box = await primary.boundingBox();
      const viewport = page.viewportSize();
      expect(box).not.toBeNull();
      expect(box!.y + box!.height).toBeLessThanOrEqual(viewport!.height);

      const bar = page.locator(".ds-sticky-action-bar:visible");
      if ((await bar.count()) > 0) {
        const position = await bar.evaluate(
          (node) => window.getComputedStyle(node).position
        );
        expect(position).toBe("fixed");
        const fab = page.locator(".ds-portfolio-workspace__navigation-fab");
        expect(await overlaps(bar, fab)).toBe(false);
        expect(await overlaps(primary, fab)).toBe(false);
        // Nothing sits on top of the primary action.
        const covered = await primary.evaluate((node) => {
          const rect = node.getBoundingClientRect();
          const top = document.elementFromPoint(
            rect.x + rect.width / 2,
            rect.y + rect.height / 2
          );
          return !node.contains(top);
        });
        expect(covered).toBe(false);
      }
    }
  });

  // GIVEN the plan step on a desktop screen, WHEN it is shown, THEN the
  // actions sit in the page flow instead of being pinned to the screen edge.
  test("desktop keeps step actions in the page flow", async ({
    page,
  }, testInfo) => {
    test.skip(
      testInfo.project.name === "storybook-mobile",
      "Pinning to the screen edge is the mobile behaviour."
    );
    await openInvest(page, "Plan add money");
    const bar = page.locator(".ds-sticky-action-bar:visible");
    await expect(bar).toHaveCount(1);
    const position = await bar.evaluate(
      (node) => window.getComputedStyle(node).position
    );
    expect(position).not.toBe("fixed");
  });

  // GIVEN each edge state of the proposal, WHEN it is shown, THEN it explains
  // why there is nothing to approve and offers a way forward.
  for (const edge of [
    { story: "Proposal blocked", title: "Target weights must total 100%" },
    { story: "Proposal needs amount", title: "Nothing to invest yet" },
    {
      story: "Proposal on target",
      title: "You already reached your target allocation",
    },
    { story: "Proposal fees too high", title: "Transaction fees are too high" },
    {
      story: "Proposal price too high",
      title: "Your amount is too small for whole units",
    },
    {
      story: "Proposal error",
      title: "We could not calculate your transactions",
    },
  ]) {
    test(`${edge.story.toLowerCase()} explains the state`, async ({ page }) => {
      await openInvest(page, edge.story);
      await expect(visibleText(page, edge.title)).toBeVisible();
      await expect(
        page.getByRole("button", { name: "Update portfolio" })
      ).toHaveCount(0);
      await expect(
        page.locator(".ds-sticky-action-bar:visible").getByRole("button")
      ).not.toHaveCount(0);
    });
  }
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
      "Compositions / Invest",
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
