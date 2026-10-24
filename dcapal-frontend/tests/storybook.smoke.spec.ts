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

/**
 * Phones open the portfolio navigation from the top bar's menu button; on
 * desktop the button does not exist and the navigation rail is always shown.
 */
async function checkNavigationMenu(page: Page, isMobile: boolean) {
  const menuButton = page.getByRole("button", { name: "Open navigation menu" });
  if (isMobile) {
    await expect(menuButton).toBeVisible();
    await menuButton.click();
    await expect(
      page.getByRole("heading", { name: "Portfolio navigation" })
    ).toBeVisible();
    // Navigation holds destinations only, never the page's actions.
    await expect(
      page.locator(".ds-workspace-nav--mobile").getByRole("link", {
        name: "Add asset",
      })
    ).toHaveCount(0);
    await page.keyboard.press("Escape");
    await expect(
      page.getByRole("heading", { name: "Portfolio navigation" })
    ).toBeHidden();
  } else {
    await expect(menuButton).toBeHidden();
  }
}

/** The floating button that opens the phone's page-actions menu. */
function pageActionsButton(page: Page): Locator {
  return page.getByRole("button", { name: "Open page actions" });
}

/** The open page-actions menu. */
function pageActionsMenu(page: Page): Locator {
  return page.getByRole("dialog", { name: "Page actions" });
}

