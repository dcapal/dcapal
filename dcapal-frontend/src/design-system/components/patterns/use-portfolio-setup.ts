import * as React from "react";
import type { AllocationMode } from "./invest-types";
import {
  canVisitSetupStep,
  emptySetupWizardValues,
  firstInvalidSetupStep,
  isSetupStepValid,
  setupWizardStepIds,
  type SetupWizardStepId,
  type SetupWizardValues,
} from "./portfolio-setup-wizard";

/** Values submitted when the name and currency step is completed. */
export interface PortfolioSetupNameDetails {
  stepId: "name";
  name: string;
  currency: string;
}

/** Values submitted when the first asset step is completed. */
export interface PortfolioSetupAssetDetails {
  stepId: "asset";
  /** Id of the asset the investor picked. */
  assetId: string;
}

/** Values submitted when the target weight step is completed. */
export interface PortfolioSetupWeightsDetails {
  stepId: "weights";
  targetWeight: string;
}

export type PortfolioSetupContinueDetails =
  | PortfolioSetupNameDetails
  | PortfolioSetupAssetDetails
  | PortfolioSetupWeightsDetails;

export interface UsePortfolioSetupOptions {
  /** Index of the step shown first; earlier steps count as completed. */
  initialStep?: number;
  /** Starts with the setup already finished, so the wizard renders nothing. */
  initialDone?: boolean;
  initialValues?: Partial<SetupWizardValues>;
  /** The Portfolio's allocation method; it words and validates the weight step. */
  allocationMode?: AllocationMode;
  /** Called with the step's values each time the investor continues past it. */
  onContinue?: (details: PortfolioSetupContinueDetails) => void;
  /** Called when the investor opens a step from the progress bar. */
  onStepSelect?: (stepId: SetupWizardStepId) => void;
  /** Called once when the last step is finished. */
  onFinish?: (values: SetupWizardValues) => void;
}

function detailsFor(
  stepId: SetupWizardStepId,
  values: SetupWizardValues
): PortfolioSetupContinueDetails {
  switch (stepId) {
    case "name":
      return { stepId, name: values.name.trim(), currency: values.currency };
    case "asset":
      return { stepId, assetId: values.assetId ?? "" };
    case "weights":
      return { stepId, targetWeight: values.weight.trim() };
  }
}

/**
 * State of the first-use setup wizard: the step on screen, how far the
 * investor has got, the values entered and the moves between steps. The last
 * step finishes the setup, after which `done` stays true.
 */
export function usePortfolioSetup({
  initialStep = 0,
  initialDone = false,
  initialValues,
  allocationMode = "simple",
  onContinue,
  onStepSelect,
  onFinish,
}: UsePortfolioSetupOptions = {}) {
  const lastIndex = setupWizardStepIds.length - 1;
  const [stepIndex, setStepIndex] = React.useState(initialStep);
  const [furthestStep, setFurthestStep] = React.useState(initialStep);
  const [completedCount, setCompletedCount] = React.useState(initialStep);
  const [done, setDone] = React.useState(initialDone);
  const [values, setValues] = React.useState<SetupWizardValues>({
    ...emptySetupWizardValues,
    ...initialValues,
  });

  // Follow the starting point when a story changes it after mount.
  const initialValuesRef = React.useRef(initialValues);
  initialValuesRef.current = initialValues;
  const mounted = React.useRef(false);
  React.useEffect(() => {
    if (!mounted.current) {
      mounted.current = true;
      return;
    }
    setStepIndex(initialStep);
    setFurthestStep(initialStep);
    setCompletedCount(initialStep);
    setDone(initialDone);
    setValues({ ...emptySetupWizardValues, ...initialValuesRef.current });
  }, [initialStep, initialDone]);

  const stepId = setupWizardStepIds[stepIndex];
  const canContinue = isSetupStepValid(stepId, values, allocationMode);

  const update = React.useCallback((patch: Partial<SetupWizardValues>) => {
    setValues((current) => ({ ...current, ...patch }));
  }, []);

  /** Moves past the current step, or finishes the setup from the last one. */
  const next = () => {
    if (!canContinue) return;
    onContinue?.(detailsFor(stepId, values));
    setCompletedCount((count) => Math.max(count, stepIndex + 1));
    if (stepIndex < lastIndex) {
      setStepIndex(stepIndex + 1);
      setFurthestStep((furthest) => Math.max(furthest, stepIndex + 1));
      return;
    }
    // An earlier step may have been emptied after it was completed.
    const invalid = firstInvalidSetupStep(values, allocationMode);
    if (invalid >= 0) {
      setStepIndex(invalid);
      return;
    }
    setDone(true);
    onFinish?.(values);
  };

  const back = () => setStepIndex((index) => Math.max(index - 1, 0));

  const canVisit = (index: number) =>
    canVisitSetupStep(index, furthestStep, values, allocationMode);

  const goTo = (index: number) => {
    if (index === stepIndex || !canVisit(index)) return;
    setStepIndex(index);
    onStepSelect?.(setupWizardStepIds[index]);
  };

  return {
    allocationMode,
    stepIndex,
    stepId,
    isLastStep: stepIndex === lastIndex,
    completedCount,
    done,
    values,
    canContinue,
    update,
    next,
    back,
    canVisit,
    goTo,
  };
}

export type PortfolioSetup = ReturnType<typeof usePortfolioSetup>;
