import * as React from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  CircleAlert,
  Info,
  Pencil,
  Search,
  Settings,
  X,
} from "lucide-react";
import { DriftDiagnostic } from "./drift-diagnostic";
import { MetricCard } from "./metric-card";
import { PageHeader } from "./page-header";
import { PortfolioAssetCollection } from "./portfolio-asset-collection";
import {
  PortfolioSetupChecklist,
  PortfolioSetupSheet,
  portfolioSetupSteps,
  type PortfolioSetupContinueDetails,
  type PortfolioSetupStep,
} from "./portfolio-setup";
import {
  PortfolioWorkspace,
  type PortfolioWorkspaceNavItem,
} from "./portfolio-workspace";
import {
  TransactionFeeConfiguration,
  TransactionFeeConfigurationForm,
} from "./transaction-fee-configuration";
import {
  Alert,
  AssetTickerChip,
  Badge,
  Button,
  Card,
  CardContent,
  Dialog,
  DialogCloseButton,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerTitle,
  EmptyState,
  Field,
  FieldGroup,
  FieldLabel,
  IconButton,
  Input,
  InputWithSuffix,
  RadioCardGroup,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Separator,
  Skeleton,
} from "../ui";
import {
  assetClassLabels,
  formatAssetClass,
  formatCurrency,
  portfolioAssets,
  type AssetClass,
  type PortfolioAsset,
} from "../../fixtures/portfolio";

type FeePolicy = "portfolio" | "zero" | "fixed" | "variable";
type FeeValues = {
  maximumImpact: string;
  amount: string;
  feeRate: string;
  minFee: string;
  maxFee: string;
};

const defaultFeeValues: FeeValues = {
  maximumImpact: "0.5",
  amount: "2.50",
  feeRate: "0.25",
  minFee: "1.00",
  maxFee: "25.00",
};

/** Shows the selected fee rule and the deterministic values behind it. */
function FeeConfigurationSummary({
  policy,
  values,
  portfolioValues = defaultFeeValues,
  onEdit,
  buttonRef,
}: {
  policy: FeePolicy;
  values: FeeValues;
  portfolioValues?: FeeValues;
  onEdit: () => void;
  buttonRef?: React.Ref<HTMLButtonElement>;
}) {
  const label = transactionFeeLabels[policy];
  const inheritedValues = portfolioValues;
  const details =
    policy === "portfolio"
      ? `Maximum impact ${inheritedValues.maximumImpact}% · Amount €${inheritedValues.amount}`
      : policy === "fixed"
        ? `Maximum impact ${values.maximumImpact}% · Amount €${values.amount}`
        : policy === "variable"
          ? `Maximum impact ${values.maximumImpact}% · Fee percentage ${values.feeRate}% · Min fee €${values.minFee} · Max fee €${values.maxFee}`
          : "No transaction fees are charged";
  return (
    <div className="ds-fee-summary">
      <div className="ds-fee-summary__copy">
        <strong>{label}</strong>
        <span>{details}</span>
      </div>
      <IconButton
        ref={buttonRef}
        label="Edit transaction fees"
        variant="ghost"
        size="sm"
        onClick={onEdit}
      >
        <Pencil size={15} aria-hidden />
      </IconButton>
    </div>
  );
}

const compositionPortfolioOptions = [
  {
    label: "Core portfolio",
    value: "core",
    currentValue: "€124,680.75",
    dayChange: "+€612.18 (+0.49%)",
    cashBalance: "€3,842.15",
    assets: portfolioAssets,
  },
  {
    label: "Long-term growth",
    value: "growth",
    currentValue: "€52,340.12",
    dayChange: "+€342.56 (+0.69%)",
    cashBalance: "€1,240.20",
    assets: portfolioAssets.slice(0, 3),
  },
  {
    label: "Emergency fund",
    value: "emergency",
    currentValue: "€15,250.00",
    dayChange: "+€12.50 (+0.08%)",
    cashBalance: "€1,850.00",
    assets: portfolioAssets.slice(4),
  },
] as const;

type CompositionPortfolio = (typeof compositionPortfolioOptions)[number];

function primaryPortfolioAsset(
  portfolio: CompositionPortfolio
): PortfolioAsset {
  return portfolio.assets[0] ?? portfolioAssets[0];
}

function portfolioDailyChangeRate(portfolio: CompositionPortfolio) {
  const match = portfolio.dayChange.match(/\(([+-]?\d+(?:\.\d+)?)%\)/);
  return match ? Number(match[1]) / 100 : 0;
}

function firstHoldingDailyChange(
  asset: PortfolioAsset,
  portfolio: CompositionPortfolio
) {
  return (
    Math.round(asset.value * portfolioDailyChangeRate(portfolio) * 100) / 100
  );
}

const mobileViewportQuery = "(max-width: 48rem)";

function getNarrowViewportSnapshot() {
  return (
    typeof window !== "undefined" &&
    window.matchMedia(mobileViewportQuery).matches
  );
}

function subscribeToViewport(callback: () => void) {
  if (typeof window === "undefined") return () => undefined;
  const mediaQuery = window.matchMedia(mobileViewportQuery);
  if (mediaQuery.addEventListener)
    mediaQuery.addEventListener("change", callback);
  else mediaQuery.addListener(callback);
  return () => {
    if (mediaQuery.removeEventListener)
      mediaQuery.removeEventListener("change", callback);
    else mediaQuery.removeListener(callback);
  };
}

function useNarrowViewport() {
  return React.useSyncExternalStore(
    subscribeToViewport,
    getNarrowViewportSnapshot,
    () => false
  );
}