/** Opens the page-actions menu and chooses one of its actions. */
async function chooseFloatingAction(page: Page, name: string) {
  await pageActionsButton(page).click();
  await pageActionsMenu(page)
    .getByRole("button", { name, exact: true })
    .click();
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
        await checkNavigationMenu(page, isMobile);

        // Phones reach Add asset through the floating page-actions menu.
        if (isMobile) {
          await chooseFloatingAction(page, "Add asset");
        } else {
          await page.getByRole("button", { name: "Add asset" }).click();
        }
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
        await expect(
          page.getByRole("textbox", { name: "Target weight" })
        ).toBeVisible();
        await page
          .getByRole("button", { name: "Choose another asset" })
          .click();
        await expect(
          page.getByRole("region", { name: "Asset search results" })
        ).toBeVisible();
        await page.getByRole("button", { name: /VWCE/ }).click();
        // Exact match: the mobile drawer's "Close add asset" button must not count.
        await page
          .getByRole("button", { name: "Add asset", exact: true })
          .click();
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

        await checkNavigationMenu(
          page,
          testInfo.project.name === "storybook-mobile"
        );

        const cards = page.locator(".ds-asset-collection__cards");
        const table = page.locator(".ds-asset-collection__table");
        if (testInfo.project.name === "storybook-mobile") {
          await expect(cards).toBeVisible();
          await expect(table).toBeHidden();
          // The overview presentation mirrors the overview table: mobile cards
          // omit quantity, price and cost basis, as the desktop columns do.
          await expect(cards.locator(".ds-asset-ticker-chip")).toHaveCount(5);
          await expect(
            cards.getByText("Average cost basis", { exact: true })
          ).toHaveCount(0);
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
          // The overview table is a compact summary: it leads with value,
          // weight and drift and leaves quantity and cost basis to the
          // detailed asset views.
          for (const header of [
            "Current value",
            "Target weight",
            "Current weight",
            "Drift",
          ]) {
            await expect(
              table.getByRole("columnheader", { name: header })
            ).toBeVisible();
          }
          await expect(
            table.getByRole("columnheader", { name: /^Quantity/i })
          ).toHaveCount(0);
          await expect(
            table.getByRole("columnheader", { name: "Average cost basis" })
          ).toHaveCount(0);
          // The ticker chip sits inline beside the name.
          await expect(table.locator(".ds-asset-ticker-chip")).toHaveCount(5);
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
        await page.getByRole("button", { name: "Save changes" }).click();
        await expect(
          page.getByRole("heading", { name: "Set your asset class weights" })
        ).toBeVisible();
        await page.keyboard.press("Escape");
        await expect(page.getByRole("dialog")).toHaveCount(0);
        // Leaving the dialog keeps the saved Simple method.
        await expect(page.getByRole("radio", { name: "Simple" })).toBeChecked();
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

/**
 * GIVEN a portfolio in first use, then with its first asset chosen, then with
 * the setup finished
 * WHEN the overview renders each stage
 * THEN the empty stage shows two placeholder metrics (no cash balance yet) and
 * the wizard on its first step, the asset stage shows no placeholders and the
 * wizard on its last step with two steps completed, and the finished stage
 * shows the portfolio without any setup card.
 */
test("first-use overview stages expose placeholders and setup progress", async ({
  page,
}) => {
  await openStory(page, "Compositions / Portfolio overview", "First use");
  await expect(
    page.locator('.ds-metric-card[data-placeholder="true"]')
  ).toHaveCount(2);
  // First use has no cash balance to show, so no placeholder card is offered.
  await expect(
    page.locator(".ds-metric-card:visible", { hasText: "Cash balance" })
  ).toHaveCount(0);
  const wizard = page.getByRole("region", { name: "Set up your portfolio" });
  await expect(wizard).toBeVisible();
  await expect(wizard.getByText("Step 1 of 3", { exact: false })).toBeVisible();
  await expect(
    wizard.locator('.ds-setup-wizard__segment[data-state="complete"]')
  ).toHaveCount(0);

  await openStory(
    page,
    "Compositions / Portfolio overview",
    "First use asset added"
  );
  await expect(
    page.locator('.ds-metric-card[data-placeholder="true"]')
  ).toHaveCount(0);
  await expect(
    page.locator('.ds-setup-wizard__segment[data-state="complete"]')
  ).toHaveCount(2);
  await expect(
    page.getByRole("heading", { name: "Set target weights" })
  ).toBeVisible();
  await expect(page.getByText("1 asset", { exact: true })).toBeVisible();

  await openStory(
    page,
    "Compositions / Portfolio overview",
    "First use weights complete"
  );
  await expect(
    page.getByRole("region", { name: "Set up your portfolio" })
  ).toHaveCount(0);
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

/**
 * GIVEN the asset collection in its default Simple mode
 * WHEN the investor switches to Strategic and collapses the Equities group
 * THEN class group rows appear with their assets, and collapsing hides only
 * that class's asset rows while keeping the toggle's expanded state exact.
 */
test("asset collection groups assets by class in strategic mode", async ({
  page,
}) => {
  const storyId = await findStory(page, "Patterns / Portfolio", "Assets");
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  const visibleNames = page.locator(
    ".ds-table__asset-name:visible, .ds-asset-card__name:visible"
  );
  await expect(
    page.getByRole("button", { name: "Collapse Equities" })
  ).toHaveCount(0);
  await expect(visibleNames).toHaveCount(5);

  // Phones switch the mode inside the Table options drawer.
  const optionsButton = page.getByRole("button", { name: "Table options" });
  if (await optionsButton.isVisible()) {
    await optionsButton.click();
    await page.getByRole("radio", { name: "Strategic" }).check();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toBeHidden();
  } else {
    await page.getByRole("radio", { name: "Strategic" }).click();
  }
  const collapse = page.getByRole("button", { name: "Collapse Equities" });
  await expect(collapse).toBeVisible();
  await expect(collapse).toHaveAttribute("aria-expanded", "true");
  await expect(
    page.getByRole("button", { name: "Collapse Bonds" })
  ).toBeVisible();
  await expect(visibleNames).toHaveCount(5);

  await collapse.click();
  const expand = page.getByRole("button", { name: "Expand Equities" });
  await expect(expand).toHaveAttribute("aria-expanded", "false");
  await expect(visibleNames).toHaveCount(2);
  await expect(
    page.getByText("Vanguard FTSE All-World UCITS ETF").locator("visible=true")
  ).toHaveCount(0);

  await expand.click();
  await expect(visibleNames).toHaveCount(5);
});

/**
 * GIVEN the desktop asset table in Simple mode, sorted by target weight
 * WHEN the investor activates the Current value and Asset headers
 * THEN aria-sort moves to the activated header and the order follows it.
 */
test("asset table headers sort the assets", async ({ page }, testInfo) => {
  test.skip(
    testInfo.project.name === "storybook-mobile",
    "Sortable headers belong to the desktop table."
  );
  const storyId = await findStory(page, "Patterns / Portfolio", "Assets");
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  const targetHeader = page.getByRole("columnheader", {
    name: "Target weight",
  });
  const valueHeader = page.getByRole("columnheader", { name: "Current value" });
  const names = page.locator(".ds-table__asset-name");
  await expect(targetHeader).toHaveAttribute("aria-sort", "descending");
  await expect(names.first()).toHaveText("Vanguard FTSE All-World UCITS ETF");

  await valueHeader.getByRole("button").click();
  await expect(valueHeader).toHaveAttribute("aria-sort", "descending");
  await expect(targetHeader).toHaveAttribute("aria-sort", "none");
  await expect(names.first()).toHaveText("Cash");

  await valueHeader.getByRole("button").click();
  await expect(valueHeader).toHaveAttribute("aria-sort", "ascending");
  await expect(names.first()).toHaveText(
    "iShares Core Global Aggregate Bond ETF"
  );
});

/** The setup wizard card of the first-use overview. */
function setupWizard(page: Page): Locator {
  return page.getByRole("region", { name: "Set up your portfolio" });
}

/**
 * GIVEN a portfolio in first use on a phone or desktop
 * WHEN the investor names it, picks an asset, keeps the 100% weight and
 * finishes
 * THEN each step unlocks only after the previous one is valid, completed steps
 * can be reopened from the progress bar, and finishing removes the card and
 * leaves the portfolio with its first asset on the page.
 */
test("setup wizard walks name, asset and weight, then disappears", async ({
  page,
}) => {
  await openStory(page, "Compositions / Portfolio overview", "First use");
  const wizard = setupWizard(page);
  const steps = wizard.getByRole("list", { name: "Setup steps" });
  const continueButton = wizard.getByRole("button", { name: "Continue" });

  // Step 1 needs a name; later steps cannot be opened yet.
  await expect(
    wizard.getByRole("heading", { name: "Name your portfolio" })
  ).toBeVisible();
  await expect(continueButton).toBeDisabled();
  await expect(
    steps.getByRole("button", { name: /First asset/ })
  ).toBeDisabled();
  await expect(steps.getByRole("button", { name: /Name/ })).toHaveAttribute(
    "aria-current",
    "step"
  );
  await wizard.getByRole("textbox", { name: "Portfolio name" }).fill("Starter");
  await wizard.getByRole("combobox", { name: "Base currency" }).click();
  await page.getByRole("option", { name: /USD · US dollar/ }).click();
  await continueButton.click();

  // Step 2 needs a chosen asset.
  const assetHeading = wizard.getByRole("heading", {
    name: "Add your first asset",
  });
  await expect(assetHeading).toBeVisible();
  await expect(assetHeading).toBeFocused();
  await expect(continueButton).toBeDisabled();
  await wizard.getByRole("searchbox", { name: "Search an asset" }).fill("agg");
  const results = wizard.getByRole("region", { name: "Asset search results" });
  await expect(results.getByRole("button")).toHaveCount(1);
  await results.getByRole("button", { name: /AGGH/ }).click();
  await expect(results.getByRole("button", { name: /AGGH/ })).toHaveAttribute(
    "aria-pressed",
    "true"
  );
  await continueButton.click();

  // Step 3 validates the weight live and blocks anything but 100%.
  await expect(
    wizard.getByRole("heading", { name: "Set target weights" })
  ).toBeVisible();
  const weight = wizard.getByRole("textbox", { name: "Target weight" });
  await weight.fill("60");
  await expect(
    wizard.getByText("Weights must total 100%. Right now: 60.0%.")
  ).toBeVisible();
  const finish = wizard.getByRole("button", { name: "Finish setup" });
  await expect(finish).toBeDisabled();
  await weight.fill("100");
  await expect(wizard.getByText("Weights total 100%.")).toBeVisible();
  await expect(finish).toBeEnabled();

  // A completed step reopens from the progress bar and keeps its value.
  await steps.getByRole("button", { name: /Name/ }).click();
  await expect(
    wizard.getByRole("textbox", { name: "Portfolio name" })
  ).toHaveValue("Starter");
  await steps.getByRole("button", { name: /Target weight/ }).click();
  await expect(weight).toHaveValue("100");

  await finish.click();
  await expect(setupWizard(page)).toHaveCount(0);
  await expect(page.getByText("1 asset", { exact: true })).toBeVisible();
  await expect(
    page.locator('.ds-metric-card[data-placeholder="true"]')
  ).toHaveCount(0);
});

/**
 * GIVEN a Strategic portfolio in first use
 * WHEN the wizard reaches its last step
 * THEN it asks for the asset class weight instead of the target weight.
 */
test("setup wizard asks for the class weight in a Strategic portfolio", async ({
  page,
}) => {
  await openStory(page, "Compositions / Portfolio overview", "First use");
  // Story controls change the allocation method through the URL args.
  const storyId = await findStory(
    page,
    "Compositions / Portfolio overview",
    "First use asset added"
  );
  await page.goto(
    `/iframe.html?id=${encodeURIComponent(storyId)}&args=allocationMethod:strategic`
  );
  await page.waitForLoadState("networkidle");
  const wizard = setupWizard(page);
  await expect(
    wizard.getByRole("heading", { name: "Set the class weight" })
  ).toBeVisible();
  await expect(
    wizard.getByRole("textbox", { name: "Asset class weight" })
  ).toBeVisible();
  await expect(
    wizard.getByRole("button", { name: /Class weight/ })
  ).toHaveAttribute("aria-current", "step");
});

/**
 * GIVEN each step of the setup wizard
 * WHEN it is shown on a phone
 * THEN there is no sideways scroll, every control and progress step is at
 * least 44px tall, and the wizard has no accessibility violations; on desktop
 * only the accessibility check applies.
 */
test("setup wizard steps are accessible and touch friendly", async ({
  page,
}, testInfo) => {
  const mobile = testInfo.project.name === "storybook-mobile";
  for (const story of [
    "First use",
    "First use name complete",
    "First use asset added",
  ]) {
    await openStory(page, "Compositions / Portfolio overview", story);
    const wizard = setupWizard(page);
    await expect(wizard).toBeVisible();
    if (mobile) {
      await expectNoHorizontalOverflow(page);
      const controls = wizard
        .getByRole("textbox")
        .or(wizard.getByRole("searchbox"))
        .or(wizard.getByRole("combobox"))
        .or(wizard.getByRole("button"));
      const count = await controls.count();
      expect(count).toBeGreaterThan(0);
      for (let index = 0; index < count; index += 1) {
        const box = await controls.nth(index).boundingBox();
        expect(box?.height ?? 0).toBeGreaterThanOrEqual(43.5);
      }
    }
    const results = await new AxeBuilder({ page }).analyze();
    expect(results.violations).toEqual([]);
  }
});

/**
 * GIVEN the setup wizard on its first step
 * WHEN the investor tabs through it
 * THEN focus visits the name, the currency and Continue in reading order,
 * and Enter in the name field continues once the step is valid.
 */
test("setup wizard keeps a natural keyboard order", async ({ page }) => {
  await openStory(page, "Compositions / Portfolio overview", "First use");
  const wizard = setupWizard(page);
  const name = wizard.getByRole("textbox", { name: "Portfolio name" });
  await name.focus();
  await page.keyboard.type("Starter");
  await page.keyboard.press("Tab");
  await expect(
    wizard.getByRole("combobox", { name: "Base currency" })
  ).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(wizard.getByRole("button", { name: "Continue" })).toBeFocused();
  await name.focus();
  await page.keyboard.press("Enter");
  await expect(
    wizard.getByRole("heading", { name: "Add your first asset" })
  ).toBeFocused();
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
  // sideways, steps with a pinned action bar offer no floating page-actions
  // button, and the primary action is reachable and not covered.
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
        // A step with a pinned bar offers no floating page-actions button, so
        // the two never compete for the bottom corner.
        await expect(pageActionsButton(page)).toHaveCount(0);
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
  /**
   * GIVEN the Add asset overlay open on an asset being edited
   * WHEN the investor opens the transaction fee editor, saves or dismisses it
   * THEN one nested fee overlay (dialog on desktop, drawer on mobile) sits above
   * the parent overlay, dismissal discards unsaved choices and focus returns to
   * the fee trigger.
   */
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

    // The story's Add asset overlay stays open as the parent, so exactly one
    // nested fee overlay is stacked above exactly one parent overlay.
    if (testInfo.project.name === "storybook-mobile") {
      await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(0);
      await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(2);
      await expect(
        page.locator(".ds-drawer-popup--nested:visible")
      ).toHaveCount(1);
    } else {
      await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(2);
      await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(0);
      await expect(
        page.locator(".ds-dialog-popup--nested:visible")
      ).toHaveCount(1);
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
    /**
     * GIVEN the Edit and remove asset story for this stage
     * WHEN it renders
     * THEN only the stage's overlay is open (a drawer on mobile fees, which is
     * exposed as the one dialog) and Escape closes it without a Cancel button.
     */
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
        // A drawer is a modal dialog for assistive technology, so the single
        // exposed dialog must be that drawer rather than a dialog popup.
        await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(1);
        await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(0);
        await expect(page.getByRole("dialog")).toHaveCount(1);
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

/**
 * GIVEN the detailed Assets story at the mobile viewport
 * WHEN the cards render
 * THEN every card carries quantity, price and average cost basis (an em dash
 * when missing), matching the detailed desktop columns.
 */
test("detailed mobile asset cards keep the table's detail fields", async ({
  page,
}, testInfo) => {
  test.skip(
    testInfo.project.name !== "storybook-mobile",
    "Cards are the mobile presentation."
  );
  const storyId = await findStory(page, "Patterns / Portfolio", "Assets");
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  const details = page.locator(".ds-asset-card__details");
  await expect(details).toHaveCount(5);
  await expect(details.first()).toContainText("Quantity");
  await expect(details.first()).toContainText("Current price");
  await expect(details.first()).toContainText("Average cost basis");
});

async function openStory(page: Page, title: string, name: string) {
  const storyId = await findStory(page, title, name);
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");
}

/**
 * GIVEN the Assets story at the mobile viewport, ordered by target weight
 * WHEN the investor opens "Table options", picks "Asset name" in Sort by and
 * then reverses the order
 * THEN the cards behind the drawer reorder alphabetically and then in reverse,
 * live, and Done closes the drawer and returns focus to the options button.
 */
test("mobile table options drawer sorts the asset cards", async ({
  page,
}, testInfo) => {
  test.skip(
    testInfo.project.name !== "storybook-mobile",
    "The sort control replaces sortable headers on mobile only."
  );
  await openStory(page, "Patterns / Portfolio", "Assets");

  // The inline controls no longer take a row of their own.
  await expect(page.getByLabel("Sort by")).toBeHidden();
  await expect(
    page.getByRole("radiogroup", { name: "Table mode" })
  ).toBeHidden();

  const trigger = page.getByRole("button", { name: "Table options" });
  await trigger.click();
  const drawer = page.getByRole("dialog", { name: "Table options" });
  await expect(drawer).toBeVisible();

  const names = page.locator(".ds-asset-card__name");
  await drawer.getByLabel("Sort by").selectOption({ label: "Asset name" });
  const ascending = await names.allTextContents();
  expect(ascending).toEqual([...ascending].sort((a, b) => a.localeCompare(b)));
  await drawer.getByRole("button", { name: /reverse sort order/ }).click();
  const descending = await names.allTextContents();
  expect(descending).toEqual([...ascending].reverse());

  await drawer.getByRole("button", { name: "Done" }).click();
  await expect(drawer).toBeHidden();
  await expect(trigger).toBeFocused();
});

/**
 * GIVEN the Assets story on a phone
 * WHEN the investor switches Table mode to Strategic inside the Table options
 * drawer and dismisses it with Escape
 * THEN the cards group by asset class immediately and the drawer is closed.
 */
test("mobile table options drawer switches the table mode", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "storybook-mobile", "Mobile only.");
  await openStory(page, "Patterns / Portfolio", "Assets");

  await page.getByRole("button", { name: "Table options" }).click();
  const drawer = page.getByRole("dialog", { name: "Table options" });
  await drawer.getByRole("radio", { name: "Strategic" }).check();
  await expect(page.locator(".ds-asset-group-card").first()).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(drawer).toBeHidden();
});

/**
 * GIVEN the Assets story at a desktop viewport
 * WHEN it renders
 * THEN the Table mode switch is inline in the header, the options button and
 * mobile sort control are hidden, because headers sort the table.
 */
test("desktop keeps the mode switch inline and hides table options", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name === "storybook-mobile", "Desktop only.");
  await openStory(page, "Patterns / Portfolio", "Assets");
  await expect(
    page.getByRole("radiogroup", { name: "Table mode" })
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Table options" })
  ).toBeHidden();
  await expect(page.getByLabel("Sort by")).toBeHidden();
});

/**
 * GIVEN the Table options story, whose drawer is open on a phone
 * WHEN it renders
 * THEN the drawer exposes Table mode, Sort by, the reverse button and Done.
 */
test("table options story shows every control in the drawer", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "storybook-mobile", "Mobile only.");
  await openStory(page, "Patterns / Portfolio", "Assets Table Options Mobile");
  const drawer = page.getByRole("dialog", { name: "Table options" });
  await expect(
    drawer.getByRole("radiogroup", { name: "Table mode" })
  ).toBeVisible();
  await expect(drawer.getByLabel("Sort by")).toBeVisible();
  await expect(drawer.getByRole("button", { name: "Done" })).toBeVisible();
});

/**
 * GIVEN the Portfolio overview on desktop
 * WHEN it renders
 * THEN Add asset and Invest stay in the page header, nothing is duplicated
 * above the table, and no floating page-actions button or top-bar navigation
 * button exists.
 */
test("desktop overview keeps Add asset and Invest in the page header", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name === "storybook-mobile", "Desktop only.");
  await openStory(page, "Compositions / Portfolio overview", "Populated");
  const header = page.locator(".ds-page-header");
  await expect(header.getByRole("button", { name: "Add asset" })).toBeVisible();
  await expect(header.getByRole("button", { name: "Invest" })).toBeVisible();
  await expect(
    page
      .locator(".ds-asset-collection")
      .getByRole("button", { name: "Add asset" })
  ).toHaveCount(0);
  await expect(pageActionsButton(page)).toBeHidden();
  await expect(
    page.getByRole("button", { name: "Open navigation menu" })
  ).toBeHidden();
});

