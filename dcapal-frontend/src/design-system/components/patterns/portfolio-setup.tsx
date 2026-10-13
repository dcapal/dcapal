import * as React from "react";
import {
  Check,
  ChevronRight,
  ArrowLeft,
  FilePlus2,
  PieChart,
  Plus,
  X,
} from "lucide-react";
import {
  Button,
  Card,
  CardContent,
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerTitle,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  IconButton,
} from "../ui";
import { FormField } from "./form-field";
import { cn } from "../../lib/cn";

export type PortfolioSetupStepStatus = "complete" | "current" | "upcoming";
export type PortfolioSetupStepId = "name" | "asset" | "weights";

export interface PortfolioSetupStep {
  id: PortfolioSetupStepId;
  title: string;
  description: string;
  status: PortfolioSetupStepStatus;
  icon?: React.ReactNode;
}

/** Values submitted when the name and currency step is completed. */
export interface PortfolioSetupNameDetails {
  stepId: "name";
  name: string;
  currency: string;
}

/** Values submitted when the first asset step is completed. */
export interface PortfolioSetupAssetDetails {
  stepId: "asset";
  assetQuery: string;
}

/** Values submitted when the target weights step is completed. */
export interface PortfolioSetupWeightsDetails {
  stepId: "weights";
  targetWeight: string;
}

export type PortfolioSetupContinueDetails =
  | PortfolioSetupNameDetails
  | PortfolioSetupAssetDetails
  | PortfolioSetupWeightsDetails;

export interface PortfolioSetupChecklistProps {
  steps?: ReadonlyArray<PortfolioSetupStep>;
  onStepSelect?: (step: PortfolioSetupStep) => void;
}

/** Shared first-use step definitions used by the index and setup page. */
export const portfolioSetupSteps: PortfolioSetupStep[] = [
  {
    id: "name",
    title: "Name and currency",
    description: "Give your portfolio a name and choose its base currency.",
    status: "current",
    icon: <FilePlus2 aria-hidden />,
  },
  {
    id: "asset",
    title: "Add your first asset",
    description: "Search for an asset and add it to your portfolio.",
    status: "upcoming",
    icon: <Plus aria-hidden />,
  },
  {
    id: "weights",
    title: "Set target weights",
    description: "Define your target allocation across assets.",
    status: "upcoming",
    icon: <PieChart aria-hidden />,
  },
];

function getStepIndex(
  stepId: PortfolioSetupStepId,
  steps: ReadonlyArray<PortfolioSetupStep> = portfolioSetupSteps
) {
  return Math.max(
    0,
    steps.findIndex((step) => step.id === stepId)
  );
}

/** Actionable first-use checklist that can resume the current setup step. */
export function PortfolioSetupChecklist({
  steps = portfolioSetupSteps,
  onStepSelect,
}: PortfolioSetupChecklistProps) {
  return (
    <section className="ds-portfolio-setup" aria-labelledby="setup-title">
      <div className="ds-portfolio-setup__heading">
        <h2 id="setup-title">Set up your portfolio</h2>
        <p>Complete the steps below to get started.</p>
      </div>
      <div className="ds-portfolio-setup__list">
        {steps.map((step) => (
          <button
            type="button"
            className="ds-portfolio-setup__step"
            key={step.id}
            data-status={step.status}
            onClick={() => onStepSelect?.(step)}
            disabled={!onStepSelect || step.status === "upcoming"}
          >
            <span className="ds-portfolio-setup__step-icon" aria-hidden>
              {step.status === "complete" ? <Check /> : step.icon}
            </span>
            <span className="ds-portfolio-setup__step-copy">
              <strong>{step.title}</strong>
              <span>{step.description}</span>
            </span>
            <ChevronRight
              aria-hidden
              className="ds-portfolio-setup__step-arrow"
            />
          </button>
        ))}
      </div>
    </section>
  );
}

interface PortfolioSetupFieldsProps {
  name: string;
  onNameChange: (name: string) => void;
  currency: string;
  onCurrencyChange: (currency: string) => void;
}