/** Selects one of the sample portfolios shared by the journey compositions. */
export function useCompositionPortfolio() {
  const [portfolioId, setPortfolioId] = React.useState("core");
  const portfolio =
    compositionPortfolioOptions.find(
      (option) => option.value === portfolioId
    ) ?? compositionPortfolioOptions[0];

  return { portfolio, setPortfolioId };
}

/** Shared selected-portfolio shell for every journey composition. */
export function PortfolioCompositionShell({
  children,
  portfolio,
  onPortfolioChange,
  selectedItem = "overview",
  onSelectItem,
}: {
  children: React.ReactNode;
  portfolio: CompositionPortfolio;
  onPortfolioChange: (value: string) => void;
  selectedItem?: string;
  onSelectItem?: (item: PortfolioWorkspaceNavItem) => boolean | void;
}) {
  const [activeItem, setActiveItem] = React.useState(selectedItem);

  React.useEffect(() => {
    setActiveItem(selectedItem);
  }, [selectedItem]);

  const handleSelectItem = onSelectItem
    ? (item: PortfolioWorkspaceNavItem) => {
        setActiveItem(item.id);
        return onSelectItem(item);
      }
    : undefined;

  return (
    <PortfolioWorkspace
      portfolioName={portfolio.label}
      portfolioOptions={compositionPortfolioOptions}
      onPortfolioChange={onPortfolioChange}
      selectedItem={activeItem}
      onSelectItem={handleSelectItem}
    >
      {children}
    </PortfolioWorkspace>
  );
}

const driftBandLabels = {
  default: "Default · ±5 pp",
  tight: "Tight · ±2 pp",
  wide: "Wide · ±10 pp",
} as const;

const transactionFeeLabels = {
  portfolio: "Default",
  zero: "Zero",
  fixed: "Fixed",
  variable: "Variable",
} as const;

export type PortfolioFirstUseStage = "empty" | "name" | "asset" | "weights";

function setupStepsForStage(stage: PortfolioFirstUseStage) {
  const completedCount =
    stage === "name" ? 1 : stage === "asset" ? 2 : stage === "weights" ? 3 : 0;
  return portfolioSetupSteps.map((step, index) => ({
    ...step,
    status:
      index < completedCount
        ? ("complete" as const)
        : index === completedCount &&
            completedCount < portfolioSetupSteps.length
          ? ("current" as const)
          : ("upcoming" as const),
  }));
}

function CloseDialogButton() {
  return <DialogCloseButton />;
}

export interface PortfolioOverviewCompositionProps {
  state?: "empty" | "populated";
  /** Selected portfolio can be shared with an overlay composition. */
  portfolioId?: string;
  /** Updates the selected portfolio when the selector changes. */
  onPortfolioChange?: (value: string) => void;
  /** Deterministic first-use stage used by the storyboard journey boards. */
  firstUseStage?: PortfolioFirstUseStage;
  /** Optional desktop-only actions rendered beside the page heading. */
  headerActions?: React.ReactNode;
  onCreatePortfolio?: () => void;
  onAddAsset?: () => void;
  /** Opens the Invest journey. */
  onInvest?: () => void;
  onEditAsset?: (assetId: string) => void;
  onAssetMenu?: (assetId: string) => void;
  onSetupStepSelect?: (step: PortfolioSetupStep) => void;
  onSetupContinue?: (details: PortfolioSetupContinueDetails) => void;
}