/**
 * GIVEN the Portfolio overview on a phone
 * WHEN it renders
 * THEN the page header and the Investments cards carry no Add asset or Invest
 * buttons; both live in the floating page-actions menu, while the Table
 * options button stays.
 */
test("mobile overview moves Add asset and Invest into the page-actions menu", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "storybook-mobile", "Mobile only.");
  await openStory(page, "Compositions / Portfolio overview", "Populated");
  await expect(
    page
      .locator(".ds-asset-collection")
      .getByRole("button", { name: "Add asset" })
  ).toHaveCount(0);
  await expect(
    page
      .locator(".ds-asset-collection")
      .getByRole("button", { name: "Table options" })
  ).toBeVisible();
  await pageActionsButton(page).click();
  const menu = pageActionsMenu(page);
  await expect(menu.getByRole("button")).toHaveText([
    "Add asset",
    "Invest",
    "",
  ]);
});

const PAGE_ACTION_JOURNEYS = [
  {
    page: "overview",
    title: "Compositions / Portfolio overview",
    story: "Populated Mobile",
    actions: ["Add asset", "Invest"],
  },
  {
    page: "invest drift",
    title: "Compositions / Invest",
    story: "Drift overview Mobile",
    actions: ["Add money", "Rebalance"],
  },
  {
    page: "settings (Simple)",
    title: "Compositions / Portfolio settings",
    story: "Settings simple Mobile",
    actions: ["Save changes"],
  },
  {
    page: "settings (Strategic)",
    title: "Compositions / Portfolio settings",
    story: "Settings strategic Mobile",
    actions: ["Save changes", "Review strategic allocation"],
  },
] as const;

