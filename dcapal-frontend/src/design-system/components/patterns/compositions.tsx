import * as React from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  CircleAlert,
  Compass,
  Info,
  Pencil,
  Plus,
  Search,
  Settings,
  TrendingUp,
  X,
} from "lucide-react";
import { DriftDiagnostic } from "./drift-diagnostic";
import { EditAssetClassDialog } from "./edit-asset-class-dialog";
import { EditPortfolioAssetDialog } from "./edit-portfolio-asset-dialog";
import type { FloatingActionMenuAction } from "./floating-action-menu";
import type { AllocationMode } from "./invest-types";
import { SetClassWeightsDialog } from "./set-class-weights-dialog";
import { useReturnFocus } from "./use-return-focus";
import {
  buildPortfolioAssetSummary,
  driftState,
} from "./portfolio-asset-groups";
import { classTargetsFromAssets } from "./strategic-allocation";
import { useNarrowViewport } from "./use-narrow-viewport";
import { usePortfolioAllocation } from "./use-portfolio-allocation";
import type { ClassWeights } from "./strategic-allocation";
import { MetricCard } from "./metric-card";
import { PageHeader } from "./page-header";
import { PortfolioAssetCollection } from "./portfolio-asset-collection";
import { PortfolioSetupWizard } from "./portfolio-setup";
import {
  emptySetupWizardValues,
  setupAssetOptions,
  type SetupWizardStepId,
} from "./portfolio-setup-wizard";
import {
  usePortfolioSetup,
  type PortfolioSetupContinueDetails,
} from "./use-portfolio-setup";
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

/**
 * Owns the fee policy and values of one asset together with the nested fee
 * editor overlay, so every surface that edits an asset shares one behaviour.
 */
