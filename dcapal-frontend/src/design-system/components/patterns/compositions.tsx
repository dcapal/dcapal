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
  ShieldCheck,
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
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
  Checkbox,
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
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
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

type AllocationPosition = {
  asset: PortfolioAsset;
  currentValue: number;
  currentWeight: number;
  targetWeight: number;
};

function parseMoney(value: string) {
  return Number(value.replace(/[^\d.]/g, "")) || 0;
}

/** Uses the displayed holdings and cash balance as the source for review math. */
function allocationPositionsForPortfolio(portfolio: CompositionPortfolio): {
  positions: AllocationPosition[];
  portfolioValue: number;
} {
  const nonCashAssets = portfolio.assets.filter(
    (asset) => asset.assetClass !== "cash"
  );
  const cashAsset = portfolio.assets.find(
    (asset) => asset.assetClass === "cash"
  );
  const cashValue = parseMoney(portfolio.cashBalance);
  const targetWeightForCash =
    nonCashAssets.length === 0
      ? 100
      : (cashAsset?.targetWeight ??
        Math.max(
          0,
          100 -
            nonCashAssets.reduce(
              (total, asset) => total + asset.targetWeight,
              0
            )
        ));
  const positions = nonCashAssets.map((asset) => ({
    asset,
    currentValue: asset.value,
    currentWeight: 0,
    targetWeight: asset.targetWeight,
  }));

  if (cashValue > 0 || cashAsset) {
    positions.push({
      asset: cashAsset ?? {
        id: "cash-balance",
        name: "Cash",
        ticker: "EUR",
        assetClass: "cash",
        quantity: cashValue,
        price: 1,
        value: cashValue,
        targetWeight: targetWeightForCash,
        currentWeight: 0,
        driftPp: 0,
        currency: "EUR",
        region: "EUR",
      },
      currentValue: cashValue,
      currentWeight: 0,
      targetWeight: targetWeightForCash,
    });
  }

  const portfolioValue = positions.reduce(
    (total, position) => total + position.currentValue,
    0
  );
  const targetWeightTotal = positions.reduce(
    (total, position) => total + position.targetWeight,
    0
  );

  return {
    portfolioValue,
    positions: positions.map((position) => ({
      ...position,
      currentWeight:
        portfolioValue > 0 ? (position.currentValue / portfolioValue) * 100 : 0,
      targetWeight:
        targetWeightTotal > 0
          ? (position.targetWeight / targetWeightTotal) * 100
          : 0,
    })),
  };
}