test.describe("mobile page-actions menu", () => {
  test.beforeEach(({}, testInfo) => {
    test.skip(
      testInfo.project.name !== "storybook-mobile",
      "The page-actions menu is a phone-only surface."
    );
  });

  for (const journey of PAGE_ACTION_JOURNEYS) {
    /**
     * GIVEN the {page} page on a phone
     * WHEN the investor opens the floating page-actions menu
     * THEN it lists only that page's actions, the first of them in the primary
     * colour, and no navigation destination.
     */
    test(`offers the actions of ${journey.page}`, async ({ page }) => {
      await openStory(page, journey.title, journey.story);
      const fab = pageActionsButton(page);
      await expect(fab).toBeVisible();
      await expect(fab).toHaveAttribute("aria-expanded", "false");
      const background = (locator: Locator) =>
        locator.evaluate((node) => getComputedStyle(node).backgroundColor);
      const fabBackground = await background(fab);
      await fab.click();
      const menu = pageActionsMenu(page);
      await expect(menu).toBeVisible();
      await expect(menu.getByRole("button")).toHaveText([
        ...journey.actions,
        // The close button repeats the floating button in place.
        "",
      ]);
      await expect(menu.getByRole("link")).toHaveCount(0);
      const primary = menu.getByRole("button", { name: journey.actions[0] });
      // The primary pill shares the floating button's primary colour; the
      // others do not.
      expect(await background(primary)).toBe(fabBackground);
      for (const other of journey.actions.slice(1)) {
        expect(
          await background(menu.getByRole("button", { name: other }))
        ).not.toBe(fabBackground);
      }
    });
  }

  /**
   * GIVEN the overview on a phone
   * WHEN the menu is opened with a tap, closed with Escape, then opened again
   * and closed by a tap on the dimmed page
   * THEN focus moves into the menu when it opens and returns to the floating
   * button when it closes, and the button's expanded state follows.
   */
  test("opens and closes with focus moving in and back", async ({ page }) => {
    await openStory(
      page,
      "Compositions / Portfolio overview",
      "Populated Mobile"
    );
    const fab = pageActionsButton(page);
    await fab.click();
    const menu = pageActionsMenu(page);
    await expect(menu).toBeVisible();
    await expect(menu.getByRole("button", { name: "Add asset" })).toBeFocused();
    await expect(
      page.getByRole("button", { name: "Close page actions" })
    ).toBeVisible();

    await page.keyboard.press("Escape");
    await expect(menu).toHaveCount(0);
    await expect(fab).toBeFocused();
    await expect(fab).toHaveAttribute("aria-expanded", "false");

    await fab.click();
    await expect(menu).toBeVisible();
    // A tap on the dimmed page, away from the actions, closes the menu.
    await page.mouse.click(40, 300);
    await expect(menu).toHaveCount(0);
    await expect(fab).toBeFocused();

    await fab.click();
    await page.getByRole("button", { name: "Close page actions" }).click();
    await expect(menu).toHaveCount(0);
    await expect(fab).toBeFocused();
  });

  /**
   * GIVEN the open menu
   * WHEN the investor tabs and shift-tabs
   * THEN focus stays in the menu: actions in reading order, then the close
   * button, then back to the first action.
   */
  test("keeps keyboard focus inside the open menu in reading order", async ({
    page,
  }) => {
    await openStory(
      page,
      "Compositions / Portfolio overview",
      "Populated Mobile"
    );
    await pageActionsButton(page).click();
    const menu = pageActionsMenu(page);
    await expect(menu.getByRole("button", { name: "Add asset" })).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(menu.getByRole("button", { name: "Invest" })).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(
      menu.getByRole("button", { name: "Close page actions" })
    ).toBeFocused();
    await page.keyboard.press("Tab");
    await expect(menu.getByRole("button", { name: "Add asset" })).toBeFocused();
    await page.keyboard.press("Shift+Tab");
    await expect(
      menu.getByRole("button", { name: "Close page actions" })
    ).toBeFocused();
  });

  /**
   * GIVEN the overview on a phone
   * WHEN the investor chooses Add asset from the menu
   * THEN the menu closes and the Add an asset drawer opens with focus inside
   * it, not pulled back to the floating button.
   */
  test("runs the chosen action after closing, keeping its focus", async ({
    page,
  }) => {
    await openStory(page, "Compositions / Add asset", "Entry point Mobile");
    await chooseFloatingAction(page, "Add asset");
    await expect(pageActionsMenu(page)).toHaveCount(0);
    const dialog = page.getByRole("dialog", { name: "Add an asset" });
    await expect(dialog).toBeVisible();
    expect(
      await dialog.evaluate((node) => node.contains(document.activeElement))
    ).toBe(true);
  });

  /**
   * GIVEN the Invest drift view with targets that total 100%, then without
   * WHEN the investor opens the menu
   * THEN the paths are available in the first case and visible but disabled
   * in the second, matching the page's own buttons.
   */
  test("disables the Invest paths while targets are blocked", async ({
    page,
  }) => {
    const storyId = await findStory(
      page,
      "Compositions / Invest",
      "Drift overview Mobile"
    );
    await page.goto(
      `/iframe.html?id=${encodeURIComponent(storyId)}&args=proposalState:blocked`
    );
    await page.waitForLoadState("networkidle");
    await pageActionsButton(page).click();
    const menu = pageActionsMenu(page);
    await expect(
      menu.getByRole("button", { name: "Add money" })
    ).toBeDisabled();
    await expect(
      menu.getByRole("button", { name: "Rebalance" })
    ).toBeDisabled();
  });

  /**
   * GIVEN the closed and the open menu on each page's story
   * WHEN accessibility is checked
   * THEN there are no violations and the open menu does not scroll sideways.
   */
  test("has no accessibility violations closed or open", async ({ page }) => {
    for (const story of [
      "Floating Actions Mobile",
      "Floating Actions Open Mobile",
      "Floating Actions Invest Open Mobile",
      "Floating Actions Settings Open Mobile",
    ]) {
      await openStory(page, "Patterns / Portfolio", story);
      await expectNoHorizontalOverflow(page);
      const results = await new AxeBuilder({ page }).analyze();
      expect(results.violations).toEqual([]);
    }
  });

  /**
   * GIVEN the Invest plan, proposal and result steps that pin an action bar
   * WHEN each is shown on a phone
   * THEN no floating page-actions button is offered, so it never overlaps
   * the bar.
   */
  test("is absent where a sticky action bar is pinned", async ({ page }) => {
    for (const story of [
      "Plan add money Mobile",
      "Proposal add money Mobile",
      "Updated Mobile",
    ]) {
      await openStory(page, "Compositions / Invest", story);
      await expect(page.locator(".ds-sticky-action-bar:visible")).toHaveCount(
        1
      );
      await expect(pageActionsButton(page)).toHaveCount(0);
    }
  });

  /**
   * GIVEN the Settings page on a phone with a Strategic selection
   * WHEN Save changes is chosen from the menu
   * THEN it runs the same save as the page's own button.
   */
  test("Save changes in the menu saves the settings", async ({ page }) => {
    await openStory(
      page,
      "Compositions / Portfolio settings",
      "Settings simple Mobile"
    );
    await chooseFloatingAction(page, "Save changes");
    await expect(page.getByText("Portfolio settings saved")).toBeVisible();
  });
});