function useAssetFeeEditor(defaultOverlayOpen = false) {
  const [policy, setPolicy] = React.useState<FeePolicy>("variable");
  const [values, setValues] = React.useState<FeeValues>(defaultFeeValues);
  const [overlayOpen, setOverlayOpen] = React.useState(defaultOverlayOpen);
  const triggerRef = React.useRef<HTMLButtonElement>(null);
  const summary = (
    <FeeConfigurationSummary
      policy={policy}
      values={values}
      portfolioValues={defaultFeeValues}
      onEdit={() => setOverlayOpen(true)}
      buttonRef={triggerRef}
    />
  );
  const overlay = (
    <TransactionFeeConfiguration
      open={overlayOpen}
      policy={policy}
      onOpenChange={setOverlayOpen}
      onPolicyChange={setPolicy}
      onSave={(next) => {
        setPolicy(next.policy);
        setValues({
          maximumImpact: next.maximumImpact,
          amount: next.amount,
          feeRate: next.feeRate,
          minFee: next.minFee,
          maxFee: next.maxFee,
        });
      }}
      includePortfolioDefault
      portfolioDefaults={defaultFeeValues}
      finalFocus={triggerRef}
    />
  );
  return { summary, overlay, setOverlayOpen };
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
  pageActions,
}: {
  children: React.ReactNode;
  portfolio: CompositionPortfolio;
  onPortfolioChange: (value: string) => void;
  selectedItem?: string;
  onSelectItem?: (item: PortfolioWorkspaceNavItem) => boolean | void;
  /** Actions of the page, offered through the phone's floating actions menu. */
  pageActions?: ReadonlyArray<FloatingActionMenuAction>;
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
      pageActions={pageActions}
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

/** Where the setup wizard starts for a first-use stage; "weights" means it is finished. */
function setupStart(stage: PortfolioFirstUseStage) {
  const step = stage === "empty" ? 0 : stage === "name" ? 1 : 2;
  return {
    initialStep: step,
    initialDone: stage === "weights",
    initialValues: {
      ...emptySetupWizardValues,
      name: stage === "empty" ? "" : "Starter",
      assetId:
        stage === "asset" || stage === "weights"
          ? setupAssetOptions[0].id
          : null,
    },
  };
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
  /**
   * Optional desktop-only actions rendered beside the page heading. Phones
   * reach Add asset and Invest through the floating page-actions menu.
   */
  headerActions?: React.ReactNode;
  onAddAsset?: () => void;
  /** Opens the Invest journey. */
  onInvest?: () => void;
  onEditAsset?: (assetId: string) => void;
  /** The Portfolio's allocation method; it fixes the asset table's mode. */
  allocationMethod?: AllocationMode;
  /** Opens the Portfolio settings page. */
  onOpenSettings?: () => void;
  /** Opens the Edit {Class} dialog on first render (Canvas stories). */
  defaultEditClass?: AssetClass;
  /** Opens the asset edit dialog for this asset id on first render (Canvas stories). */
  defaultEditAssetId?: string;
  onAssetMenu?: (assetId: string) => void;
  onSetupStepSelect?: (stepId: SetupWizardStepId) => void;
  onSetupContinue?: (details: PortfolioSetupContinueDetails) => void;
}

/** The overview hub shown in the portfolio journey boards. */
export function PortfolioOverviewComposition({
  state = "populated",
  portfolioId,
  onPortfolioChange,
  firstUseStage = "empty",
  headerActions,
  onAddAsset = () => undefined,
  onInvest = () => undefined,
  onEditAsset = () => undefined,
  allocationMethod = "simple",
  onOpenSettings = () => undefined,
  defaultEditClass,
  defaultEditAssetId,
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
  const allocation = usePortfolioAllocation(portfolioAssets, allocationMethod);
  const { switchToSimple, switchToStrategic } = allocation;
  // Switching by prop uses the class targets the current weights imply, so
  // Simple edits carry over to Strategic.
  const classTargetsRef = React.useRef(
    classTargetsFromAssets(allocation.assets)
  );
  classTargetsRef.current = classTargetsFromAssets(allocation.assets);
  const methodRef = React.useRef(allocation.method);
  methodRef.current = allocation.method;
  React.useEffect(() => {
    // Follow the method prop when a story changes it after mount.
    if (methodRef.current === allocationMethod) return;
    if (allocationMethod === "simple") switchToSimple();
    else switchToStrategic(classTargetsRef.current);
  }, [allocationMethod, switchToSimple, switchToStrategic]);
  const strategic = allocation.method === "strategic";
  const portfolioAssetIds = portfolio.assets.map((asset) => asset.id);
  const portfolioAllocationAssets = allocation.assets.filter((asset) =>
    portfolioAssetIds.includes(asset.id)
  );
  const [editClass, setEditClass] = React.useState<AssetClass | null>(
    defaultEditClass ?? null
  );
  const [editAssetId, setEditAssetId] = React.useState(
    defaultEditAssetId ?? portfolioAssets[0].id
  );
  const [editAssetOpen, setEditAssetOpen] = React.useState(
    defaultEditAssetId !== undefined
  );
  const assetFees = useAssetFeeEditor();
  const classFocus = useReturnFocus();
  const assetFocus = useReturnFocus();
  const editedAsset =
    allocation.assets.find((asset) => asset.id === editAssetId) ??
    allocation.assets[0];
  const headerId = React.useId();
  const start = setupStart(firstUseStage);
  const setup = usePortfolioSetup({
    ...start,
    allocationMode: allocation.method,
    onContinue: onSetupContinue,
    onStepSelect: onSetupStepSelect,
    onFinish: () => {
      // The card unmounts, so focus moves to the page heading.
      window.requestAnimationFrame(() =>
        document.getElementById(headerId)?.focus()
      );
    },
  });

  const isFirstUse = state === "empty";
  const hasFirstAsset = !isFirstUse || setup.completedCount >= 2 || setup.done;
  const firstAsset =
    allocation.assets.find((asset) => asset.id === setup.values.assetId) ??
    portfolioAllocationAssets[0];
  const overviewAssets = isFirstUse
    ? hasFirstAsset && firstAsset
      ? [firstAsset]
      : []
    : portfolioAllocationAssets;
  const driftAsset = overviewAssets[0] ?? primaryPortfolioAsset(portfolio);
  // Strategic describes the largest class gap, matching the class rows below;
  // Simple describes the first asset.
  const classGaps = strategic
    ? buildPortfolioAssetSummary(
        portfolioAllocationAssets,
        allocation.allocation
      ).groups
    : [];
  const largestClassGap = classGaps.reduce<(typeof classGaps)[number] | null>(
    (best, group) =>
      !best || Math.abs(group.driftPp) > Math.abs(best.driftPp) ? group : best,
    null
  );
  const diagnosticDrift = largestClassGap
    ? largestClassGap.driftPp
    : driftAsset.driftPp;
  const diagnosticState = driftState(diagnosticDrift);
  const diagnostic = {
    name: largestClassGap
      ? largestClassGap.label
      : assetClassLabels[driftAsset.assetClass],
    driftPp: diagnosticDrift,
    status:
      diagnosticState === "on-target" ? ("within" as const) : diagnosticState,
  };
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

  // The table follows the Portfolio's allocation method; it has no mode switch.
  const collectionProps = {
    mode: allocation.method,
    showModeToggle: false,
    targets: strategic ? allocation.allocation : undefined,
    onEditClass: strategic
      ? (assetClass: AssetClass) => {
          classFocus.remember();
          setEditClass(assetClass);
        }
      : undefined,
    onEdit: (asset: PortfolioAsset) => {
      assetFocus.remember();
      setEditAssetId(asset.id);
      setEditAssetOpen(true);
      onEditAsset(asset.id);
    },
    onMenu: (asset: PortfolioAsset) => onAssetMenu(asset.id),
  };

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
      pageActions={[
        {
          label: "Add asset",
          icon: <Plus aria-hidden />,
          primary: true,
          onClick: onAddAsset,
        },
        {
          label: "Invest",
          icon: <TrendingUp aria-hidden />,
          onClick: onInvest,
        },
      ]}
    >
      <PageHeader
        id={headerId}
        tabIndex={-1}
        className="ds-page-header--overview"
        title={portfolio.label}
        description={
          <span className="ds-overview-meta">
            As of 2 Aug 2026 <span aria-hidden>·</span>{" "}
            <Badge>{strategic ? "Strategic" : "Simple"}</Badge>
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
            <IconButton
              label="Open portfolio settings"
              variant="ghost"
              onClick={onOpenSettings}
            >
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
          <PortfolioSetupWizard setup={setup} />
          {hasFirstAsset ? (
            <PortfolioAssetCollection
              assets={overviewAssets}
              presentation="overview"
              {...collectionProps}
            />
          ) : null}
        </>
      ) : (
        <>
          <DriftDiagnostic
            assetName={diagnostic.name}
            driftPp={diagnostic.driftPp}
            status={diagnostic.status}
            onReview={() => undefined}
          />
          <PortfolioAssetCollection
            assets={overviewAssets}
            presentation="overview"
            {...collectionProps}
          />
        </>
      )}
      <EditAssetClassDialog
        open={editClass !== null}
        onOpenChange={(next) => {
          if (!next) setEditClass(null);
        }}
        assetClass={editClass ?? "equities"}
        finalFocus={classFocus.ref}
        assets={portfolioAllocationAssets}
        allocation={allocation.allocation}
        onSave={(weight) => {
          if (editClass) allocation.setClassWeight(editClass, weight);
        }}
      />
      {editedAsset ? (
        <EditPortfolioAssetDialog
          open={editAssetOpen}
          onOpenChange={setEditAssetOpen}
          asset={editedAsset}
          assets={portfolioAllocationAssets}
          finalFocus={assetFocus.ref}
          method={allocation.method}
          allocation={allocation.allocation}
          feeSummary={assetFees.summary}
          onSave={(weight) => {
            if (weight.method === "strategic") {
              allocation.setAssetShare(editedAsset.id, weight.share);
            } else {
              allocation.setAbsoluteWeight(
                editedAsset.id,
                weight.absoluteWeight
              );
            }
          }}
        />
      ) : null}
      {assetFees.overlay}
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
  /**
   * The Portfolio's allocation method. Simple asks for an absolute target
   * weight; Strategic asks for the asset's share of its class.
   */
  allocationMethod?: AllocationMode;
}

const searchResults = [
  portfolioAssets[0],
  portfolioAssets[1],
  portfolioAssets[2],
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
  allocationMethod = "simple",
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
  const classTargetFor = (assetClass: AssetClass) =>
    portfolioAssets
      .filter((asset) => asset.assetClass === assetClass)
      .reduce((sum, asset) => sum + asset.targetWeight, 0) || 100;
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
        <section
          className="ds-search-results"
          aria-label="Asset search results"
        >
          {searchResults.map((asset) => (
            <SearchResultCard
              key={asset.id}
              asset={asset}
              selected={asset.id === selectedAssetId}
              onSelect={() => handleSelectResult(asset.id)}
            />
          ))}
        </section>
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
            {allocationMethod === "strategic" ? (
              <Field orientation="inline">
                <FieldLabel htmlFor="asset-target">
                  Share of {assetClassLabels[portfolioAsset.assetClass]}
                </FieldLabel>
                <InputWithSuffix
                  id="asset-target"
                  defaultValue={String(
                    Number(
                      (
                        (portfolioAsset.targetWeight /
                          classTargetFor(portfolioAsset.assetClass)) *
                        100
                      ).toFixed(1)
                    )
                  )}
                  inputMode="decimal"
                  suffix="%"
                />
              </Field>
            ) : (
              <Field orientation="inline">
                <FieldLabel htmlFor="asset-target">Target weight</FieldLabel>
                <InputWithSuffix
                  id="asset-target"
                  defaultValue={String(portfolioAsset.targetWeight)}
                  inputMode="decimal"
                  suffix="%"
                />
              </Field>
            )}
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
        allocationMethod={allocationMethod}
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
  /**
   * The Portfolio's allocation method. Simple edits one absolute target
   * weight and lists only the edited asset; Strategic edits the asset's share
   * of its class and lists the whole portfolio so the class is complete.
   */
  allocationMethod?: AllocationMode;
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
  allocationMethod = "simple",
}: EditRemoveAssetCompositionProps) {
  const { portfolio, setPortfolioId } = useCompositionPortfolio();
  const allocation = usePortfolioAllocation(portfolioAssets, allocationMethod);
  const strategic = allocation.method === "strategic";
  const visibleAssets = allocation.assets.filter((asset) =>
    portfolio.assets.some((candidate) => candidate.id === asset.id)
  );
  const primaryId = primaryPortfolioAsset(portfolio).id;
  const [editingId, setEditingId] = React.useState(primaryId);
  const portfolioAsset =
    allocation.assets.find((asset) => asset.id === editingId) ??
    allocation.assets[0];
  const showRemove = state === "remove-confirmation" || stage === "remove";
  const [saved, setSaved] = React.useState(state === "saved");
  const [editOpen, setEditOpen] = React.useState(stage === "details" && open);
  const [removeOpen, setRemoveOpen] = React.useState(
    state === "remove-confirmation" || stage === "remove"
  );
  const fees = useAssetFeeEditor(stage === "fees");
  const { setOverlayOpen: setFeeOverlayOpen } = fees;
  const editFocus = useReturnFocus();
  const portfolioAllocationAssets = allocation.assets.filter((asset) =>
    portfolio.assets.some((candidate) => candidate.id === asset.id)
  );
  React.useEffect(() => {
    setEditOpen(stage === "details" && open);
    setFeeOverlayOpen(stage === "fees");
    setRemoveOpen(state === "remove-confirmation" || stage === "remove");
  }, [open, stage, state, setFeeOverlayOpen]);

  const handleEditOpenChange = (nextOpen: boolean) => {
    setEditOpen(nextOpen);
  };

  const openEditDialog = (assetId: string) => {
    editFocus.remember();
    setEditingId(assetId);
    setEditOpen(true);
  };
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
        assets={strategic ? visibleAssets : [portfolioAsset]}
        mode={allocation.method}
        showModeToggle={false}
        targets={strategic ? allocation.allocation : undefined}
        onEdit={(asset) => openEditDialog(asset.id)}
      />
      <EditPortfolioAssetDialog
        open={editOpen}
        onOpenChange={handleEditOpenChange}
        asset={portfolioAsset}
        assets={portfolioAllocationAssets}
        finalFocus={editFocus.ref}
        method={allocation.method}
        allocation={allocation.allocation}
        feeSummary={fees.summary}
        onRemove={() => setRemoveOpen(true)}
        onSave={(weight) => {
          if (weight.method === "strategic") {
            allocation.setAssetShare(portfolioAsset.id, weight.share);
          } else {
            allocation.setAbsoluteWeight(
              portfolioAsset.id,
              weight.absoluteWeight
            );
          }
          setSaved(true);
        }}
      />
      {fees.overlay}
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
  /** The Portfolio's saved allocation method when the page opens. */
  allocationMethod?: AllocationMode;
  /** The method card selected when the page opens; defaults to the saved method. */
  initialSelection?: AllocationMode;
  /** Opens the Set your asset class weights dialog on first render (Canvas stories). */
  classWeightsOpen?: boolean;
  /** Class weights the dialog opens with; defaults to the weights implied by the asset targets. */
  classWeightsInitial?: ClassWeights;
  /** Returns to the Portfolio page. */
  onGoToPortfolio?: () => void;
}

