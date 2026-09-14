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
        const trigger =
          testInfo.project.name === "storybook-mobile"
            ? page.getByRole("button", { name: "Open add asset actions" })
            : page.getByRole("button", { name: "Add asset" });
        await expect(trigger).toBeVisible();
        await trigger.click();
        if (testInfo.project.name === "storybook-mobile") {
          const addAssetAction = page.getByRole("button", {
            name: "Add asset",
          });
          await expect(addAssetAction).toBeVisible();
          await addAssetAction.click();
        }
        await expect(page.getByRole("dialog")).toBeVisible();
        await expect(
          page.getByRole("textbox", { name: /Search by name or ticker/i })
        ).toBeFocused();

        const openResults = await new AxeBuilder({ page }).analyze();
        expect(openResults.violations).toEqual([]);

        await page.keyboard.press("Escape");
        await expect(page.getByRole("dialog")).toBeHidden();
        if (testInfo.project.name !== "storybook-mobile") {
          await expect(trigger).toBeFocused();
        }
      }

      if (journey === "portfolio overview") {
        await expect(
          page.locator('button[aria-label="Edit VWCE"]:visible')
        ).toBeVisible();
        await expect(
          page.locator('button[aria-label="More actions for VWCE"]:visible')
        ).toBeVisible();

        const fab = page.getByRole("button", {
          name: "Open portfolio actions",
        });
        await fab.click();
        const closeFab = page.getByRole("button", {
          name: "Close portfolio actions",
        });
        await expect(closeFab).toBeVisible();
        for (const action of [
          "Add asset",
          "Allocate",
          "Rebalance",
          "Edit portfolio",
        ]) {
          await expect(
            page.getByRole("button", { name: action }).last()
          ).toBeVisible();
        }
        await page.keyboard.press("Tab");
        await expect(
          page.getByRole("button", { name: "Add asset" }).last()
        ).toBeFocused();
        await closeFab.click();

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

test("overview currency popup stays compact and follows its trigger", async ({
  page,
}) => {
  const storyId = await findStory(
    page,
    "Compositions / Portfolio overview",
    "Populated"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  const trigger = page.locator(".ds-chart__currency:visible");
  await trigger.click();
  const popup = page.locator(".ds-chart__currency-popup:visible");
  await expect(popup).toBeVisible();
  await expect(popup).toContainText("EUR");
  const triggerBox = await trigger.boundingBox();
  const popupBox = await popup.boundingBox();
  expect(triggerBox).not.toBeNull();
  expect(popupBox).not.toBeNull();
  if (triggerBox && popupBox) {
    expect(Math.abs(popupBox.x - triggerBox.x)).toBeLessThanOrEqual(2);
    expect(Math.abs(popupBox.width - triggerBox.width)).toBeLessThanOrEqual(2);
  }
  await page.keyboard.press("Escape");
  await expect(popup).toBeHidden();
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
    "Allocation Review Deltas"
  );
  await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
  await page.waitForLoadState("networkidle");

  for (const label of [
    "Amount",
    "Weight",
    "Commission charged",
    "Fee impact",
  ]) {
    await expect(page.getByText(label, { exact: true })).toBeVisible();
  }
  await expect(page.getByText("€2,607.36", { exact: true })).toBeVisible();
  await expect(page.getByText("+€977.76", { exact: true })).toBeVisible();
  await expect(page.getByText("100.0%", { exact: true })).toBeVisible();
  await expect(page.getByText("+70.0 pp", { exact: true })).toBeVisible();
  await expect(page.getByText("€0.00", { exact: true })).toBeVisible();
  await expect(page.getByText("0.0%", { exact: true })).toBeVisible();
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
      "Portfolio default"
    );

    await feeTrigger.click();
    await page.getByRole("radio", { name: "Fixed" }).click();
    await page.getByLabel("Maximum impact").fill("9");
    await page.getByLabel("Amount").fill("99");
    await page.getByRole("radio", { name: "Portfolio default" }).click();
    await expect(page.locator(".ds-fee-configuration:visible")).toContainText(
      "maximum impact 0.5% · amount €2.50"
    );
    await page.keyboard.press("Escape");
    await expect(page.locator(".ds-fee-summary:visible")).toContainText(
      "Portfolio default"
    );

    await feeTrigger.click();
    await page.getByRole("radio", { name: "Fixed" }).click();

    if (testInfo.project.name === "storybook-mobile") {
      await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(1);
      await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(1);
    } else {
      await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(2);
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

  test("allocation footers keep compact ghost Back left and primary action right", async ({
    page,
  }) => {
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
    await expect(optionsButtons.first()).toHaveClass(/ds-button--ghost/);
    const optionsLayout = await optionsButtons.evaluateAll((buttons) =>
      buttons.map((button) => ({
        x: button.getBoundingClientRect().x,
        width: button.getBoundingClientRect().width,
      }))
    );
    expect(optionsLayout[0].x).toBeLessThan(optionsLayout[1].x);

    const resultId = await findStory(
      page,
      "Compositions / Allocate and rebalance",
      "Allocation result"
    );
    await page.goto(`/iframe.html?id=${encodeURIComponent(resultId)}`);
    await page.waitForLoadState("networkidle");
    const resultFooter = page.locator(".ds-allocation-footer:visible");
    const resultButtons = resultFooter.getByRole("button");
    await expect(resultButtons.first()).toHaveClass(/ds-button--ghost/);
    const resultLayout = await resultButtons.evaluateAll((buttons) =>
      buttons.map((button) => button.getBoundingClientRect().x)
    );
    expect(resultLayout[0]).toBeLessThan(resultLayout[1]);
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
    await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(1);
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
    await expect(page.locator(".ds-dialog-popup:visible")).toHaveCount(1);
    await expect(page.locator(".ds-drawer-popup:visible")).toHaveCount(0);
  });

  test("floating actions render trailing icons for every board action", async ({
    page,
  }) => {
    const storyId = await findStory(
      page,
      "Patterns / Portfolio",
      "Floating actions"
    );
    await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
    await page.waitForLoadState("networkidle");
    await page.getByRole("button", { name: "Open portfolio actions" }).click();
    const actions = page.locator(".ds-fab__actions .ds-button:visible");
    await expect(actions).toHaveCount(4);
    await expect(actions.nth(0)).toContainText("Add asset");
    await expect(actions.nth(0).locator(".ds-fab__action-icon")).toHaveCount(1);
    await expect(actions.nth(3)).toContainText("Edit portfolio");
    await expect(actions.nth(3).locator(".ds-fab__action-icon")).toHaveCount(1);
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
    test(`${storyName} opens only its active dialog`, async ({ page }) => {
      const storyId = await findStory(
        page,
        "Compositions / Edit and remove asset",
        storyName
      );
      await page.goto(`/iframe.html?id=${encodeURIComponent(storyId)}`);
      await page.waitForLoadState("networkidle");

      await expect(page.getByRole("dialog")).toHaveCount(1);
      await expect(page.getByRole("button", { name: "Cancel" })).toHaveCount(0);
      await page.keyboard.press("Escape");
      await expect(page.getByRole("dialog")).toBeHidden();
    });
  }
});