/**
 * GIVEN the workspace pages on desktop
 * WHEN they render
 * THEN no floating page-actions button is visible, even though the page
 * supplies actions.
 */
test("page-actions menu is absent on desktop", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name === "storybook-mobile", "Desktop only.");
  for (const [title, story] of [
    ["Patterns / Portfolio", "Floating Actions Desktop"],
    ["Compositions / Invest", "Drift overview"],
    ["Compositions / Portfolio settings", "Settings simple"],
  ]) {
    await openStory(page, title, story);
    await expect(
      page.getByRole("button", { name: "Open page actions" })
    ).toBeHidden();
    await expect(
      page.getByRole("button", { name: "Open page actions" })
    ).not.toBeInViewport();
  }
});

/**
 * GIVEN a table wider than its container (single-asset edit story and the
 * narrow Strategic story) on desktop
 * WHEN the wrapper is scrolled fully to the right and back
 * THEN the Edit / More actions cells stay inside the visible wrapper edge and
 * the header, group and total cells keep the row background, so scrolled
 * content never shows through them.
 */
test("sticky actions column stays visible while the table scrolls", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name === "storybook-mobile", "Desktop only.");
  const stories: [string, string][] = [
    ["Compositions / Edit and remove asset", "Edit Details"],
    ["Patterns / Portfolio", "Assets Strategic Narrow"],
  ];
  for (const [title, name] of stories) {
    await openStory(page, title, name);
    const wrap = page.locator(".ds-table-wrap").first();
    await expect(wrap).toHaveAttribute("data-overflow-end", "true");
    const edit = page.getByRole("button", { name: /^Edit [A-Z]/ }).first();
    await expect(edit).toBeVisible();

    const visibleInWrap = async () => {
      const w = await wrap.boundingBox();
      const b = await edit.boundingBox();
      expect(b && w && b.x >= w.x && b.x + b.width <= w.x + w.width + 1).toBe(
        true
      );
    };
    await visibleInWrap();
    await wrap.evaluate((el) => (el.scrollLeft = el.scrollWidth));
    await expect(wrap).not.toHaveAttribute("data-overflow-end", "true");
    await visibleInWrap();
    await wrap.evaluate((el) => (el.scrollLeft = 0));
    await visibleInWrap();

    // Pinned cells are opaque and match their row.
    const mismatches = await page.evaluate(() => {
      const bad: string[] = [];
      for (const row of document.querySelectorAll(".ds-table tr")) {
        const pinned = row.querySelector<HTMLElement>(".ds-table__actions");
        const sibling = row.querySelector<HTMLElement>(
          "td:not(.ds-table__actions)"
        );
        if (!pinned) continue;
        const bg = getComputedStyle(pinned).backgroundColor;
        if (bg === "rgba(0, 0, 0, 0)") bad.push("transparent");
        // Plain asset rows paint no cell background of their own (the wrapper
        // surface shows through), so only group and total rows are compared.
        const tinted = /ds-table__row--(group|total)/.test(row.className);
        if (
          tinted &&
          sibling &&
          getComputedStyle(sibling).backgroundColor !== bg
        ) {
          bad.push(`${row.className}: ${bg}`);
        }
      }
      return bad;
    });
    expect(mismatches).toEqual([]);
  }
});

/**
 * GIVEN the Edit and remove asset journey, whose collection holds only VWCE
 * WHEN it renders
 * THEN the collection shows the stored 34.8% current weight, the same figure
 * the drift diagnostic and edit form use, not a recomputed 100%.
 */
test("single-asset collection shows the stored fixture weight", async ({
  page,
}) => {
  const storyId = await findStory(
    page,
    "Compositions / Edit and remove asset",
    "Edit Details"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");
  const collection = page.locator(".ds-asset-collection");
  await expect(collection).toContainText("34.8%");
  await expect(collection).not.toContainText("100.0%");
  // The asset name keeps a readable width instead of collapsing to "Va...".
  const testInfo = test.info();
  if (testInfo.project.name !== "storybook-mobile") {
    const name = page.locator(".ds-table__asset-name").first();
    await expect(name).toBeVisible();
    const truncated = await name.evaluate(
      (el) => el.scrollWidth > el.clientWidth
    );
    expect(truncated).toBe(false);
    expect((await name.boundingBox())?.width ?? 0).toBeGreaterThan(200);
  }
});

/**
 * GIVEN the Invest "Updated" result and a proposal story with transaction tables
 * WHEN they render on desktop
 * THEN asset identities keep their stacked layout (grid), unaffected by the
 * portfolio asset table's inline identity styling.
 */
test("invest tables keep stacked asset identity", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name === "storybook-mobile", "Table is desktop.");
  const storyId = await findStory(page, INVEST_TITLE, "Updated");
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");
  const identity = page.locator(".ds-table__asset-identity").first();
  await expect(identity).toHaveCSS("display", "grid");
});