/** The overview hub shown in the portfolio journey boards. */
export function PortfolioOverviewComposition({
  state = "populated",
  portfolioId,
  onPortfolioChange,
  firstUseStage = "empty",
  headerActions,
  onCreatePortfolio = () => undefined,
  onAddAsset = () => undefined,
  onInvest = () => undefined,
  onEditAsset = () => undefined,
  onAssetMenu = () => undefined,
  onSetupStepSelect,
  onSetupContinue,
}: PortfolioOverviewCompositionProps) {
  const [uncontrolledPortfolioId, setUncontrolledPortfolioId] =
    React.useState("core");
  const selectedPortfolioId = portfolioId ?? uncontrolledPortfolioId;
  const portfolio =
    compositionPortfolioOptions.find(
      (option) => option.value === selectedPortfolioId
    ) ?? compositionPortfolioOptions[0];
  const handlePortfolioChange = (nextPortfolioId: string) => {
    setUncontrolledPortfolioId(nextPortfolioId);
    onPortfolioChange?.(nextPortfolioId);
  };
  const [setupSteps, setSetupSteps] = React.useState<PortfolioSetupStep[]>(() =>
    setupStepsForStage(firstUseStage)
  );
  const [journeyStage, setJourneyStage] =
    React.useState<PortfolioFirstUseStage>(firstUseStage);
  const [setupValues, setSetupValues] = React.useState({
    name: "",
    currency: "",
    assetQuery: "",
    targetWeight: "",
  });
  const [setupSheetOpen, setSetupSheetOpen] = React.useState(false);
  const [activeSetupStep, setActiveSetupStep] = React.useState(
    () => portfolioSetupSteps[0]
  );
  React.useEffect(() => {
    setJourneyStage(firstUseStage);
    setSetupSteps(setupStepsForStage(firstUseStage));
    setActiveSetupStep(
      setupStepsForStage(firstUseStage).find(
        (step) => step.status === "current"
      ) ?? portfolioSetupSteps[portfolioSetupSteps.length - 1]
    );
  }, [firstUseStage]);
  const handleSetupStepSelect = (step: PortfolioSetupStep) => {
    if (step.status === "upcoming") return;
    setActiveSetupStep(
      setupSteps.find((candidate) => candidate.id === step.id) ?? step
    );
    setSetupSheetOpen(true);
    onSetupStepSelect?.(step);
    if (step.id === "name") onCreatePortfolio();
    if (step.id === "asset") onAddAsset();
    if (step.id === "weights") onInvest();
  };
  const handleSetupContinue = (details: PortfolioSetupContinueDetails) => {
    setSetupValues((currentValues) =>
      details.stepId === "name"
        ? { ...currentValues, name: details.name, currency: details.currency }
        : details.stepId === "asset"
          ? { ...currentValues, assetQuery: details.assetQuery }
          : { ...currentValues, targetWeight: details.targetWeight }
    );
    const currentIndex = setupSteps.findIndex(
      (step) => step.id === activeSetupStep.id
    );
    const nextStep = setupSteps[currentIndex + 1];

    const nextStage =
      details.stepId === "name"
        ? "name"
        : details.stepId === "asset"
          ? "asset"
          : "weights";
    setJourneyStage(nextStage);

    setSetupSteps((currentSteps) =>
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

    if (nextStep) {
      setActiveSetupStep(nextStep);
      setSetupSheetOpen(true);
    } else {
      setSetupSheetOpen(false);
    }
    onSetupContinue?.(details);
  };

  const isFirstUse = state === "empty";
  const hasFirstAsset =
    !isFirstUse || journeyStage === "asset" || journeyStage === "weights";
  const overviewAssets = isFirstUse
    ? hasFirstAsset
      ? portfolio.assets.slice(0, 1)
      : []
    : portfolio.assets;
  const driftAsset = primaryPortfolioAsset(portfolio);
  const firstHolding = overviewAssets[0];
  const firstHoldingChange = firstHolding
    ? firstHoldingDailyChange(firstHolding, portfolio)
    : 0;
  const firstHoldingChangePercent = portfolioDailyChangeRate(portfolio) * 100;
  const firstHoldingChangeLabel = firstHolding
    ? `${firstHoldingChange >= 0 ? "+" : "−"}${formatCurrency(
        Math.abs(firstHoldingChange),
        firstHolding.currency
      )} (${firstHoldingChangePercent >= 0 ? "+" : ""}${firstHoldingChangePercent.toFixed(2)}%)`
    : "—";

  return (
    <PortfolioCompositionShell
      portfolio={portfolio}
      onPortfolioChange={handlePortfolioChange}
      selectedItem="overview"
      onSelectItem={(item) => {
        if (item.id === "invest") {
          onInvest();
          return true;
        }
        return false;
      }}
    >
      <PageHeader
        className="ds-page-header--overview"
        title={portfolio.label}
        description={
          <span className="ds-overview-meta">
            As of 2 Aug 2026 <span aria-hidden>·</span> <Badge>Simple</Badge>
          </span>
        }
        actions={
          <div className="ds-overview-header-actions">
            <div className="ds-overview-header-actions__desktop">
              {headerActions ?? (
                <>
                  <Button variant="ghost" onClick={onAddAsset}>
                    Add asset
                  </Button>
                  <Button variant="ghost" onClick={onInvest}>
                    Invest
                  </Button>
                </>
              )}
            </div>
            <div className="ds-overview-header-actions__mobile">
              <Button variant="primary" onClick={onAddAsset}>
                Add asset
              </Button>
            </div>
            <IconButton label="Open portfolio settings" variant="ghost">
              <Settings size={18} aria-hidden />
            </IconButton>
          </div>
        }
      />
      <div className="ds-overview-metrics">
        <div className="ds-metric-grid">
          <MetricCard
            density="compact"
            metric={{
              label: "Current value",
              value: hasFirstAsset
                ? isFirstUse && firstHolding
                  ? formatCurrency(firstHolding.value, firstHolding.currency)
                  : portfolio.currentValue
                : "—",
              context: hasFirstAsset
                ? isFirstUse
                  ? "Value of your first holding"
                  : "As of 2 Aug 2026"
                : "Add an asset to see your portfolio value",
            }}
            placeholder={!hasFirstAsset}
          />
          <MetricCard
            density="compact"
            metric={{
              label: "Day change",
              value: hasFirstAsset
                ? isFirstUse
                  ? firstHoldingChangeLabel
                  : portfolio.dayChange
                : "—",
              context: hasFirstAsset
                ? isFirstUse
                  ? "Since previous close · fixture estimate"
                  : "Since previous close"
                : "Available after your first asset",
              trend: hasFirstAsset
                ? isFirstUse
                  ? firstHoldingChange < 0
                    ? "negative"
                    : firstHoldingChange > 0
                      ? "positive"
                      : "neutral"
                  : "positive"
                : undefined,
            }}
            placeholder={!hasFirstAsset}
          />
          {!isFirstUse ? (
            <MetricCard
              density="compact"
              metric={{
                label: "Cash balance",
                value: portfolio.cashBalance,
                context: "Available to invest",
              }}
            />
          ) : null}
        </div>
        <Card className="ds-overview-mobile-summary">
          <CardContent>
            <span className="ds-metric-card__label">Current value</span>
            <strong className="ds-overview-mobile-summary__value">
              {hasFirstAsset
                ? isFirstUse && firstHolding
                  ? formatCurrency(firstHolding.value, firstHolding.currency)
                  : portfolio.currentValue
                : "—"}
            </strong>
            <span className="ds-overview-mobile-summary__change">
              {hasFirstAsset
                ? isFirstUse
                  ? firstHoldingChangeLabel
                  : portfolio.dayChange
                : "Add an asset to see change"}
            </span>
            {!isFirstUse ? (
              <div className="ds-overview-mobile-summary__cash">
                <span>Cash balance</span>
                <strong>{portfolio.cashBalance}</strong>
              </div>
            ) : null}
          </CardContent>
        </Card>
      </div>
      {state === "empty" ? (
        <>
          <Card className="ds-overview-empty-checklist">
            <CardContent>
              <PortfolioSetupChecklist
                steps={setupSteps}
                onStepSelect={handleSetupStepSelect}
              />
            </CardContent>
          </Card>
          <PortfolioSetupSheet
            open={setupSheetOpen}
            onOpenChange={setSetupSheetOpen}
            step={activeSetupStep}
            steps={setupSteps}
            initialName={setupValues.name}
            initialCurrency={setupValues.currency}
            initialAssetQuery={setupValues.assetQuery}
            initialTargetWeight={setupValues.targetWeight}
            onContinue={handleSetupContinue}
          />
          {hasFirstAsset ? (
            <PortfolioAssetCollection
              assets={overviewAssets}
              presentation="overview"
              onEdit={(asset) => onEditAsset(asset.id)}
              onMenu={(asset) => onAssetMenu(asset.id)}
            />
          ) : null}
        </>
      ) : (
        <>
          <DriftDiagnostic
            assetName={assetClassLabels[driftAsset.assetClass]}
            driftPp={driftAsset.driftPp}
            status={
              driftAsset.driftPp > 1
                ? "over"
                : driftAsset.driftPp < -1
                  ? "under"
                  : "within"
            }
            onReview={() => undefined}
          />
          <PortfolioAssetCollection
            assets={overviewAssets}
            presentation="overview"
            onEdit={(asset) => onEditAsset(asset.id)}
            onMenu={(asset) => onAssetMenu(asset.id)}
          />
        </>
      )}
    </PortfolioCompositionShell>
  );
}

export type AddAssetState =
  | "entry"
  | "loading"
  | "search-results"
  | "empty"
  | "error"
  | "unpriced"
  | "edit";

export interface AddAssetCompositionProps {
  state?: AddAssetState;
  /** Opened by default only for focused interactive state stories. */
  open?: boolean;
}

const searchResults = [
  portfolioAssets[0],
  portfolioAssets[1],
  portfolioAssets[2],
];

const strategicAssetClasses: ReadonlyArray<{
  key: AssetClass;
  target: number;
}> = [
  { key: "equities", target: 60 },
  { key: "bonds", target: 25 },
  { key: "cash", target: 10 },
  { key: "crypto", target: 5 },
  { key: "commodities", target: 0 },
  { key: "other", target: 0 },
];

function SearchResultCard({
  asset,
  selected = false,
  onSelect,
}: {
  asset: (typeof portfolioAssets)[number];
  selected?: boolean;
  onSelect?: () => void;
}) {
  return (
    <Button
      type="button"
      variant={selected ? "secondary" : "outline"}
      className="ds-search-result"
      pressed={selected}
      onClick={onSelect}
    >
      <span className="ds-search-result__copy">
        <AssetTickerChip ticker={asset.ticker} />
        <span>{asset.name}</span>
        <small>{formatAssetClass(asset)}</small>
      </span>
      <span className="ds-search-result__price">
        {formatCurrency(asset.price, asset.currency)}
      </span>
    </Button>
  );
}

/** Add-asset search and configuration stages used by the Storybook journey. */
export function AddAssetComposition({
  state = "entry",
  open = state !== "entry",
}: AddAssetCompositionProps) {
  const [portfolioId, setPortfolioId] = React.useState("core");
  const portfolio =
    compositionPortfolioOptions.find(
      (option) => option.value === portfolioId
    ) ?? compositionPortfolioOptions[0];
  const [searchState, setSearchState] = React.useState<AddAssetState>(state);
  const [overlayOpen, setOverlayOpen] = React.useState(open);
  const isMobile = useNarrowViewport();
  const [searchValue, setSearchValue] = React.useState(
    state === "search-results" ? "VW" : ""
  );
  const [selectedAssetId, setSelectedAssetId] = React.useState<string | null>(
    state === "edit" ? (portfolio.assets[0]?.id ?? portfolioAssets[0].id) : null
  );
  const [feePolicy, setFeePolicy] = React.useState<FeePolicy>("portfolio");
  const [feeValues, setFeeValues] = React.useState<FeeValues>(defaultFeeValues);
  const [feeOverlayOpen, setFeeOverlayOpen] = React.useState(false);
  const feeTriggerRef = React.useRef<HTMLButtonElement>(null);
  const searchInputRef = React.useRef<HTMLInputElement>(null);
  const portfolioAsset =
    portfolio.assets.find((asset) => asset.id === selectedAssetId) ??
    portfolioAssets.find((asset) => asset.id === selectedAssetId) ??
    primaryPortfolioAsset(portfolio);
  const isEdit = searchState === "edit" || selectedAssetId !== null;
  const showResults = searchState === "search-results";

  React.useEffect(() => {
    setSearchState(state);
    setSearchValue(state === "search-results" ? "VW" : "");
    setSelectedAssetId(
      state === "edit"
        ? (portfolio.assets[0]?.id ?? portfolioAssets[0].id)
        : null
    );
  }, [state]);

  React.useEffect(() => {
    setOverlayOpen(open);
  }, [open]);

  const handleOverlayOpenChange = (nextOpen: boolean) => {
    setOverlayOpen(nextOpen);
    if (!nextOpen) {
      setSelectedAssetId(null);
      setSearchState("entry");
      setSearchValue("");
    }
  };

  const openAddAsset = () => {
    setSelectedAssetId(null);
    setSearchState("entry");
    setSearchValue("");
    setOverlayOpen(true);
  };

  const handleFeePolicyChange = (nextPolicy: FeePolicy) => {
    setFeePolicy(nextPolicy);
  };

  const handleSearch = (value: string) => {
    setSearchValue(value);
    setSearchState(value.trim() ? "search-results" : "entry");
    setSelectedAssetId(null);
  };

  const handleSelectResult = (assetId: string) => {
    setSelectedAssetId(assetId);
    setSearchState("edit");
  };

  const searchPane = (
    <section
      className="ds-add-asset-overlay__search"
      aria-label="Search assets"
    >
      <div className="ds-add-asset-page__search-heading">
        <h2>Find an asset</h2>
        <p>Search by name or ticker, then review it before adding.</p>
      </div>
      <Field>
        <FieldLabel htmlFor="asset-search">Search by name or ticker</FieldLabel>
        <div className="ds-input-with-icon">
          <Search size={16} aria-hidden />
          <Input
            ref={searchInputRef}
            id="asset-search"
            value={searchValue}
            onChange={(event) => handleSearch(event.target.value)}
            placeholder="Search ETFs and more"
          />
        </div>
      </Field>
      {searchState === "loading" ? (
        <div
          aria-label="Searching assets"
          role="status"
          className="ds-story-stack"
        >
          <Skeleton />
          <Skeleton />
          <Skeleton />
        </div>
      ) : null}
      {showResults ? (
        <div className="ds-search-results" aria-label="Asset search results">
          {searchResults.map((asset) => (
            <SearchResultCard
              key={asset.id}
              asset={asset}
              selected={asset.id === selectedAssetId}
              onSelect={() => handleSelectResult(asset.id)}
            />
          ))}
        </div>
      ) : null}
      {searchState === "empty" ? (
        <EmptyState
          title="No assets found"
          description="Try a different name or ticker."
        />
      ) : null}
      {searchState === "error" ? (
        <Alert variant="destructive">
          <CircleAlert size={18} aria-hidden />
          <div>
            <strong>Search could not be completed</strong>
            <p>Try again or check your connection.</p>
          </div>
        </Alert>
      ) : null}
      {searchState === "unpriced" ? (
        <Card className="ds-search-result-card">
          <CardContent>
            <AssetTickerChip ticker="VWCE.DE" />
            <p className="ds-description ds-allocation-note">
              Vanguard FTSE All-World UCITS ETF
            </p>
            <Badge variant="warning">Price unavailable</Badge>
          </CardContent>
        </Card>
      ) : null}
    </section>
  );

  const assetPreview = (
    <section
      className="ds-add-asset-overlay__details"
      aria-label={isEdit ? "Asset details" : "Asset preview"}
    >
      {isEdit ? (
        <>
          <div className="ds-add-asset-overlay__asset-heading">
            <div>
              <p className="ds-eyebrow">Selected asset</p>
              <h2>{portfolioAsset.name}</h2>
              <p>{formatAssetClass(portfolioAsset)}</p>
            </div>
            <AssetTickerChip ticker={portfolioAsset.ticker} />
          </div>
          <Separator />
          <FieldGroup>
            <Field orientation="inline">
              <FieldLabel htmlFor="asset-quantity">Quantity</FieldLabel>
              <Input
                id="asset-quantity"
                defaultValue={String(portfolioAsset.quantity)}
                inputMode="decimal"
              />
            </Field>
            <Field orientation="inline">
              <FieldLabel htmlFor="asset-basis">Basis cost / PMC</FieldLabel>
              <InputWithSuffix
                id="asset-basis"
                defaultValue={
                  portfolioAsset.averageCostBasis?.toFixed(2) ?? "0.00"
                }
                inputMode="decimal"
                suffix={portfolioAsset.currency ?? "EUR"}
              />
            </Field>
            <Field orientation="inline">
              <FieldLabel htmlFor="asset-target">Target weight</FieldLabel>
              <InputWithSuffix
                id="asset-target"
                defaultValue={String(portfolioAsset.targetWeight)}
                inputMode="decimal"
                suffix="%"
              />
            </Field>
            <Field>
              <FieldLabel>Drift band</FieldLabel>
              <Select defaultValue="default">
                <SelectTrigger>
                  <SelectValue>
                    {(value) =>
                      driftBandLabels[value as keyof typeof driftBandLabels] ??
                      String(value ?? "")
                    }
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="default">Default · ±5 pp</SelectItem>
                  <SelectItem value="tight">Tight · ±2 pp</SelectItem>
                  <SelectItem value="wide">Wide · ±10 pp</SelectItem>
                </SelectContent>
              </Select>
            </Field>
            <Field>
              <FieldLabel>Transaction fees</FieldLabel>
              <FeeConfigurationSummary
                policy={feePolicy}
                values={feeValues}
                portfolioValues={defaultFeeValues}
                onEdit={() => setFeeOverlayOpen(true)}
                buttonRef={feeTriggerRef}
              />
            </Field>
          </FieldGroup>
          <div className="ds-add-asset-overlay__actions">
            <Button
              variant="outline"
              onClick={() => {
                setSelectedAssetId(null);
                setSearchState("search-results");
              }}
            >
              Choose another asset
            </Button>
            <Button
              onClick={() => {
                setSelectedAssetId(null);
                setSearchState("entry");
                setSearchValue("");
                setOverlayOpen(false);
              }}
            >
              Add asset
            </Button>
          </div>
        </>
      ) : (
        <EmptyState
          title="Select an asset"
          description="Choose a search result to review its price and set your target weight."
        />
      )}
    </section>
  );

  return (
    <>
      <PortfolioOverviewComposition
        state="populated"
        portfolioId={portfolioId}
        onPortfolioChange={setPortfolioId}
        onAddAsset={openAddAsset}
      />
      {isMobile ? (
        <Drawer open={overlayOpen} onOpenChange={handleOverlayOpenChange}>
          <DrawerContent className="ds-add-asset-drawer">
            <div className="ds-add-asset-overlay__header">
              <div>
                <DrawerTitle>Add an asset</DrawerTitle>
                <DrawerDescription>
                  Search for an investment to add to {portfolio.label}.
                </DrawerDescription>
              </div>
              <DrawerClose
                render={
                  <IconButton label="Close add asset" variant="ghost">
                    <X aria-hidden />
                  </IconButton>
                }
              />
            </div>
            <div className="ds-add-asset-overlay__body">
              {searchPane}
              {isEdit ? assetPreview : null}
            </div>
          </DrawerContent>
        </Drawer>
      ) : (
        <Dialog open={overlayOpen} onOpenChange={handleOverlayOpenChange}>
          <DialogContent className="ds-add-asset-dialog">
            <DialogHeader>
              <div className="ds-add-asset-overlay__header">
                <div>
                  <DialogTitle>Add an asset</DialogTitle>
                  <DialogDescription>
                    Search for an investment to add to {portfolio.label}.
                  </DialogDescription>
                </div>
                <DialogCloseButton />
              </div>
            </DialogHeader>
            <div className="ds-add-asset-overlay__body">
              {searchPane}
              {isEdit ? assetPreview : null}
            </div>
          </DialogContent>
        </Dialog>
      )}
      <TransactionFeeConfiguration
        open={feeOverlayOpen}
        policy={feePolicy}
        onOpenChange={setFeeOverlayOpen}
        onPolicyChange={handleFeePolicyChange}
        onSave={(values) => {
          setFeePolicy(values.policy);
          setFeeValues({
            maximumImpact: values.maximumImpact,
            amount: values.amount,
            feeRate: values.feeRate,
            minFee: values.minFee,
            maxFee: values.maxFee,
          });
        }}
        includePortfolioDefault
        portfolioDefaults={defaultFeeValues}
        finalFocus={feeTriggerRef}
      />
    </>
  );
}

export type EditAssetStage = "details" | "fees" | "remove";

export interface EditRemoveAssetCompositionProps {
  state?: "editing" | "saved" | "error" | "remove-confirmation";
  stage?: EditAssetStage;
  open?: boolean;
}

function RemoveAssetDialog({
  asset,
  defaultOpen = false,
  open,
  onOpenChange,
  includeTrigger = false,
}: {
  asset: PortfolioAsset;
  defaultOpen?: boolean;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  includeTrigger?: boolean;
}) {
  return (
    <Dialog open={open} defaultOpen={defaultOpen} onOpenChange={onOpenChange}>
      {includeTrigger ? (
        <DialogTrigger
          render={<Button variant="destructive">Remove asset</Button>}
        />
      ) : null}
      <DialogContent className="ds-remove-dialog">
        <DialogHeader>
          <div className="ds-remove-dialog__body">
            <div className="ds-remove-dialog__icon" aria-hidden>
              <CircleAlert size={28} />
            </div>
            <DialogTitle>
              Remove {asset.ticker} from this portfolio?
            </DialogTitle>
            <DialogDescription>
              This asset has a target weight of {asset.targetWeight.toFixed(1)}%
              and a current holding value of{" "}
              {formatCurrency(asset.value, asset.currency)}. All holding data,
              allocation, and historical performance will be removed.
            </DialogDescription>
          </div>
          <CloseDialogButton />
        </DialogHeader>
        <DialogFooter>
          <Button variant="destructive">Remove asset</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/** Edit, fee configuration, and destructive removal stages for one asset. */
export function EditRemoveAssetComposition({
  state = "editing",
  stage = state === "remove-confirmation" ? "remove" : "details",
  open = false,
}: EditRemoveAssetCompositionProps) {
  const { portfolio, setPortfolioId } = useCompositionPortfolio();
  const portfolioAsset = primaryPortfolioAsset(portfolio);
  const showRemove = state === "remove-confirmation" || stage === "remove";
  const [saved, setSaved] = React.useState(state === "saved");
  const [editOpen, setEditOpen] = React.useState(stage === "details" && open);
  const [removeOpen, setRemoveOpen] = React.useState(
    state === "remove-confirmation" || stage === "remove"
  );
  const [feePolicy, setFeePolicy] = React.useState<FeePolicy>("variable");
  const [feeValues, setFeeValues] = React.useState<FeeValues>(defaultFeeValues);
  const [feeOverlayOpen, setFeeOverlayOpen] = React.useState(stage === "fees");
  const feeTriggerRef = React.useRef<HTMLButtonElement>(null);
  const editOriginRef = React.useRef<HTMLElement | null>(null);
  const isMobile = useNarrowViewport();
  React.useEffect(() => {
    setEditOpen(stage === "details" && open);
    setFeeOverlayOpen(stage === "fees");
    setRemoveOpen(state === "remove-confirmation" || stage === "remove");
  }, [open, stage, state]);

  const handleEditOpenChange = (nextOpen: boolean) => {
    setEditOpen(nextOpen);
    if (!nextOpen) {
      window.requestAnimationFrame(() => editOriginRef.current?.focus());
    }
  };

  const openEditDialog = () => {
    editOriginRef.current =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    setEditOpen(true);
  };
  const editIdentity = (
    <div className="ds-asset-dialog-identity">
      <span
        className={`ds-asset-card__dot ds-asset-card__dot--${portfolioAsset.assetClass}`}
        aria-hidden
      />
      <div>
        <strong>{portfolioAsset.name}</strong>
        <span>{formatAssetClass(portfolioAsset)}</span>
      </div>
      <AssetTickerChip ticker={portfolioAsset.ticker} />
    </div>
  );
  const editFields = (
    <>
      {editIdentity}
      <Separator />
      <FieldGroup>
        <Field orientation="inline">
          <FieldLabel htmlFor="edit-quantity">Quantity</FieldLabel>
          <Input
            id="edit-quantity"
            defaultValue={String(portfolioAsset.quantity)}
          />
        </Field>
        <Field orientation="inline">
          <FieldLabel htmlFor="edit-basis">Basis cost / PMC</FieldLabel>
          <InputWithSuffix
            id="edit-basis"
            defaultValue={portfolioAsset.averageCostBasis?.toFixed(2) ?? "0.00"}
            suffix={portfolioAsset.currency ?? "EUR"}
          />
        </Field>
        <Field orientation="inline">
          <FieldLabel htmlFor="edit-target">Target weight</FieldLabel>
          <InputWithSuffix
            id="edit-target"
            defaultValue={String(portfolioAsset.targetWeight)}
            suffix="%"
          />
        </Field>
        <Field orientation="inline">
          <FieldLabel>Current weight</FieldLabel>
          <span className="ds-value-readonly">
            {portfolioAsset.currentWeight.toFixed(1)}%
          </span>
        </Field>
        <Field orientation="inline">
          <FieldLabel>Drift</FieldLabel>
          <span
            className={`ds-value-readonly ds-status-${
              portfolioAsset.driftPp > 1
                ? "warning"
                : portfolioAsset.driftPp < -1
                  ? "primary"
                  : "success"
            }`}
          >
            {portfolioAsset.driftPp > 0 ? "+" : ""}
            {portfolioAsset.driftPp.toFixed(1)} pp
          </span>
        </Field>
        <Field>
          <FieldLabel>Transaction fees</FieldLabel>
          <FeeConfigurationSummary
            policy={feePolicy}
            values={feeValues}
            portfolioValues={defaultFeeValues}
            onEdit={() => setFeeOverlayOpen(true)}
            buttonRef={feeTriggerRef}
          />
        </Field>
      </FieldGroup>
    </>
  );
  const editActions = (
    <>
      <Button
        variant="destructive"
        type="button"
        onClick={() => setRemoveOpen(true)}
      >
        Remove asset
      </Button>
      <Button
        onClick={() => {
          setSaved(true);
          handleEditOpenChange(false);
        }}
      >
        Save changes
      </Button>
    </>
  );
  return (
    <PortfolioCompositionShell
      portfolio={portfolio}
      onPortfolioChange={setPortfolioId}
      selectedItem="overview"
    >
      <PageHeader title={portfolio.label} />
      {state === "saved" || saved ? (
        <Alert variant="success" className="ds-edit-remove-asset__save-alert">
          <Check size={18} aria-hidden />
          <strong>Asset changes saved</strong>
        </Alert>
      ) : null}
      {state === "error" ? (
        <Alert
          variant="destructive"
          className="ds-edit-remove-asset__save-alert"
        >
          <CircleAlert size={18} aria-hidden />
          <strong>Could not save asset changes</strong>
        </Alert>
      ) : null}
      <PortfolioAssetCollection
        assets={[portfolioAsset]}
        onEdit={openEditDialog}
      />
      {isMobile ? (
        <Drawer open={editOpen} onOpenChange={handleEditOpenChange}>
          <DrawerContent className="ds-edit-asset-sheet">
            <div className="ds-edit-asset-sheet__header">
              <div>
                <DrawerTitle>Edit {portfolioAsset.ticker}</DrawerTitle>
                <DrawerDescription>
                  Review and edit asset details.
                </DrawerDescription>
              </div>
              <DrawerClose
                render={
                  <IconButton label="Close edit asset" variant="ghost">
                    <X aria-hidden />
                  </IconButton>
                }
              />
            </div>
            {editFields}
            <DrawerFooter className="ds-dialog-footer--stacked-mobile">
              {editActions}
            </DrawerFooter>
          </DrawerContent>
        </Drawer>
      ) : (
        <Dialog open={editOpen} onOpenChange={handleEditOpenChange}>
          <DialogContent className="ds-edit-asset-dialog">
            <DialogHeader>
              <div>
                <DialogTitle>Edit {portfolioAsset.ticker}</DialogTitle>
                <DialogDescription>
                  Review and edit asset details.
                </DialogDescription>
              </div>
              <CloseDialogButton />
            </DialogHeader>
            {editFields}
            <DialogFooter className="ds-dialog-footer--stacked-mobile">
              {editActions}
            </DialogFooter>
          </DialogContent>
        </Dialog>
      )}
      <TransactionFeeConfiguration
        open={feeOverlayOpen}
        policy={feePolicy}
        onOpenChange={setFeeOverlayOpen}
        onPolicyChange={setFeePolicy}
        onSave={(values) => {
          setFeePolicy(values.policy);
          setFeeValues({
            maximumImpact: values.maximumImpact,
            amount: values.amount,
            feeRate: values.feeRate,
            minFee: values.minFee,
            maxFee: values.maxFee,
          });
        }}
        includePortfolioDefault
        portfolioDefaults={defaultFeeValues}
        finalFocus={feeTriggerRef}
      />
      <RemoveAssetDialog
        asset={portfolioAsset}
        defaultOpen={showRemove}
        open={removeOpen}
        onOpenChange={setRemoveOpen}
      />
    </PortfolioCompositionShell>
  );
}

export interface PortfolioSettingsCompositionProps {
  saveState?: "idle" | "success" | "error";
  view?: "settings" | "strategic";
}

/** Settings and strategic allocation configuration stages. */
export function PortfolioSettingsComposition({
  saveState = "idle",
  view = "settings",
}: PortfolioSettingsCompositionProps) {
  const { portfolio, setPortfolioId } = useCompositionPortfolio();
  const [method, setMethod] = React.useState<"simple" | "strategic">("simple");
  const [currentView, setCurrentView] = React.useState(view);
  const [localSaveState, setLocalSaveState] = React.useState<"success" | null>(
    null
  );
  const [settingsFeePolicy, setSettingsFeePolicy] = React.useState<
    "zero" | "fixed" | "variable"
  >("zero");
  const [settingsFeeValues, setSettingsFeeValues] =
    React.useState<FeeValues>(defaultFeeValues);
  React.useEffect(() => setCurrentView(view), [view]);
  React.useEffect(() => setLocalSaveState(null), [view]);
  const strategic = currentView === "strategic";
  const visibleSaveState = localSaveState ?? saveState;

  return (
    <PortfolioCompositionShell
      portfolio={portfolio}
      onPortfolioChange={setPortfolioId}
      selectedItem="settings"
    >
      <PageHeader
        className="ds-settings-page__header"
        title={strategic ? "Strategic allocation" : "Portfolio settings"}
        description={
          strategic
            ? "Set asset-class targets and acceptable drift ranges."
            : `Configure how ${portfolio.label} is built and how allocation suggestions work.`
        }
        actions={
          strategic ? (
            <Button
              variant="outline"
              onClick={() => {
                setCurrentView("settings");
                setMethod("simple");
              }}
            >
              <ArrowLeft size={16} aria-hidden /> Portfolio settings
            </Button>
          ) : null
        }
      />
      {visibleSaveState === "success" ? (
        <Alert variant="success">
          <Check size={18} aria-hidden />
          <strong>Portfolio settings saved</strong>
        </Alert>
      ) : null}
      {visibleSaveState === "error" ? (
        <Alert variant="destructive">
          <CircleAlert size={18} aria-hidden />
          <strong>Could not save portfolio settings</strong>
        </Alert>
      ) : null}
      <Card
        className={`ds-settings-page ${strategic ? "ds-settings-page--strategic" : ""}`}
      >
        <CardContent>
          {strategic ? (
            <div className="ds-strategic-allocation">
              <div className="ds-strategic-allocation__header">
                <span>Asset class</span>
                <span>Target weight</span>
                <span>Drift band</span>
              </div>
              {strategicAssetClasses.map(({ key: assetClass, target }) => (
                <div className="ds-strategic-allocation__row" key={assetClass}>
                  <strong>
                    <span
                      className={`ds-asset-card__dot ds-asset-card__dot--${assetClass}`}
                      aria-hidden
                    />
                    {assetClassLabels[assetClass]}
                  </strong>
                  <div className="ds-strategic-allocation__field">
                    <span className="ds-strategic-allocation__field-label">
                      Target weight
                    </span>
                    <InputWithSuffix
                      aria-label={`${assetClassLabels[assetClass]} target weight`}
                      defaultValue={`${target}`}
                      suffix="%"
                    />
                  </div>
                  <div className="ds-strategic-allocation__field">
                    <span className="ds-strategic-allocation__field-label">
                      Drift band
                    </span>
                    <Select defaultValue="±5 pp">
                      <SelectTrigger
                        aria-label={`${assetClassLabels[assetClass]} drift band`}
                      >
                        <SelectValue>
                          {(value) => String(value ?? "")}
                        </SelectValue>
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="±2 pp">±2 pp</SelectItem>
                        <SelectItem value="±5 pp">±5 pp</SelectItem>
                        <SelectItem value="±10 pp">±10 pp</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                </div>
              ))}
              <Alert variant="info">
                <Info size={18} aria-hidden />
                <span>
                  Asset weights inside each class are configured in the asset
                  editor.
                </span>
              </Alert>
              <Button variant="link" onClick={() => setCurrentView("settings")}>
                View guidance <ArrowRight size={14} aria-hidden />
              </Button>
            </div>
          ) : (
            <FieldGroup>
              <div className="ds-field">
                <RadioCardGroup
                  label="Allocation method"
                  value={method}
                  onValueChange={(value) =>
                    setMethod(value as "simple" | "strategic")
                  }
                  options={[
                    {
                      value: "simple",
                      title: "Simple",
                      description:
                        "Use absolute target weights for each asset.",
                    },
                    {
                      value: "strategic",
                      title: "Strategic",
                      description:
                        "Use asset-class targets with relative weights inside each class.",
                    },
                  ]}
                />
                {method === "strategic" ? (
                  <Button
                    variant="outline"
                    onClick={() => setCurrentView("strategic")}
                  >
                    Review strategic allocation
                  </Button>
                ) : null}
              </div>
              <Field>
                <FieldLabel>Portfolio-level transaction fees</FieldLabel>
                <TransactionFeeConfigurationForm
                  policy={settingsFeePolicy}
                  onPolicyChange={(value) =>
                    setSettingsFeePolicy(value as "zero" | "fixed" | "variable")
                  }
                  {...settingsFeeValues}
                  onMaximumImpactChange={(maximumImpact) =>
                    setSettingsFeeValues((current) => ({
                      ...current,
                      maximumImpact,
                    }))
                  }
                  onAmountChange={(amount) =>
                    setSettingsFeeValues((current) => ({ ...current, amount }))
                  }
                  onFeeRateChange={(feeRate) =>
                    setSettingsFeeValues((current) => ({ ...current, feeRate }))
                  }
                  onMinFeeChange={(minFee) =>
                    setSettingsFeeValues((current) => ({ ...current, minFee }))
                  }
                  onMaxFeeChange={(maxFee) =>
                    setSettingsFeeValues((current) => ({ ...current, maxFee }))
                  }
                  idPrefix="portfolio-fee"
                />
              </Field>
            </FieldGroup>
          )}
          <div className="ds-settings-page__footer">
            <Button onClick={() => setLocalSaveState("success")}>
              {strategic ? "Save strategic allocation" : "Save changes"}
            </Button>
          </div>
        </CardContent>
      </Card>
    </PortfolioCompositionShell>
  );
}