/** Shared name and base-currency controls for the inline and sheet forms. */
function PortfolioSetupFields({
  name,
  onNameChange,
  currency,
  onCurrencyChange,
}: PortfolioSetupFieldsProps) {
  return (
    <div className="ds-portfolio-setup-sheet__fields">
      <FormField label="Portfolio name">
        <Input
          value={name}
          placeholder="e.g. Long-term Growth"
          onChange={(event) => onNameChange(event.target.value)}
        />
      </FormField>
      <FormField label="Base currency">
        <Select
          value={currency}
          onValueChange={(value) => onCurrencyChange(value ?? "")}
        >
          <SelectTrigger aria-label="Base currency">
            <SelectValue placeholder="Select currency" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="EUR">EUR · Euro</SelectItem>
            <SelectItem value="USD">USD · US dollar</SelectItem>
            <SelectItem value="GBP">GBP · Pound sterling</SelectItem>
          </SelectContent>
        </Select>
      </FormField>
    </div>
  );
}

export interface PortfolioSetupFormProps {
  className?: string;
  initialName?: string;
  initialCurrency?: string;
  onContinue?: (details: PortfolioSetupContinueDetails) => void;
}

/** Inline first step of the portfolio wizard for the setup page. */
export function PortfolioSetupForm({
  className,
  initialName = "",
  initialCurrency = "",
  onContinue,
}: PortfolioSetupFormProps) {
  const [name, setName] = React.useState(initialName);
  const [currency, setCurrency] = React.useState(initialCurrency);

  React.useEffect(() => {
    setName(initialName);
    setCurrency(initialCurrency);
  }, [initialCurrency, initialName]);

  return (
    <div className={cn("ds-portfolio-setup-form", className)}>
      <PortfolioSetupFields
        name={name}
        onNameChange={setName}
        currency={currency}
        onCurrencyChange={setCurrency}
      />
      <Button
        className="ds-portfolio-setup-sheet__continue"
        disabled={!name.trim() || !currency}
        onClick={() =>
          onContinue?.({ stepId: "name", name: name.trim(), currency })
        }
      >
        Continue
      </Button>
    </div>
  );
}

export interface PortfolioSetupSheetProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onContinue?: (details: PortfolioSetupContinueDetails) => void;
  step?: PortfolioSetupStep;
  steps?: ReadonlyArray<PortfolioSetupStep>;
  initialName?: string;
  initialCurrency?: string;
  initialAssetQuery?: string;
  initialTargetWeight?: string;
}

/** Focused mobile-first setup surface backed by the shared Base UI drawer. */
export function PortfolioSetupSheet({
  open,
  onOpenChange,
  onContinue,
  step = portfolioSetupSteps[0],
  steps = portfolioSetupSteps,
  initialName = "",
  initialCurrency = "",
  initialAssetQuery = "",
  initialTargetWeight = "",
}: PortfolioSetupSheetProps) {
  const [name, setName] = React.useState(initialName);
  const [currency, setCurrency] = React.useState(initialCurrency);
  const [assetQuery, setAssetQuery] = React.useState("");
  const [targetWeight, setTargetWeight] = React.useState("");

  React.useEffect(() => {
    if (open) {
      setName(initialName);
      setCurrency(initialCurrency);
      setAssetQuery(initialAssetQuery);
      setTargetWeight(initialTargetWeight);
    }
  }, [
    initialAssetQuery,
    initialCurrency,
    initialName,
    initialTargetWeight,
    open,
    step.id,
  ]);

  const canContinue =
    step.status !== "upcoming" &&
    (step.id === "name"
      ? Boolean(name.trim() && currency)
      : step.id === "asset"
        ? Boolean(assetQuery.trim())
        : Boolean(targetWeight.trim()));

  return (
    <Drawer open={open} onOpenChange={onOpenChange}>
      <DrawerContent className="ds-portfolio-setup-sheet">
        <div className="ds-portfolio-setup-sheet__header">
          <div>
            <span className="ds-portfolio-setup-sheet__step-count">
              Step {getStepIndex(step.id, steps) + 1} of {steps.length}
            </span>
            <DrawerTitle>
              {step.id === "name" ? "New portfolio" : step.title}
            </DrawerTitle>
            <DrawerDescription>{step.description}</DrawerDescription>
          </div>
          <DrawerClose
            render={
              <IconButton label="Close new portfolio" variant="ghost">
                <X aria-hidden />
              </IconButton>
            }
          />
        </div>
        {step.id === "name" ? (
          <PortfolioSetupFields
            name={name}
            onNameChange={setName}
            currency={currency}
            onCurrencyChange={setCurrency}
          />
        ) : (
          <div className="ds-portfolio-setup-sheet__fields">
            {step.id === "asset" ? (
              <FormField label="First asset">
                <Input
                  value={assetQuery}
                  placeholder="Search by name or ticker"
                  onChange={(event) => setAssetQuery(event.target.value)}
                />
              </FormField>
            ) : (
              <FormField label="Target weight">
                <Input
                  value={targetWeight}
                  inputMode="decimal"
                  placeholder="e.g. 60"
                  onChange={(event) => setTargetWeight(event.target.value)}
                />
              </FormField>
            )}
          </div>
        )}
        <Button
          className="ds-portfolio-setup-sheet__continue"
          disabled={!canContinue}
          onClick={() => {
            if (step.status === "upcoming") return;
            if (step.id === "name") {
              onContinue?.({
                stepId: "name",
                name: name.trim(),
                currency,
              });
            } else if (step.id === "asset") {
              onContinue?.({ stepId: "asset", assetQuery: assetQuery.trim() });
            } else {
              onContinue?.({
                stepId: "weights",
                targetWeight: targetWeight.trim(),
              });
            }
          }}
        >
          Continue
        </Button>
      </DrawerContent>
    </Drawer>
  );
}