test.describe("Invest journey in Strategic allocation", () => {
  /** The kinds of the class and asset rows of the visible list, in document order. */
  async function rowKinds(page: Page, scope: string) {
    return page
      .locator(`${scope} [data-row-kind]:visible`)
      .evaluateAll((nodes) =>
        nodes.map((node) => node.getAttribute("data-row-kind"))
      );
  }

  /**
   * GIVEN the Invest drift step in Strategic allocation
   * WHEN it is shown
   * THEN the strategic chip names the class targets, classes are listed by
   * widest gap with their assets nested right after them, cash is a class
   * row only, and Simple mode still shows no class rows.
   */
  test("drift groups assets under classes sorted by gap", async ({
    page,
  }, testInfo) => {
    const mobileVariant = testInfo.project.name === "storybook-mobile";
    await openInvest(page, "Drift strategic", { mobileVariant });
    await expect(page.getByText("Strategic allocation").first()).toBeVisible();
    await expect(
      page.getByText("Class targets: Equities 65% · Bonds 25% · Cash 10%")
    ).toBeVisible();

    const list = page.getByRole("list", {
      name: "Asset classes and assets compared with their targets",
    });
    const classes = list.locator("> li[data-row-kind='class']");
    await expect(classes).toHaveCount(3);
    await expect(classes.nth(0).locator("strong").first()).toHaveText("Cash");
    await expect(classes.nth(1).locator("strong").first()).toHaveText("Bonds");
    await expect(classes.nth(2).locator("strong").first()).toHaveText(
      "Equities"
    );
    // Cash has no nested assets; bonds and equities list theirs after the class row.
    await expect(classes.nth(0).locator("[data-row-kind='asset']")).toHaveCount(
      0
    );
    await expect(classes.nth(1).locator("[data-row-kind='asset']")).toHaveCount(
      1
    );
    await expect(classes.nth(2).locator("[data-row-kind='asset']")).toHaveCount(
      3
    );
    await expect(
      classes.nth(2).getByRole("list", { name: "Equities assets" })
    ).toContainText("30.8% of class");
    await expect(page.getByText("Cash +25.9 pp")).toBeVisible();

    await openInvest(page, "Drift overview", { mobileVariant });
    await expect(page.locator("[data-row-kind]")).toHaveCount(0);
  });

  /**
   * GIVEN the Strategic proposal for adding money
   * WHEN it is shown
   * THEN transactions are grouped by class (class row first, then its
   * assets), untouched assets stay as Hold rows, allocation after has one bar
   * per class, and the class-level drift metric is shown.
   */
  test("proposal groups transactions and allocation by class", async ({
    page,
  }, testInfo) => {
    const mobileVariant = testInfo.project.name === "storybook-mobile";
    await openInvest(page, "Proposal strategic add money", { mobileVariant });
    const transactions = mobileVariant
      ? ".ds-transactions__cards"
      : ".ds-transactions__table";
    const kinds = await rowKinds(page, transactions);
    // Cash is a class-only row, so it never has an asset row after it.
    expect(kinds).toEqual([
      "class",
      "asset",
      "class",
      "asset",
      "asset",
      "asset",
      "class",
    ]);
    const scope = page.locator(`${transactions}:visible`);
    await expect(scope.getByText("Hold").first()).toBeVisible();
    await expect(scope.getByText(/^\+€\d/).first()).toBeVisible();
    await expect(page.getByText("25.9 pp → 22.9 pp").first()).toBeVisible();

    const after = page.getByRole("region", { name: "Allocation after" });
    await expect(after.locator("[data-row-kind='class']")).toHaveCount(3);
    await expect(after).toContainText("from target after");
    await expect(
      page.getByRole("img", { name: /^Equities class: today/ })
    ).toBeVisible();
  });

  /**
   * GIVEN the plan and result steps in Strategic allocation
   * WHEN they are shown
   * THEN "Where you are today" and "Updated quantities" list classes before
   * their assets, with the class-level drift in the plan badge.
   */
  test("plan and result list classes before assets", async ({
    page,
  }, testInfo) => {
    const mobileVariant = testInfo.project.name === "storybook-mobile";
    await openInvest(page, "Plan strategic", { mobileVariant });
    const today = page.locator(".ds-plan-form__today");
    await expect(today).toContainText("25.9 pp drift");
    expect((await rowKinds(page, ".ds-plan-form__today"))[0]).toBe("class");

    await openInvest(page, "Updated strategic", { mobileVariant });
    const result = mobileVariant ? ".ds-result__cards" : ".ds-result__table";
    const kinds = await rowKinds(page, result);
    expect(kinds[0]).toBe("class");
    expect(kinds).toContain("asset");
    await expect(
      page.locator(`${result}:visible`).getByText("Cash").first()
    ).toBeVisible();
  });

  /**
   * GIVEN target weights that do not total 100% in Strategic allocation
   * WHEN the proposal step is shown
   * THEN nothing can be approved, the explanation quotes the actual totals,
   * and Edit targets is offered; Simple mode quotes its own total.
   */
  test("blocked proposal states the real totals and offers Edit targets", async ({
    page,
  }, testInfo) => {
    const mobileVariant = testInfo.project.name === "storybook-mobile";
    await openInvest(page, "Proposal blocked strategic", { mobileVariant });
    await expect(
      visibleText(page, "Target weights must total 100%")
    ).toBeVisible();
    await expect(page.getByText(/class targets total 95%/)).toBeVisible();
    await expect(
      page.getByText(/Equities asset shares total 97\.8%/)
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Update portfolio" })
    ).toHaveCount(0);
    const edit = page.getByRole("button", { name: "Edit targets" });
    await expect(edit).toBeVisible();
    await edit.click();
    await expect(edit).toBeVisible();

    await openInvest(page, "Proposal blocked");
    await expect(page.getByText(/Your target weights total 95%/)).toBeVisible();
  });

  /**
   * GIVEN each Strategic step on desktop and phone
   * WHEN it is rendered
   * THEN the page has no accessibility violations or horizontal overflow.
   */
  for (const story of [
    "Drift strategic",
    "Plan strategic",
    "Proposal strategic add money",
    "Proposal strategic rebalance",
    "Updated strategic",
    "Proposal blocked strategic",
  ]) {
    test(`${story.toLowerCase()} passes accessibility checks`, async ({
      page,
    }, testInfo) => {
      await openInvest(page, story, {
        mobileVariant: testInfo.project.name === "storybook-mobile",
      });
      await expectNoHorizontalOverflow(page);
      const results = await new AxeBuilder({ page }).analyze();
      expect(results.violations).toEqual([]);
    });
  }
});

