import { portfolioAssets, type AssetClass } from "../../fixtures/portfolio";
import type { AllocationMode } from "./invest-types";

/** The three steps of the first-use setup wizard, in order. */
export const setupWizardStepIds = ["name", "asset", "weights"] as const;
export type SetupWizardStepId = (typeof setupWizardStepIds)[number];

/** What the investor has entered so far across the steps. */
export interface SetupWizardValues {
  name: string;
  currency: string;
  /** Id of the chosen first asset; null until one is picked. */
  assetId: string | null;
  /** Text typed in the asset search field. */
  assetQuery: string;
  /** Target weight as typed, so partial input is kept as-is. */
  weight: string;
}

/** Values a fresh wizard starts with: one asset means a 100% target. */
export const emptySetupWizardValues: SetupWizardValues = {
  name: "",
  currency: "EUR",
  assetId: null,
  assetQuery: "",
  weight: "100",
};

/** Words shown for one step. */
export interface SetupWizardStepCopy {
  id: SetupWizardStepId;
  /** Short name in the progress bar. */
  label: string;
  /** Title of the step panel. */
  heading: string;
  /** One line on why the step matters. */
  reason: string;
}

/**
 * The words of each step. The last step follows the Portfolio's allocation
 * method: Simple asks for the asset's target weight, Strategic for the weight
 * of its asset class.
 */
export function setupWizardSteps(
  mode: AllocationMode
): readonly SetupWizardStepCopy[] {
  const strategic = mode === "strategic";
  return [
    {
      id: "name",
      label: "Name",
      heading: "Name your portfolio",
      reason:
        "A name and a base currency. Every value in this portfolio is shown in that currency.",
    },
    {
      id: "asset",
      label: "First asset",
      heading: "Add your first asset",
      reason:
        "Pick an asset to track. You can add quantities and more assets right after setup.",
    },
    {
      id: "weights",
      label: strategic ? "Class weight" : "Target weight",
      heading: strategic ? "Set the class weight" : "Set target weights",
      reason: strategic
        ? "The share of your portfolio for this asset's class. With one class it is 100%; you will split it as you add classes."
        : "The share of your portfolio this asset should make up. With one asset it is 100%; you will rebalance as you add more.",
    },
  ];
}

/** An asset the wizard can offer as a first holding. */
export interface SetupAssetOption {
  id: string;
  ticker: string;
  name: string;
  assetClass: AssetClass;
}

/** Investable fixtures (everything but cash), in fixture order. */
export const setupAssetOptions: readonly SetupAssetOption[] = portfolioAssets
  .filter((asset) => asset.assetClass !== "cash")
  .map(({ id, ticker, name, assetClass }) => ({
    id,
    ticker,
    name,
    assetClass,
  }));

/** The picks offered before the investor types anything. */
export const popularSetupAssets: readonly SetupAssetOption[] =
  setupAssetOptions.slice(0, 4);

/** Assets whose ticker or name contains the query, ignoring case. */
export function searchSetupAssets(
  query: string,
  options: readonly SetupAssetOption[] = setupAssetOptions
): SetupAssetOption[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return [...options];
  return options.filter(
    (option) =>
      option.ticker.toLowerCase().includes(needle) ||
      option.name.toLowerCase().includes(needle)
  );
}

/** Result of checking the typed target weight. */
export interface SetupWeightCheck {
  valid: boolean;
  /** The parsed weight; null when the text is not a number. */
  value: number | null;
  /** What to tell the investor, whether the weight is valid or not. */
  message: string;
}

/**
 * Checks the weight of the only asset (Simple) or only class (Strategic).
 * It must be a plain number, comma or dot decimal, and total exactly 100%.
 */
export function checkSetupWeight(
  text: string,
  mode: AllocationMode
): SetupWeightCheck {
  const noun = mode === "strategic" ? "Class weights" : "Weights";
  const trimmed = text.trim().replace(",", ".");
  if (!trimmed) {
    return { valid: false, value: null, message: "Enter a weight." };
  }
  if (!/^\d+(\.\d+)?$/.test(trimmed)) {
    return {
      valid: false,
      value: null,
      message: "Enter a number such as 100 or 62.5.",
    };
  }
  const value = Number(trimmed);
  if (value > 100) {
    return {
      valid: false,
      value,
      message: "A weight cannot be more than 100%.",
    };
  }
  if (Math.abs(value - 100) >= 0.05) {
    return {
      valid: false,
      value,
      message: `${noun} must total 100%. Right now: ${value.toFixed(1)}%.`,
    };
  }
  return { valid: true, value, message: `${noun} total 100%.` };
}

/** Whether the values entered for one step allow moving on. */
export function isSetupStepValid(
  stepId: SetupWizardStepId,
  values: SetupWizardValues,
  mode: AllocationMode
): boolean {
  switch (stepId) {
    case "name":
      return Boolean(values.name.trim() && values.currency);
    case "asset":
      return values.assetId !== null;
    case "weights":
      return checkSetupWeight(values.weight, mode).valid;
  }
}

/** Index of the first step with unusable values; -1 when every step is valid. */
export function firstInvalidSetupStep(
  values: SetupWizardValues,
  mode: AllocationMode
): number {
  return setupWizardStepIds.findIndex(
    (id) => !isSetupStepValid(id, values, mode)
  );
}

/**
 * Whether a step in the progress bar can be opened: the investor has reached
 * it before and every step ahead of it is still valid.
 */
export function canVisitSetupStep(
  index: number,
  furthestStep: number,
  values: SetupWizardValues,
  mode: AllocationMode
): boolean {
  if (index < 0 || index >= setupWizardStepIds.length) return false;
  if (index > furthestStep) return false;
  return setupWizardStepIds
    .slice(0, index)
    .every((id) => isSetupStepValid(id, values, mode));
}