const driftBandOptions = ["±2 pp", "±5 pp", "±10 pp"] as const;

/**
 * Portfolio settings: the allocation method, one Portfolio-wide drift band and
 * the transaction fees. Moving to Strategic opens the dialog that sets every
 * asset class weight at once; the method only changes when that dialog is saved.
 */
export function PortfolioSettingsComposition({
  saveState = "idle",
  allocationMethod = "simple",
  initialSelection,
  classWeightsOpen = false,
  classWeightsInitial,
  onGoToPortfolio = () => undefined,
}: PortfolioSettingsCompositionProps) {
  const { portfolio, setPortfolioId } = useCompositionPortfolio();
  const allocation = usePortfolioAllocation(portfolioAssets, allocationMethod);
  const [selected, setSelected] = React.useState<AllocationMode>(
    initialSelection ?? allocationMethod
  );
  const [weightsOpen, setWeightsOpen] = React.useState(classWeightsOpen);
  const [localSaveState, setLocalSaveState] = React.useState<"success" | null>(
    null
  );
  const [settingsFeePolicy, setSettingsFeePolicy] = React.useState<
    "zero" | "fixed" | "variable"
  >("zero");
  const [settingsFeeValues, setSettingsFeeValues] =
    React.useState<FeeValues>(defaultFeeValues);
  const [driftBand, setDriftBand] = React.useState<string>("±5 pp");
  React.useEffect(() => setWeightsOpen(classWeightsOpen), [classWeightsOpen]);
  const visibleSaveState = localSaveState ?? saveState;
  const weightsFocus = useReturnFocus();
  const classWeightsSavedRef = React.useRef(false);

  const handleSave = () => {
    if (selected === "strategic" && allocation.method === "simple") {
      weightsFocus.remember();
      setWeightsOpen(true);
      return;
    }
    if (selected === "simple" && allocation.method === "strategic") {
      allocation.switchToSimple();
    }
    setLocalSaveState("success");
  };

  return (
    <PortfolioCompositionShell
      portfolio={portfolio}
      onPortfolioChange={setPortfolioId}
      selectedItem="settings"
      pageActions={[
        {
          label: "Save changes",
          icon: <Check aria-hidden />,
          primary: true,
          onClick: handleSave,
        },
        ...(allocation.method === "strategic"
          ? [
              {
                label: "Review strategic allocation",
                icon: <Compass aria-hidden />,
                onClick: onGoToPortfolio,
              },
            ]
          : []),
      ]}
    >
      <PageHeader
        className="ds-settings-page__header"
        title="Portfolio settings"
        description={`Configure how ${portfolio.label} is built and how allocation suggestions work.`}
        actions={
          <Button variant="outline" onClick={onGoToPortfolio}>
            <ArrowLeft size={16} aria-hidden /> Back to {portfolio.label}
          </Button>
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
      <Card className="ds-settings-page">
        <CardContent>
          <FieldGroup>
            <div className="ds-field">
              <RadioCardGroup
                label="Allocation method"
                value={selected}
                onValueChange={(value) => setSelected(value as AllocationMode)}
                options={[
                  {
                    value: "simple",
                    title: "Simple",
                    description:
                      "One target weight per asset. Weights across all assets total 100%.",
                  },
                  {
                    value: "strategic",
                    title: "Strategic",
                    description:
                      "Targets per asset class, then each asset's share inside its class.",
                  },
                ]}
              />
              {selected === "strategic" && allocation.method === "strategic" ? (
                <Alert variant="info">
                  <Info size={18} aria-hidden />
                  <span>
                    Class weights and asset shares are edited from the Portfolio
                    page. Use the edit icon on a class row.{" "}
                    <Button variant="link" onClick={onGoToPortfolio}>
                      Go to Portfolio
                    </Button>
                  </span>
                </Alert>
              ) : null}
              {selected === "strategic" && allocation.method === "simple" ? (
                <Alert variant="info">
                  <Info size={18} aria-hidden />
                  <span>
                    Saving opens a dialog to set all class weights at once.
                    Later edits happen on the Portfolio page.
                  </span>
                </Alert>
              ) : null}
              {selected === "simple" && allocation.method === "strategic" ? (
                <Alert variant="info">
                  <Info size={18} aria-hidden />
                  <span>
                    Switching to Simple keeps each asset&apos;s current absolute
                    weight (class target × share) as its target. You can switch
                    back at any time.
                  </span>
                </Alert>
              ) : null}
            </div>
            <Field>
              <FieldLabel>Drift band</FieldLabel>
              <p className="ds-description">
                One range for the whole portfolio. Drift inside it counts as on
                target.
              </p>
              <Select
                value={driftBand}
                onValueChange={(value) => setDriftBand(String(value))}
              >
                <SelectTrigger aria-label="Drift band">
                  <SelectValue>{(value) => String(value ?? "")}</SelectValue>
                </SelectTrigger>
                <SelectContent>
                  {driftBandOptions.map((option) => (
                    <SelectItem key={option} value={option}>
                      {option}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </Field>
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
          <div className="ds-settings-page__footer">
            <Button onClick={handleSave}>Save changes</Button>
          </div>
        </CardContent>
      </Card>
      <SetClassWeightsDialog
        open={weightsOpen}
        onOpenChange={(next) => {
          setWeightsOpen(next);
          // Leaving the dialog without saving keeps the saved method.
          if (!next && !classWeightsSavedRef.current) setSelected("simple");
          if (!next) classWeightsSavedRef.current = false;
        }}
        assets={allocation.assets}
        shares={allocation.sharesOnSwitch}
        finalFocus={weightsFocus.ref}
        initialWeights={classWeightsInitial}
        onSave={(weights) => {
          classWeightsSavedRef.current = true;
          allocation.switchToStrategic(weights);
          setSelected("strategic");
          setLocalSaveState("success");
        }}
      />
    </PortfolioCompositionShell>
  );
}