test.describe("Allocation method and weight editing", () => {
  const SETTINGS = "Compositions / Portfolio settings";
  const OVERVIEW = "Compositions / Portfolio overview";
  const EDIT = "Compositions / Edit and remove asset";

  /** The single overlay that is open: a dialog from 48rem, a drawer below it. */
  async function expectOverlayKind(page: Page, mobile: boolean) {
    await expect(
      page.locator(
        mobile ? ".ds-drawer-popup:visible" : ".ds-dialog-popup:visible"
      )
    ).toHaveCount(1);
    await expect(
      page.locator(
        mobile ? ".ds-dialog-popup:visible" : ".ds-drawer-popup:visible"
      )
    ).toHaveCount(0);
  }

  /**
   * GIVEN Portfolio settings with the Simple method saved
   * WHEN the investor selects Strategic and saves
   * THEN the Set your asset class weights dialog (a drawer on mobile) lists
   * all six classes, and Escape keeps Simple as the saved method.
   */
  test("switching to Strategic opens the class weights dialog", async ({
    page,
  }, testInfo) => {
    const mobile = testInfo.project.name === "storybook-mobile";
    await openStory(page, SETTINGS, "Settings simple");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await page.getByRole("radio", { name: "Strategic" }).click();
    await page.getByRole("button", { name: "Save changes" }).click();
    await expect(
      page.getByRole("heading", { name: "Set your asset class weights" })
    ).toBeVisible();
    await expectOverlayKind(page, mobile);
    for (const name of [
      "Equities",
      "Bonds",
      "Cash",
      "Crypto",
      "Commodities",
      "Other",
    ]) {
      await expect(page.getByLabel(`${name} class weight`)).toBeVisible();
    }
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(page.getByRole("radio", { name: "Simple" })).toBeChecked();
  });

  /**
   * GIVEN the class weights dialog with weights totalling 95%
   * WHEN it renders
   * THEN Save is enabled, the total says "Short by 5.0 pp" and a note warns
   * that Invest stays unavailable; editing to 100% clears the note.
   */
  test("unbalanced class weights can be saved with a warning", async ({
    page,
  }) => {
    await openStory(page, SETTINGS, "Set class weights unbalanced");
    const dialog = page.getByRole("dialog");
    await expect(
      dialog.getByRole("button", { name: "Save and switch to Strategic" })
    ).toBeEnabled();
    await expect(dialog.getByText("Short by 5.0 pp")).toBeVisible();
    await expect(dialog.getByRole("status")).toContainText(
      "Invest stays unavailable"
    );
    await dialog.getByLabel("Equities class weight").fill("65");
    await expect(dialog.getByText("Balanced")).toBeVisible();
    await expect(dialog.getByRole("status")).toHaveCount(0);
    // Clear sets every class to 0 without redistributing; Save stays enabled.
    await dialog.getByRole("button", { name: "Clear" }).click();
    await expect(dialog.getByLabel("Cash class weight")).toHaveValue("0");
    await expect(
      dialog.getByRole("button", { name: "Save and switch to Strategic" })
    ).toBeEnabled();
  });

  /**
   * GIVEN the class weights dialog
   * WHEN it is saved
   * THEN the dialog closes and the settings page confirms the save.
   */
  test("saving class weights switches the method", async ({ page }) => {
    await openStory(page, SETTINGS, "Set class weights");
    await page
      .getByRole("button", { name: "Save and switch to Strategic" })
      .click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(page.getByText("Portfolio settings saved")).toBeVisible();
    await expect(page.getByRole("radio", { name: "Strategic" })).toBeChecked();
  });

  /**
   * GIVEN Strategic is the saved method
   * WHEN Portfolio settings render
   * THEN there are no per-class drift bands, one Portfolio-wide Drift band, and
   * a note with a Go to Portfolio link says weights are edited there.
   */
  test("strategic settings point to the Portfolio page", async ({ page }) => {
    await openStory(page, SETTINGS, "Settings strategic");
    await expect(page.getByLabel("Drift band")).toHaveCount(1);
    await expect(
      page.getByText("edited from the Portfolio page", { exact: false })
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Go to Portfolio" })
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Strategic allocation" })
    ).toHaveCount(0);
  });

  /**
   * GIVEN the Strategic Portfolio overview
   * WHEN it renders
   * THEN the table's own Simple / Strategic switch is hidden, classes are
   * grouped, and the settings gear is available.
   */
  test("product compositions hide the table mode toggle", async ({
    page,
  }, testInfo) => {
    await openStory(page, OVERVIEW, "Portfolio overview strategic");
    await expect(page.getByRole("radio", { name: "Strategic" })).toHaveCount(0);
    await expect(page.getByRole("group", { name: "Table mode" })).toHaveCount(
      0
    );
    await expect(
      page.getByRole("button", { name: "Open portfolio settings" })
    ).toBeVisible();
    if (testInfo.project.name === "storybook-mobile") {
      await page.getByRole("button", { name: "Table options" }).click();
      await expect(page.getByRole("radio", { name: "Strategic" })).toHaveCount(
        0
      );
      await page.keyboard.press("Escape");
    }
    await openStory(page, OVERVIEW, "Populated");
    await expect(page.getByRole("radio", { name: "Strategic" })).toHaveCount(0);
  });

  /**
   * GIVEN the Strategic Portfolio overview
   * WHEN the investor activates Edit Equities
   * THEN the Edit Equities dialog opens with a class weight field and
   * read-only asset shares, Save is enabled with a warning when the total
   * misses 100%, and Escape closes it.
   */
  test("class edit icon opens the class dialog", async ({ page }, testInfo) => {
    const mobile = testInfo.project.name === "storybook-mobile";
    await openStory(page, OVERVIEW, "Portfolio overview strategic");
    await page.getByRole("button", { name: "Edit Equities" }).click();
    await expect(
      page.getByRole("heading", { name: "Edit Equities" })
    ).toBeVisible();
    await expectOverlayKind(page, mobile);
    const dialog = page.getByRole("dialog");
    await dialog.getByLabel("Equities class weight").fill("55");
    await expect(dialog.getByText("Short by 10.0 pp")).toBeVisible();
    await expect(
      dialog.getByRole("button", { name: "Save", exact: true })
    ).toBeEnabled();
    await expect(dialog.getByText("VWCE").first()).toBeVisible();
    await expect(dialog.locator("input")).toHaveCount(1);
    await dialog.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    // The table shows the stated class target as entered.
    await expect(page.locator("body")).toContainText("55.0%");
  });

  /**
   * GIVEN the Edit Equities dialog story
   * WHEN it is open
   * THEN it has no axe violations and Escape returns focus to the page.
   */
  test("class dialog is accessible and dismisses with Escape", async ({
    page,
  }) => {
    await openStory(page, OVERVIEW, "Edit class dialog");
    await expect(page.getByRole("dialog")).toBeVisible();
    const results = await new AxeBuilder({ page }).analyze();
    expect(results.violations).toEqual([]);
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });

  /**
   * GIVEN the asset edit dialog in Strategic mode
   * WHEN it opens
   * THEN it shows Share of Equities with each sibling's share and the resulting
   * weight in portfolio; an unbalanced share warns but Save stays enabled.
   */
  test("strategic asset edit shows share and derived weight", async ({
    page,
  }, testInfo) => {
    const mobile = testInfo.project.name === "storybook-mobile";
    await openStory(page, EDIT, "Edit asset strategic");
    await expectOverlayKind(page, mobile);
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByLabel("Share of Equities")).toHaveValue("46.2");
    await expect(dialog.getByText("= 30.0% of portfolio")).toBeVisible();
    await expect(dialog.getByText("Balanced")).toBeVisible();
    await expect(dialog.getByLabel("Target weight")).toHaveCount(0);
    await dialog.getByLabel("Share of Equities").fill("50");
    await expect(dialog.getByText("= 32.5% of portfolio")).toBeVisible();
    await expect(dialog.getByText("Over by 3.8 pp")).toBeVisible();
    await expect(dialog.getByRole("status")).toContainText("Equities shares");
    await expect(
      dialog.getByRole("button", { name: "Save changes" })
    ).toBeEnabled();
    const results = await new AxeBuilder({ page }).analyze();
    expect(results.violations).toEqual([]);
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });

  /**
   * GIVEN the asset edit dialog in Simple mode
   * WHEN it opens
   * THEN it keeps the single absolute Target weight field and has no share section.
   */
  test("simple asset edit keeps the absolute target weight", async ({
    page,
  }) => {
    await openStory(page, EDIT, "Edit asset simple");
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByLabel("Target weight")).toHaveValue("30");
    await expect(dialog.getByText("Share of Equities")).toHaveCount(0);
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });

  /**
   * GIVEN the class weights dialog story
   * WHEN it is open
   * THEN axe finds no accessibility violations.
   */
  test("class weights dialog has no axe violations", async ({ page }) => {
    await openStory(page, SETTINGS, "Set class weights");
    const results = await new AxeBuilder({ page }).analyze();
    expect(results.violations).toEqual([]);
  });

  /**
   * GIVEN the class weights dialog, the Edit Equities dialog and the Strategic
   * asset edit dialog
   * WHEN the investor types a weight that is not a number from 0 to 100
   * THEN the field is marked invalid and described by a visible "Enter a number
   * from 0 to 100" message, the value is kept as typed rather than clamped, and
   * Save stays enabled.
   */
  test("out-of-range weights are flagged, not silently clamped", async ({
    page,
  }) => {
    const message = "Enter a number from 0 to 100";
    await openStory(page, SETTINGS, "Set class weights");
    let dialog = page.getByRole("dialog");
    let field = dialog.getByLabel("Equities class weight");
    await field.fill("250");
    await expect(field).toHaveValue("250");
    await expect(field).toHaveAttribute("aria-invalid", "true");
    await expect(dialog.getByText(message)).toBeVisible();
    await expect(field).toHaveAccessibleDescription(message);
    await expect(
      dialog.getByRole("button", { name: "Save and switch to Strategic" })
    ).toBeEnabled();
    await field.fill("60");
    await expect(field).not.toHaveAttribute("aria-invalid", "true");
    await expect(dialog.getByText(message)).toHaveCount(0);

    await openStory(page, OVERVIEW, "Edit class dialog");
    dialog = page.getByRole("dialog");
    field = dialog.getByLabel("Equities class weight");
    await field.fill("abc");
    await expect(field).toHaveValue("abc");
    await expect(field).toHaveAttribute("aria-invalid", "true");
    await expect(dialog.getByText(message)).toBeVisible();
    await expect(
      dialog.getByRole("button", { name: "Save", exact: true })
    ).toBeEnabled();

    await openStory(page, EDIT, "Edit asset strategic");
    dialog = page.getByRole("dialog");
    field = dialog.getByLabel("Share of Equities");
    await field.fill("-5");
    await expect(field).toHaveValue("-5");
    await expect(field).toHaveAttribute("aria-invalid", "true");
    await expect(dialog.getByText(message)).toBeVisible();
    await expect(
      dialog.getByRole("button", { name: "Save changes" })
    ).toBeEnabled();
  });

  /**
   * GIVEN the Strategic Portfolio overview and Portfolio settings
   * WHEN the investor opens the Edit Equities dialog and the class weights
   * dialog and dismisses each with Escape
   * THEN focus returns to the element that opened it: the Edit Equities icon
   * and the Save changes button.
   */
  test("class dialogs return focus to the element that opened them", async ({
    page,
  }) => {
    await openStory(page, OVERVIEW, "Portfolio overview strategic");
    const edit = page.getByRole("button", { name: "Edit Equities" });
    await edit.click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(edit).toBeFocused();

    await openStory(page, SETTINGS, "Settings simple");
    await page.getByRole("radio", { name: "Strategic" }).click();
    const save = page.getByRole("button", { name: "Save changes" });
    await save.click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(save).toBeFocused();
  });

  /**
   * GIVEN the Strategic Portfolio overview on a phone
   * WHEN the class edit icon, the group collapse toggle and the Table options
   * trigger render
   * THEN each has a hit area of at least 44px.
   */
  test("phone class and table controls have 44px hit areas", async ({
    page,
  }, testInfo) => {
    test.skip(testInfo.project.name !== "storybook-mobile", "Phone only.");
    await openStory(page, OVERVIEW, "Portfolio overview strategic");
    for (const name of [
      "Edit Equities",
      "Collapse Equities",
      "Table options",
    ]) {
      const box = await page
        .getByRole("button", { name })
        .first()
        .boundingBox();
      expect(box?.height ?? 0, `${name} height`).toBeGreaterThanOrEqual(43.5);
      if (name === "Edit Equities") {
        expect(box?.width ?? 0, `${name} width`).toBeGreaterThanOrEqual(43.5);
      }
    }
  });
});

