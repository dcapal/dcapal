import * as React from "react";
import { ArrowLeft, Check, FilePlus2, PieChart, Plus } from "lucide-react";
import {
  AssetTickerChip,
  Button,
  Input,
  InputWithSuffix,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui";
import { FormField } from "./form-field";
import {
  popularSetupAssets,
  searchSetupAssets,
  setupAssetOptions,
  setupWizardSteps,
  checkSetupWeight,
  type SetupAssetOption,
} from "./portfolio-setup-wizard";
import {
  usePortfolioSetup,
  type PortfolioSetup,
  type PortfolioSetupContinueDetails,
} from "./use-portfolio-setup";
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

const currencyOptions = [
  { value: "EUR", label: "EUR · Euro" },
  { value: "USD", label: "USD · US dollar" },
  { value: "GBP", label: "GBP · Pound sterling" },
  { value: "CHF", label: "CHF · Swiss franc" },
] as const;

/** One selectable asset: a ticker chip, its name and a check when chosen. */
function SetupAssetPick({
  asset,
  selected,
  onSelect,
}: {
  asset: SetupAssetOption;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      className="ds-setup-wizard__pick"
      aria-pressed={selected}
      onClick={onSelect}
    >
      <AssetTickerChip ticker={asset.ticker} />
      <span className="ds-setup-wizard__pick-name">{asset.name}</span>
      {selected ? <Check aria-hidden /> : null}
    </button>
  );
}

function SetupNameFields({ setup }: { setup: PortfolioSetup }) {
  return (
    <div className="ds-setup-wizard__fields ds-setup-wizard__fields--name">
      <FormField label="Portfolio name">
        <Input
          value={setup.values.name}
          placeholder="e.g. Long-term Growth"
          autoComplete="off"
          onChange={(event) => setup.update({ name: event.target.value })}
        />
      </FormField>
      <FormField label="Base currency">
        <Select
          value={setup.values.currency}
          onValueChange={(value) => setup.update({ currency: value ?? "" })}
        >
          <SelectTrigger aria-label="Base currency">
            <SelectValue>
              {(value) =>
                currencyOptions.find((option) => option.value === value)
                  ?.label ?? "Select currency"
              }
            </SelectValue>
          </SelectTrigger>
          <SelectContent>
            {currencyOptions.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </FormField>
    </div>
  );
}

function SetupAssetFields({ setup }: { setup: PortfolioSetup }) {
  const query = setup.values.assetQuery;
  const searching = query.trim().length > 0;
  const results = searchSetupAssets(query);
  const choose = (asset: SetupAssetOption) =>
    setup.update({ assetId: asset.id });
  return (
    <div className="ds-setup-wizard__fields">
      <FormField label="Search an asset">
        <Input
          type="search"
          value={query}
          placeholder="Ticker or name"
          autoComplete="off"
          onChange={(event) => setup.update({ assetQuery: event.target.value })}
        />
      </FormField>
      {searching ? (
        <section
          className="ds-setup-wizard__picks"
          aria-label="Asset search results"
        >
          {results.length > 0 ? (
            results.map((asset) => (
              <SetupAssetPick
                key={asset.id}
                asset={asset}
                selected={asset.id === setup.values.assetId}
                onSelect={() => choose(asset)}
              />
            ))
          ) : (
            <p className="ds-setup-wizard__empty">
              No asset matches &ldquo;{query.trim()}&rdquo;.
            </p>
          )}
        </section>
      ) : (
        <section
          className="ds-setup-wizard__picks-group"
          aria-labelledby="setup-wizard-popular"
        >
          <p id="setup-wizard-popular" className="ds-setup-wizard__hint">
            Or start from a popular pick
          </p>
          <div className="ds-setup-wizard__picks">
            {popularSetupAssets.map((asset) => (
              <SetupAssetPick
                key={asset.id}
                asset={asset}
                selected={asset.id === setup.values.assetId}
                onSelect={() => choose(asset)}
              />
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

function SetupWeightFields({ setup }: { setup: PortfolioSetup }) {
  const strategic = setup.allocationMode === "strategic";
  const asset =
    setupAssetOptions.find((option) => option.id === setup.values.assetId) ??
    setupAssetOptions[0];
  const check = checkSetupWeight(setup.values.weight, setup.allocationMode);
  return (
    <div className="ds-setup-wizard__fields">
      <div className="ds-setup-wizard__chosen">
        <AssetTickerChip ticker={asset.ticker} />
        <span className="ds-setup-wizard__pick-name">{asset.name}</span>
      </div>
      <FormField
        label={strategic ? "Asset class weight" : "Target weight"}
        description={
          strategic
            ? "Share of the whole portfolio for this asset's class. You can set the asset's share of the class later from its edit dialog."
            : "Share of the whole portfolio you want this asset to be."
        }
        error={check.valid ? undefined : check.message}
      >
        <InputWithSuffix
          value={setup.values.weight}
          inputMode="decimal"
          suffix="%"
          placeholder="e.g. 100"
          onChange={(event) => setup.update({ weight: event.target.value })}
        />
      </FormField>
      <p className="ds-setup-wizard__valid" role="status">
        {check.valid ? check.message : ""}
      </p>
    </div>
  );
}

export interface PortfolioSetupWizardProps {
  /** State and moves of the wizard, from `usePortfolioSetup`. */
  setup: PortfolioSetup;
  className?: string;
}

/**
 * The first-use "Set up your portfolio" card. A segmented progress bar names
 * the steps (reached steps can be reopened), the left side says what the step
 * is for and the right side holds its real form with Back and Continue.
 * Finishing the last step removes the card, so the page shows only the
 * portfolio itself. On phones it is the same pattern in a single column.
 */
export function PortfolioSetupWizard({
  setup,
  className,
}: PortfolioSetupWizardProps) {
  const titleId = React.useId();
  const headingRef = React.useRef<HTMLHeadingElement>(null);
  const firstRender = React.useRef(true);
  const steps = setupWizardSteps(setup.allocationMode);
  const copy = steps[setup.stepIndex];

  // Moves focus to the new step so keyboard and screen-reader users follow it.
  React.useEffect(() => {
    if (firstRender.current) {
      firstRender.current = false;
      return;
    }
    headingRef.current?.focus({ preventScroll: true });
  }, [setup.stepIndex]);

  if (setup.done) return null;

  return (
    <section
      className={cn("ds-setup-wizard", className)}
      aria-labelledby={titleId}
    >
      <div className="ds-setup-wizard__top">
        <h2 id={titleId}>Set up your portfolio</h2>
        <span>
          Step {setup.stepIndex + 1} of {steps.length} · about a minute
        </span>
      </div>
      <ol className="ds-setup-wizard__steps" aria-label="Setup steps">
        {steps.map((step, index) => {
          const state =
            index === setup.stepIndex
              ? "current"
              : index < setup.completedCount
                ? "complete"
                : "upcoming";
          return (
            <li key={step.id}>
              <button
                type="button"
                className="ds-setup-wizard__segment"
                data-state={state}
                aria-current={state === "current" ? "step" : undefined}
                disabled={index !== setup.stepIndex && !setup.canVisit(index)}
                onClick={() => setup.goTo(index)}
              >
                <span aria-hidden className="ds-setup-wizard__bar" />
                <span className="ds-setup-wizard__segment-label">
                  <span aria-hidden className="ds-setup-wizard__dot">
                    {state === "complete" ? <Check /> : index + 1}
                  </span>
                  <span>{step.label}</span>
                  {state === "complete" ? (
                    <span className="ds-sr-only">, completed</span>
                  ) : null}
                </span>
              </button>
            </li>
          );
        })}
      </ol>
      <div className="ds-setup-wizard__body">
        <div className="ds-setup-wizard__intro">
          <h3 ref={headingRef} tabIndex={-1}>
            {copy.heading}
          </h3>
          <p>{copy.reason}</p>
        </div>
        <form
          className="ds-setup-wizard__form"
          noValidate
          onSubmit={(event) => {
            event.preventDefault();
            setup.next();
          }}
        >
          {setup.stepId === "name" ? <SetupNameFields setup={setup} /> : null}
          {setup.stepId === "asset" ? <SetupAssetFields setup={setup} /> : null}
          {setup.stepId === "weights" ? (
            <SetupWeightFields setup={setup} />
          ) : null}
          <div className="ds-setup-wizard__actions">
            {setup.stepIndex > 0 ? (
              <Button variant="ghost" type="button" onClick={setup.back}>
                Back
              </Button>
            ) : (
              <span />
            )}
            <Button type="submit" disabled={!setup.canContinue}>
              {setup.isLastStep ? "Finish setup" : "Continue"}
            </Button>
          </div>
        </form>
      </div>
    </section>
  );
}

export interface PortfolioSetupPageProps {
  onBack?: () => void;
  backHref?: string;
  /** Called when the investor reopens a step from the progress bar. */
  onStepSelect?: (stepId: PortfolioSetupStepId) => void;
  /** Called with each step's values as the investor continues. */
  onContinue?: (details: PortfolioSetupContinueDetails) => void;
}

/** Full-page "New portfolio" setup: the three-step wizard under a heading. */
export function PortfolioSetupPage({
  onBack,
  backHref = "/allocate",
  onStepSelect,
  onContinue,
}: PortfolioSetupPageProps) {
  const setup = usePortfolioSetup({ onContinue, onStepSelect });
  return (
    <div className="ds-portfolio-setup-page">
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
      <h1 className="ds-portfolio-setup-page__title">New portfolio</h1>
      <PortfolioSetupWizard setup={setup} />
      {setup.done ? (
        <p role="status" className="ds-portfolio-setup-page__done">
          {setup.values.name.trim()} is ready.
        </p>
      ) : null}
    </div>
  );
}