/** Keeps rebalance trades, fees, and cash on one displayed portfolio snapshot. */
function calculateRebalanceReview(
  portfolio: CompositionPortfolio,
  budget: number,
  { buyOnly, useFractional }: { buyOnly: boolean; useFractional: boolean }
) {
  const { positions, portfolioValue } =
    allocationPositionsForPortfolio(portfolio);
  const rowsForNetBudget = (netBudget: number) => {
    const targetValueTotal = portfolioValue + netBudget;
    const underweight = positions.map((position) => ({
      ...position,
      deficit:
        position.asset.assetClass === "cash"
          ? 0
          : Math.max(
              0,
              (targetValueTotal * position.targetWeight) / 100 -
                position.currentValue
            ),
    }));
    const totalDeficit = underweight.reduce(
      (total, position) => total + position.deficit,
      0
    );
    const proposedTrades = underweight.map((position) => {
      const difference =
        (targetValueTotal * position.targetWeight) / 100 -
        position.currentValue;
      const suggestedAmount = buyOnly
        ? totalDeficit > 0
          ? Math.min(
              netBudget * (position.deficit / totalDeficit),
              position.deficit
            )
          : 0
        : Math.abs(difference);
      const price = Math.max(position.asset.price, Number.EPSILON);
      const precision = useFractional ? 100 : 1;
      const requestedQuantity =
        Math.floor((suggestedAmount / price + Number.EPSILON) * precision) /
        precision;
      const availableQuantity =
        !buyOnly && difference < 0
          ? position.currentValue / price
          : requestedQuantity;
      const quantity = Math.min(requestedQuantity, availableQuantity);
      const amount = Math.round(quantity * price * 100) / 100;
      const signedAmount =
        position.asset.assetClass === "cash" || difference === 0
          ? 0
          : buyOnly
            ? amount
            : Math.sign(difference) * amount;

      return { ...position, amount, signedAmount };
    });
    const netTradeAmount = proposedTrades.reduce(
      (total, trade) => total + trade.signedAmount,
      0
    );

    return proposedTrades.map((trade) => {
      const proposedValue =
        trade.asset.assetClass === "cash"
          ? trade.currentValue + netBudget - netTradeAmount
          : trade.currentValue + trade.signedAmount;

      return {
        ...trade,
        proposedWeight:
          targetValueTotal > 0 ? (proposedValue / targetValueTotal) * 100 : 0,
      };
    });
  };

  let estimatedCommission = 0;
  let rows = rowsForNetBudget(budget);
  for (let attempt = 0; attempt <= positions.length; attempt += 1) {
    const nextCommission =
      rows.filter(
        ({ asset, amount }) => asset.assetClass !== "cash" && amount >= 0.01
      ).length * 1;
    if (nextCommission === estimatedCommission) break;
    estimatedCommission = nextCommission;
    rows = rowsForNetBudget(Math.max(0, budget - estimatedCommission));
  }

  const trades = rows.filter(
    ({ asset, amount }) => asset.assetClass !== "cash" && amount >= 0.01
  );
  const investedAmount = trades.reduce(
    (total, trade) => total + (trade.signedAmount > 0 ? trade.signedAmount : 0),
    0
  );
  const netTradeAmount = trades.reduce(
    (total, trade) => total + trade.signedAmount,
    0
  );
  const cashAsset = positions.find(
    (position) => position.asset.assetClass === "cash"
  )?.asset;
  const startingCash =
    positions.find((position) => position.asset.assetClass === "cash")
      ?.currentValue ?? 0;
  const unallocatedBudget = Math.max(
    0,
    budget - investedAmount - estimatedCommission
  );
  const cashAfter =
    startingCash + budget - netTradeAmount - estimatedCommission;

  return {
    rows,
    trades,
    estimatedCommission,
    cashAsset,
    cashAfter,
    unallocatedBudget,
    buyOnly,
  };
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

function useCompositionPortfolio() {
  const [portfolioId, setPortfolioId] = React.useState("core");
  const portfolio =
    compositionPortfolioOptions.find(
      (option) => option.value === portfolioId
    ) ?? compositionPortfolioOptions[0];

  return { portfolio, setPortfolioId };
}

/** Shared selected-portfolio shell for every journey composition. */
function PortfolioCompositionShell({
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
  onAllocate?: () => void;
  onRebalance?: () => void;
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
  onAllocate = () => undefined,
  onRebalance = () => undefined,
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
    if (step.id === "weights") onAllocate();
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
        if (item.id === "allocate") {
          onAllocate();
          return true;
        }
        if (item.id === "rebalance") {
          onRebalance();
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
                  <Button variant="ghost" onClick={onAllocate}>
                    Allocate
                  </Button>
                  <Button variant="ghost" onClick={onRebalance}>
                    Rebalance
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

export interface AllocateCompositionProps {
  state?: "options" | "result";
  mode?: "allocate" | "rebalance";
}

function AllocationOption({
  checked,
  defaultChecked,
  children,
  description,
  onCheckedChange,
}: {
  checked?: boolean;
  defaultChecked?: boolean;
  children: React.ReactNode;
  description: string;
  onCheckedChange?: (checked: boolean) => void;
}) {
  return (
    <label className="ds-story-option ds-allocation-option">
      <Checkbox
        checked={checked}
        defaultChecked={defaultChecked}
        onCheckedChange={(value) => onCheckedChange?.(value === true)}
      />
      <span>
        <strong>{children}</strong>
        <small>{description}</small>
      </span>
    </label>
  );
}

/** Allocation and rebalance options plus the deterministic review result. */
export function AllocateComposition({
  state = "options",
  mode = "allocate",
}: AllocateCompositionProps) {
  const { portfolio, setPortfolioId } = useCompositionPortfolio();
  const [showResult, setShowResult] = React.useState(state === "result");
  const [isConfirmed, setIsConfirmed] = React.useState(false);
  const [budgetInput, setBudgetInput] = React.useState("1,000");
  const [buyOnly, setBuyOnly] = React.useState(true);
  const [useFractionalQuantities, setUseFractionalQuantities] =
    React.useState(false);
  React.useEffect(() => {
    setShowResult(state === "result");
    setIsConfirmed(false);
  }, [state]);
  React.useEffect(() => {
    setIsConfirmed(false);
  }, [portfolio.value]);
  React.useEffect(() => {
    setBuyOnly(true);
    setUseFractionalQuantities(false);
  }, [mode]);
  const isResult = showResult;
  const actionLabel = mode === "rebalance" ? "Rebalance" : "Allocate";
  const allocationBudget = Math.max(0, parseMoney(budgetInput));
  const allocationSnapshot = allocationPositionsForPortfolio(portfolio);
  const allocationAssetPosition =
    allocationSnapshot.positions.find(
      (position) =>
        position.asset.assetClass !== "cash" &&
        position.targetWeight > position.currentWeight
    ) ??
    allocationSnapshot.positions.find(
      (position) => position.asset.assetClass !== "cash"
    );
  const allocationAsset =
    allocationAssetPosition?.asset ?? primaryPortfolioAsset(portfolio);
  const allocationAssetValue =
    allocationAssetPosition?.currentValue ?? allocationAsset.value;
  const allocationCurrentWeight =
    allocationAssetPosition?.currentWeight ?? allocationAsset.currentWeight;
  const allocationPrice = Math.max(allocationAsset.price, Number.EPSILON);
  const allocationQuantityPrecision = useFractionalQuantities ? 100 : 1;
  const allocationQuantity =
    allocationAsset.assetClass === "cash"
      ? 0
      : Math.max(
          0,
          Math.floor(
            ((allocationBudget - 1) / allocationPrice + Number.EPSILON) *
              allocationQuantityPrecision
          ) / allocationQuantityPrecision
        );
  const allocationCommission = allocationQuantity > 0 ? 1 : 0;
  const allocationAmountValue =
    Math.round(allocationQuantity * allocationPrice * 100) / 100;
  const allocationRemainingCash =
    allocationBudget - allocationAmountValue - allocationCommission;
  const startingCashBalance =
    allocationSnapshot.positions.find(
      (position) => position.asset.assetClass === "cash"
    )?.currentValue ?? 0;
  const allocationCashAfter = startingCashBalance + allocationRemainingCash;
  const allocationAmount = formatCurrency(
    allocationAmountValue,
    allocationAsset.currency
  );
  const allocationWeightAfter =
    ((allocationAssetValue + allocationAmountValue) /
      (allocationSnapshot.portfolioValue +
        allocationBudget -
        allocationCommission)) *
    100;
  const allocationWeightChange =
    allocationWeightAfter - allocationCurrentWeight;
  const rebalanceReview = calculateRebalanceReview(
    portfolio,
    allocationBudget,
    {
      buyOnly,
      useFractional: useFractionalQuantities,
    }
  );
  const { rows: rebalanceRows, trades: tradeRows } = rebalanceReview;
  const isRebalance = mode === "rebalance";
  const compositionTitle = isRebalance
    ? "Rebalance to your target allocation"
    : "How much do you want to allocate?";
  const compositionDescription = isRebalance
    ? buyOnly
      ? "Use new savings to move underweight holdings closer to target. No assets will be sold."
      : "Move your holdings towards target using the new budget and, where needed, sales from overweight assets."
    : "Invest new savings across your target assets without selling existing holdings.";
  return (
    <PortfolioCompositionShell
      portfolio={portfolio}
      onPortfolioChange={setPortfolioId}
      selectedItem={mode === "rebalance" ? "rebalance" : "allocate"}
    >
      <div
        className={`ds-allocation-layout${isRebalance ? " ds-allocation-layout--rebalance" : ""}`}
      >
        <h1 className="ds-sr-only">Allocate and rebalance</h1>
        {!isResult ? (
          <>
            <header className="ds-allocation-page-heading">
              <div>
                <Badge variant="primary">{actionLabel}</Badge>
                <h2>{compositionTitle}</h2>
                <p>
                  {portfolio.label} <span aria-hidden>·</span>{" "}
                  {compositionDescription}
                </p>
              </div>
            </header>
            <div className="ds-allocation-options-grid">
              <Card className="ds-allocation-panel">
                <CardHeader>
                  <CardTitle>Investment budget</CardTitle>
                  <CardDescription>
                    {isRebalance
                      ? buyOnly
                        ? "Choose the amount to invest in holdings below target."
                        : "Choose the new budget to combine with proceeds from overweight positions."
                      : "Set the new savings amount to distribute across your target assets."}
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <FieldGroup>
                    <Field>
                      <FieldLabel htmlFor="allocation-amount">
                        Amount to invest
                      </FieldLabel>
                      <InputWithSuffix
                        id="allocation-amount"
                        inputMode="decimal"
                        value={budgetInput}
                        onChange={(event) => setBudgetInput(event.target.value)}
                        suffix="EUR"
                      />
                      <p className="ds-description ds-allocation-note">
                        <ShieldCheck size={14} aria-hidden /> Your target
                        allocation is already set
                      </p>
                    </Field>
                    <AllocationOption
                      defaultChecked
                      description={
                        isRebalance
                          ? "Prioritise holdings below target. Overweight assets stay untouched."
                          : "Prefer buy-only recommendations that preserve your existing holdings."
                      }
                    >
                      {isRebalance
                        ? "Apply target allocation"
                        : "Tax-efficient algorithm"}
                    </AllocationOption>
                    <AllocationOption
                      {...(isRebalance
                        ? {
                            checked: buyOnly,
                            onCheckedChange: setBuyOnly,
                          }
                        : { defaultChecked: true })}
                      description={
                        isRebalance
                          ? buyOnly
                            ? "Only place buy orders. No existing assets will be sold."
                            : "Allow sales from overweight holdings to help reach target weights."
                          : "Use available budget across your target assets; any remainder stays as cash."
                      }
                    >
                      {isRebalance ? "Buy only" : "Allocate the full budget"}
                    </AllocationOption>
                    <AllocationOption
                      checked={useFractionalQuantities}
                      onCheckedChange={setUseFractionalQuantities}
                      description="Use fractional quantities where the asset and provider support them."
                    >
                      Use fractional quantities
                    </AllocationOption>
                  </FieldGroup>
                </CardContent>
                <CardFooter className="ds-allocation-footer ds-allocation-options__footer">
                  <Button variant="ghost" size="sm">
                    <ArrowLeft size={16} aria-hidden /> Back
                  </Button>
                  <Button onClick={() => setShowResult(true)}>
                    {isRebalance ? "Review rebalance" : "Allocate budget"}{" "}
                    <ArrowRight size={16} aria-hidden />
                  </Button>
                </CardFooter>
              </Card>
              <Card className="ds-allocation-context-card" density="compact">
                <CardHeader>
                  <CardTitle>Portfolio target</CardTitle>
                  <CardDescription>
                    Current allocation compared with your plan.
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <div className="ds-allocation-target-list">
                    <div className="ds-allocation-target-list__header">
                      <span>Asset</span>
                      <span>Current</span>
                      <span>Target</span>
                    </div>
                    {allocationSnapshot.positions.map(
                      ({ asset, currentWeight, targetWeight }) => (
                        <div
                          className="ds-allocation-target-list__row"
                          key={asset.id}
                        >
                          <span>
                            <strong>{asset.ticker}</strong>
                            <small>{asset.name}</small>
                          </span>
                          <span>{currentWeight.toFixed(1)}%</span>
                          <strong>{targetWeight.toFixed(1)}%</strong>
                        </div>
                      )
                    )}
                  </div>
                  <p className="ds-description ds-allocation-context-note">
                    {isRebalance
                      ? buyOnly
                        ? "Review buy-only orders before confirming. Overweight holdings remain untouched."
                        : "Review purchases and sales before confirming. Proposed sales are capped at each displayed holding."
                      : "Your existing holdings stay in place. The new budget is invested only in target assets."}
                  </p>
                </CardContent>
              </Card>
            </div>
          </>
        ) : (
          <div
            className={
              isRebalance
                ? "ds-allocation-result ds-allocation-result--rebalance"
                : "ds-allocation-result"
            }
          >
            <Card className="ds-allocation-panel">
              <CardHeader>
                <Button
                  className="ds-allocation-mobile-back"
                  variant="ghost"
                  size="sm"
                  onClick={() => setShowResult(false)}
                >
                  <ArrowLeft size={16} aria-hidden /> Back to options
                </Button>
                <Badge variant="primary">{actionLabel} review</Badge>
                <CardTitle>
                  {isRebalance
                    ? "Review your rebalance"
                    : "Your allocation is ready"}
                </CardTitle>
                <CardDescription>
                  {portfolio.label} · Review the proposed changes before
                  updating your portfolio.
                </CardDescription>
              </CardHeader>
              {isRebalance ? (
                <CardContent className="ds-allocation-review-content">
                  <Card className="ds-review-card ds-rebalance-review">
                    <CardHeader>
                      <CardTitle>Weight comparison</CardTitle>
                      <CardDescription>
                        Proposed weights reflect your target allocation.
                      </CardDescription>
                    </CardHeader>
                    <CardContent>
                      <Table
                        aria-label="Rebalance weight comparison"
                        className="ds-table--rebalance-review"
                      >
                        <TableHeader>
                          <TableRow>
                            <TableHead>Asset</TableHead>
                            <TableHead numeric>Current</TableHead>
                            <TableHead numeric>Target</TableHead>
                            <TableHead numeric>Proposed</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {rebalanceRows.map(
                            ({
                              asset,
                              currentWeight,
                              targetWeight,
                              proposedWeight,
                            }) => (
                              <TableRow key={asset.id}>
                                <TableCell data-label="Asset">
                                  <span className="ds-table__asset-identity">
                                    <strong className="ds-table__asset-name">
                                      {asset.name}
                                    </strong>
                                    <span className="ds-table__asset-ticker">
                                      {asset.ticker}
                                    </span>
                                  </span>
                                </TableCell>
                                <TableCell numeric data-label="Current">
                                  {currentWeight.toFixed(1)}%
                                </TableCell>
                                <TableCell numeric data-label="Target">
                                  {targetWeight.toFixed(1)}%
                                </TableCell>
                                <TableCell numeric data-label="Proposed">
                                  {proposedWeight.toFixed(1)}%
                                </TableCell>
                              </TableRow>
                            )
                          )}
                        </TableBody>
                      </Table>
                    </CardContent>
                  </Card>
                  <div className="ds-allocation-review-side">
                    <Card className="ds-review-card ds-allocation-transaction-card">
                      <CardHeader>
                        <CardTitle>
                          {buyOnly
                            ? "Proposed purchases"
                            : "Proposed transactions"}
                        </CardTitle>
                        <CardDescription>
                          {buyOnly
                            ? "Buy-only orders move underweight holdings closer to target; overweight positions stay untouched."
                            : "Purchases and sales move holdings closer to target. Sales never exceed the displayed position value."}
                        </CardDescription>
                      </CardHeader>
                      <CardContent className="ds-story-stack ds-allocation-transaction-list">
                        {tradeRows.map(({ asset, amount, signedAmount }) => {
                          const quantity =
                            amount / Math.max(asset.price, Number.EPSILON);
                          const verb = signedAmount > 0 ? "Buy" : "Sell";
                          const quantityLabel = Number.isInteger(quantity)
                            ? quantity.toFixed(0)
                            : quantity
                                .toFixed(4)
                                .replace(/0+$/, "")
                                .replace(/\.$/, "");
                          return (
                            <section
                              className="ds-allocation-transaction"
                              key={asset.id}
                            >
                              <div className="ds-allocation-transaction__top">
                                <div className="ds-allocation-transaction__identity">
                                  <strong>{asset.ticker}</strong>
                                  <span>{asset.name}</span>
                                </div>
                                <Badge
                                  variant={
                                    verb === "Buy" ? "success" : "destructive"
                                  }
                                >
                                  {verb}
                                </Badge>
                              </div>
                              <dl className="ds-review-list ds-allocation-transaction__details">
                                <div className="ds-review-row">
                                  <dt>Quantity</dt>
                                  <dd>
                                    <strong>{quantityLabel} units</strong>
                                    <span>
                                      @{" "}
                                      {formatCurrency(
                                        asset.price,
                                        asset.currency
                                      )}
                                    </span>
                                  </dd>
                                </div>
                                <div className="ds-review-row">
                                  <dt>Amount</dt>
                                  <dd>
                                    <strong>
                                      {formatCurrency(amount, asset.currency)}
                                    </strong>
                                  </dd>
                                </div>
                                <div className="ds-review-row">
                                  <dt>Commission estimate</dt>
                                  <dd>
                                    <strong>€1.00</strong>
                                  </dd>
                                </div>
                                <div className="ds-review-row">
                                  <dt>Estimated first-year fee impact</dt>
                                  <dd>
                                    <strong>€0.18</strong>
                                  </dd>
                                </div>
                              </dl>
                            </section>
                          );
                        })}
                        {tradeRows.length === 0 ? (
                          <EmptyState
                            title="No trades to preview"
                            description="This portfolio has no investable holdings yet. The new budget remains in cash."
                          />
                        ) : null}
                      </CardContent>
                      <CardFooter
                        className={`ds-allocation-cash-summary${buyOnly ? "" : " ds-allocation-cash-summary--single"}`}
                      >
                        {buyOnly ? (
                          <>
                            <span>
                              <strong>Unallocated budget</strong>
                              <small>
                                {formatCurrency(allocationBudget)} budget ·{" "}
                                {formatCurrency(
                                  tradeRows.reduce(
                                    (total, trade) =>
                                      total + Math.max(0, trade.signedAmount),
                                    0
                                  )
                                )}{" "}
                                invested ·{" "}
                                {formatCurrency(
                                  rebalanceReview.estimatedCommission
                                )}{" "}
                                estimated commission
                              </small>
                            </span>
                            <strong className="ds-allocation-cash-summary__amount">
                              {formatCurrency(
                                rebalanceReview.unallocatedBudget
                              )}
                            </strong>
                          </>
                        ) : (
                          <span>
                            <strong>Cash after rebalance</strong>
                            <small>
                              Includes the new budget, sale proceeds, purchases,
                              and estimated commissions.
                            </small>
                          </span>
                        )}
                        <span className="ds-allocation-cash-summary__balance">
                          <small>
                            {buyOnly ? "Cash balance after" : "Available cash"}
                          </small>
                          <strong>
                            {formatCurrency(
                              rebalanceReview.cashAfter,
                              rebalanceReview.cashAsset?.currency ?? "EUR"
                            )}
                          </strong>
                        </span>
                      </CardFooter>
                    </Card>
                  </div>
                </CardContent>
              ) : (
                <CardContent className="ds-allocation-review-content ds-allocation-review-content--allocation">
                  <Card className="ds-review-card ds-allocation-transaction-card">
                    <CardHeader>
                      <CardTitle>
                        {allocationAsset.assetClass === "cash"
                          ? "Keep the budget in cash"
                          : `Buy ${allocationQuantity} ${allocationAsset.ticker}`}
                      </CardTitle>
                      <CardDescription>
                        {allocationAsset.name}
                        {allocationAsset.assetClass !== "cash"
                          ? ` · ${formatCurrency(allocationAsset.price, allocationAsset.currency)} per unit`
                          : ""}
                      </CardDescription>
                    </CardHeader>
                    <CardContent>
                      <dl className="ds-review-list">
                        <div className="ds-review-row">
                          <dt>Quantity</dt>
                          <dd>
                            <strong>{allocationQuantity} units</strong>
                            <span>
                              @{" "}
                              {formatCurrency(
                                allocationAsset.price,
                                allocationAsset.currency
                              )}
                            </span>
                          </dd>
                        </div>
                        <div className="ds-review-row">
                          <dt>Amount</dt>
                          <dd>
                            <strong>{allocationAmount}</strong>
                          </dd>
                        </div>
                        {allocationAsset.assetClass !== "cash" ? (
                          <div className="ds-review-row">
                            <dt>Weight after allocation</dt>
                            <dd>
                              <strong>
                                {allocationWeightAfter.toFixed(1)}%
                              </strong>
                              <span className="ds-status-success">
                                {allocationWeightChange >= 0 ? "+" : ""}
                                {allocationWeightChange.toFixed(1)} pp
                              </span>
                            </dd>
                          </div>
                        ) : null}
                        <div className="ds-review-row">
                          <dt>Commission estimate</dt>
                          <dd>
                            <strong>
                              {formatCurrency(
                                allocationCommission,
                                allocationAsset.currency
                              )}
                            </strong>
                          </dd>
                        </div>
                        <div className="ds-review-row">
                          <dt>Estimated first-year fee impact</dt>
                          <dd>
                            <strong>
                              {allocationAsset.assetClass === "cash"
                                ? "€0.00"
                                : "€0.18"}
                            </strong>
                          </dd>
                        </div>
                      </dl>
                    </CardContent>
                  </Card>
                  <Card className="ds-review-card ds-allocation-cash-card">
                    <CardHeader>
                      <CardTitle>
                        Unallocated cash{" "}
                        <Badge variant="success">
                          {formatCurrency(
                            allocationRemainingCash,
                            allocationAsset.currency
                          )}
                        </Badge>
                      </CardTitle>
                    </CardHeader>
                    <CardContent>
                      <p className="ds-description">
                        {allocationAsset.assetClass === "cash"
                          ? "No investable holdings are available yet, so the full budget remains in cash."
                          : "The budget left after whole units and the estimated commission stays available for next time."}
                      </p>
                      <p className="ds-description ds-allocation-cash-after">
                        Cash balance after allocation:{" "}
                        {formatCurrency(
                          allocationCashAfter,
                          allocationAsset.currency
                        )}
                      </p>
                    </CardContent>
                  </Card>
                </CardContent>
              )}
              {isConfirmed ? (
                <Alert variant="success" className="ds-allocation-confirmation">
                  <Check size={18} aria-hidden />
                  <strong>
                    {isRebalance ? "Rebalance confirmed" : "Allocation updated"}
                  </strong>
                </Alert>
              ) : null}
              <CardFooter className="ds-allocation-footer">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    setIsConfirmed(false);
                    setShowResult(false);
                  }}
                >
                  <ArrowLeft size={16} aria-hidden /> Back to options
                </Button>
                <Button
                  disabled={isConfirmed}
                  onClick={() => setIsConfirmed(true)}
                >
                  {isConfirmed ? (
                    <>
                      <Check size={16} aria-hidden />
                      {isRebalance
                        ? "Rebalance confirmed"
                        : "Portfolio updated"}
                    </>
                  ) : isRebalance ? (
                    "Confirm rebalance"
                  ) : (
                    "Update portfolio"
                  )}
                </Button>
              </CardFooter>
            </Card>
          </div>
        )}
      </div>
    </PortfolioCompositionShell>
  );
}