export interface PortfolioSetupPageProps {
  onBack?: () => void;
  backHref?: string;
  onStepSelect?: (step: PortfolioSetupStep) => void;
  onContinue?: (details: PortfolioSetupContinueDetails) => void;
}

/** Full first-use setup page combining the inline step and resumable checklist. */
export function PortfolioSetupPage({
  onBack,
  backHref = "/allocate",
  onStepSelect,
  onContinue,
}: PortfolioSetupPageProps) {
  const [steps, setSteps] = React.useState<PortfolioSetupStep[]>(() =>
    portfolioSetupSteps.map((step) => ({ ...step }))
  );
  const [open, setOpen] = React.useState(false);
  const [activeStep, setActiveStep] = React.useState(
    () => portfolioSetupSteps[0]
  );
  const [setupValues, setSetupValues] = React.useState({
    name: "",
    currency: "",
    assetQuery: "",
    targetWeight: "",
  });

  const handleContinue = (details: PortfolioSetupContinueDetails) => {
    setSetupValues((currentValues) =>
      details.stepId === "name"
        ? { ...currentValues, name: details.name, currency: details.currency }
        : details.stepId === "asset"
          ? { ...currentValues, assetQuery: details.assetQuery }
          : { ...currentValues, targetWeight: details.targetWeight }
    );
    const currentIndex = steps.findIndex((step) => step.id === activeStep.id);
    const nextStep = steps[currentIndex + 1];

    setSteps((currentSteps) =>
      currentSteps.map((step, index) => ({
        ...step,
        status:
          index <= currentIndex
            ? "complete"
            : index === currentIndex + 1
              ? "current"
              : "upcoming",
      }))
    );
    onContinue?.(details);

    if (nextStep) {
      setActiveStep(nextStep);
      setOpen(true);
    } else {
      setOpen(false);
    }
  };

  return (
    <div
      className="ds-portfolio-setup-page"
      data-sheet-open={open || undefined}
    >
      <a
        className="ds-portfolio-setup-page__back"
        href={backHref}
        onClick={(event) => {
          if (onBack) {
            event.preventDefault();
            onBack();
          }
        }}
      >
        <ArrowLeft aria-hidden />
        My portfolios
      </a>
      {steps[0]?.status === "complete" ? null : (
        <Card className="ds-portfolio-setup-page__form-card">
          <CardContent>
            <span className="ds-portfolio-setup-sheet__step-count">
              Step 1 of 3
            </span>
            <h1>New portfolio</h1>
            <p>Give your portfolio a name and choose your base currency.</p>
            <PortfolioSetupForm
              initialName={setupValues.name}
              initialCurrency={setupValues.currency}
              onContinue={handleContinue}
            />
          </CardContent>
        </Card>
      )}
      <PortfolioSetupChecklist
        steps={steps}
        onStepSelect={(step) => {
          if (step.status === "upcoming") return;
          setActiveStep(step);
          onStepSelect?.(step);
          setOpen(true);
        }}
      />
      <PortfolioSetupSheet
        open={open}
        onOpenChange={setOpen}
        step={activeStep}
        steps={steps}
        initialName={setupValues.name}
        initialCurrency={setupValues.currency}
        initialAssetQuery={setupValues.assetQuery}
        initialTargetWeight={setupValues.targetWeight}
        onContinue={handleContinue}
      />
    </div>
  );
}
