import { describe, expect, it } from "vitest";

import {
  canVisitSetupStep,
  checkSetupWeight,
  emptySetupWizardValues,
  firstInvalidSetupStep,
  isSetupStepValid,
  searchSetupAssets,
  setupAssetOptions,
  setupWizardSteps,
  type SetupWizardValues,
} from "./portfolio-setup-wizard";

const complete: SetupWizardValues = {
  name: "Starter",
  currency: "EUR",
  assetId: setupAssetOptions[0].id,
  assetQuery: "",
  weight: "100",
};

/**
 * GIVEN the Portfolio's allocation method
 * WHEN the wizard words its third step
 * THEN Simple asks for target weights and Strategic for the class weight.
 */
describe("setupWizardSteps", () => {
  it("words the last step by allocation method", () => {
    expect(setupWizardSteps("simple")[2]).toMatchObject({
      label: "Target weight",
      heading: "Set target weights",
    });
    expect(setupWizardSteps("strategic")[2]).toMatchObject({
      label: "Class weight",
      heading: "Set the class weight",
    });
  });
});

/**
 * GIVEN text typed as the weight of the only asset
 * WHEN it is checked
 * THEN only exactly 100% is valid and every other input says why not.
 */
describe("checkSetupWeight", () => {
  it.each([
    ["100", true],
    ["100.0", true],
    ["100,0", true],
    ["99.96", true],
    [" 100 ", true],
    ["", false],
    ["abc", false],
    ["-5", false],
    ["60", false],
    ["100.5", false],
    ["150", false],
  ])("treats %j as valid=%s", (text, valid) => {
    expect(checkSetupWeight(text, "simple").valid).toBe(valid);
  });

  it("explains how far off the total is", () => {
    expect(checkSetupWeight("60", "simple").message).toBe(
      "Weights must total 100%. Right now: 60.0%."
    );
    expect(checkSetupWeight("60", "strategic").message).toBe(
      "Class weights must total 100%. Right now: 60.0%."
    );
    expect(checkSetupWeight("150", "simple").message).toBe(
      "A weight cannot be more than 100%."
    );
  });
});

/**
 * GIVEN the values entered so far
 * WHEN each step is checked
 * THEN a step is valid only with its own required values.
 */
describe("isSetupStepValid", () => {
  it("requires a name, a chosen asset and a 100% weight", () => {
    expect(isSetupStepValid("name", emptySetupWizardValues, "simple")).toBe(
      false
    );
    expect(isSetupStepValid("name", complete, "simple")).toBe(true);
    expect(
      isSetupStepValid("name", { ...complete, name: "   " }, "simple")
    ).toBe(false);
    expect(
      isSetupStepValid("asset", { ...complete, assetId: null }, "simple")
    ).toBe(false);
    expect(
      isSetupStepValid("weights", { ...complete, weight: "50" }, "simple")
    ).toBe(false);
  });

  it("reports the first step that needs attention", () => {
    expect(firstInvalidSetupStep(complete, "simple")).toBe(-1);
    expect(firstInvalidSetupStep({ ...complete, name: "" }, "simple")).toBe(0);
    expect(firstInvalidSetupStep({ ...complete, weight: "1" }, "simple")).toBe(
      2
    );
  });
});

/**
 * GIVEN how far the investor has got
 * WHEN a progress-bar step is clicked
 * THEN it opens only when it was reached and the steps before it are valid.
 */
describe("canVisitSetupStep", () => {
  it("allows reached steps only", () => {
    expect(canVisitSetupStep(0, 1, complete, "simple")).toBe(true);
    expect(canVisitSetupStep(1, 1, complete, "simple")).toBe(true);
    expect(canVisitSetupStep(2, 1, complete, "simple")).toBe(false);
  });

  it("blocks a step once an earlier one became invalid", () => {
    expect(canVisitSetupStep(2, 2, { ...complete, name: "" }, "simple")).toBe(
      false
    );
    expect(canVisitSetupStep(0, 2, { ...complete, name: "" }, "simple")).toBe(
      true
    );
  });
});

/**
 * GIVEN the fixture assets
 * WHEN the investor searches
 * THEN tickers and names match ignoring case, and an empty query lists all.
 */
describe("searchSetupAssets", () => {
  it("matches ticker or name", () => {
    expect(searchSetupAssets("vwce").map((a) => a.ticker)).toEqual(["VWCE"]);
    expect(searchSetupAssets("bond").length).toBeGreaterThan(0);
    expect(searchSetupAssets("zzzz")).toEqual([]);
    expect(searchSetupAssets("  ")).toHaveLength(setupAssetOptions.length);
  });
});