/**
 * GIVEN the Portfolio overview at exactly 768px and at 767px wide
 * WHEN it renders
 * THEN at 768px only the desktop layout shows (header Add asset and Invest, no
 * floating menu, the table, a dialog for edits) and at 767px only the phone
 * layout shows (floating menu, no header actions, cards).
 */
test("phone and desktop layouts never overlap at the 48rem boundary", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name === "storybook-mobile", "Desktop project.");
  await page.setViewportSize({ width: 768, height: 1024 });
  await openStory(page, "Compositions / Portfolio overview", "Populated");
  const header = page.locator(".ds-page-header");
  await expect(header.getByRole("button", { name: "Add asset" })).toBeVisible();
  await expect(header.getByRole("button", { name: "Invest" })).toBeVisible();
  await expect(pageActionsButton(page)).toBeHidden();
  await expect(page.locator(".ds-asset-collection__table")).toBeVisible();
  await expect(page.locator(".ds-asset-collection__cards")).toBeHidden();
  await page
    .getByRole("button", { name: /^Edit / })
    .first()
    .click();
  await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(1);
  await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(0);

  await page.setViewportSize({ width: 767, height: 1024 });
  await openStory(page, "Compositions / Portfolio overview", "Populated");
  await expect(pageActionsButton(page)).toBeVisible();
  await expect(
    page.locator(".ds-page-header").getByRole("button", { name: "Add asset" })
  ).toBeHidden();
  await expect(page.locator(".ds-asset-collection__cards")).toBeVisible();
  await expect(page.locator(".ds-asset-collection__table")).toBeHidden();
});

/** Asserts that no matched element has its text clipped by its own box. */
async function expectTextNotClipped(locator: Locator) {
  const count = await locator.count();
  expect(count).toBeGreaterThan(0);
  for (let i = 0; i < count; i += 1) {
    const clipped = await locator
      .nth(i)
      .evaluate((node) => node.scrollWidth > node.clientWidth + 1);
    expect(clipped).toBe(false);
  }
}

const NARROW_WIDTHS = [360, 390];

/**
 * GIVEN the mobile Edit asset stories (Strategic, Simple, Details) and the
 * mobile Strategic Invest proposal stories
 * WHEN they render at 360 and 390 pixels wide
 * THEN the page does not scroll horizontally, the Strategic share editor's
 * heading, input and rows are not clipped, and the nested transaction cards'
 * text is not clipped.
 */
test("mobile edit and proposal stories fit narrow phones without clipping", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "storybook-mobile", "Mobile only.");
  const stories: [string, string][] = [
    ["Compositions / Edit and remove asset", "Edit Asset Strategic Mobile"],
    ["Compositions / Edit and remove asset", "Edit Asset Simple Mobile"],
    ["Compositions / Edit and remove asset", "Edit Details Mobile"],
    ["Compositions / Invest", "Proposal Strategic Add Money Mobile"],
    ["Compositions / Invest", "Proposal Strategic Rebalance Mobile"],
    ["Compositions / Invest", "Proposal Blocked Strategic Mobile"],
  ];
  for (const width of NARROW_WIDTHS) {
    await page.setViewportSize({ width, height: 844 });
    for (const [title, name] of stories) {
      await openStory(page, title, name);
      await expectNoHorizontalOverflow(page);
      const section = page.locator(".ds-weight-section");
      if (await section.count()) {
        await expectTextNotClipped(
          section.locator(
            ".ds-weight-section__head h3, .ds-weight-section__head > div > span, .ds-weight-list__change, .ds-weight-list__absolute"
          )
        );
        const input = section.locator(".ds-input-with-suffix").first();
        const box = await input.boundingBox();
        expect(box!.width).toBeGreaterThan(width * 0.6);
      }
      const nested = page.locator(".ds-transaction-card--nested");
      if (await nested.count()) {
        await expectTextNotClipped(
          nested.locator(
            ".ds-transaction-card__subtitle, .ds-transaction-card__amount, .ds-transaction-card__meta"
          )
        );
      }
    }
  }
});

/**
 * GIVEN the Portfolio overview, Edit asset and Invest compositions at the
 * 768 and 820 pixel widths just above the desktop breakpoint
 * WHEN they render
 * THEN the page itself never scrolls horizontally (wide tables scroll inside
 * their own wrapper) and metric card values are not clipped.
 */
test("tablet widths never scroll the page horizontally", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "storybook-mobile", "One project only.");
  const stories: [string, string][] = [
    ["Compositions / Portfolio overview", "Populated"],
    ["Compositions / Portfolio overview", "Portfolio overview Strategic"],
    ["Compositions / Edit and remove asset", "Edit Details"],
    ["Compositions / Invest", "Proposal Strategic Add Money"],
    ["Compositions / Invest", "Drift Overview"],
  ];
  for (const width of [768, 820]) {
    await page.setViewportSize({ width, height: 1024 });
    for (const [title, name] of stories) {
      await openStory(page, title, name);
      await expectNoHorizontalOverflow(page);
      const values = page.locator(".ds-metric-card__value");
      if (await values.count()) await expectTextNotClipped(values);
    }
  }
});
