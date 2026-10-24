import * as React from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  CircleAlert,
  Info,
  PartyPopper,
  Pencil,
  Plus,
  RefreshCw,
  Search,
  Scale,
  Settings,
  ShieldCheck,
} from "lucide-react";
import { AppShell, TopNavigation } from "./app-shell";
import { DriftDiagnostic } from "./drift-diagnostic";
import { FloatingActionMenu } from "./floating-action-menu";
import { MetricCard } from "./metric-card";
import { PageHeader } from "./page-header";
import { PerformanceChart } from "./performance-chart";
import { PortfolioAssetCollection } from "./portfolio-asset-collection";
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

const navigation = (
  <TopNavigation
    links={[
      { label: "Overview", href: "#overview", active: true },
      { label: "Historical analytics", href: "#history" },
    ]}
    onMenuClick={() => undefined}
  />
);

const driftBandLabels = {
  default: "Default · ±5 pp",
  tight: "Tight · ±2 pp",
  wide: "Wide · ±10 pp",
} as const;

const transactionFeeLabels = {
  portfolio: "Portfolio default",
  zero: "Zero",
  fixed: "Fixed",
  variable: "Variable",
} as const;

function CloseDialogButton() {
  return <DialogCloseButton />;
}

export interface PortfolioOverviewCompositionProps {
  state?: "empty" | "populated";
  /** Optional desktop-only actions rendered beside the page heading. */
  headerActions?: React.ReactNode;
  onAddAsset?: () => void;
  onAllocate?: () => void;
  onRebalance?: () => void;
  onEditAsset?: (assetId: string) => void;
  onAssetMenu?: (assetId: string) => void;
}

/** The overview hub shown in the portfolio journey boards. */
export function PortfolioOverviewComposition({
  state = "populated",
  headerActions,
  onAddAsset = () => undefined,
  onAllocate = () => undefined,
  onRebalance = () => undefined,
  onEditAsset = () => undefined,
  onAssetMenu = () => undefined,
}: PortfolioOverviewCompositionProps) {
  return (
    <AppShell navigation={navigation}>
      <PageHeader
        className="ds-page-header--overview"
        title="Core portfolio"
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
              value: "€124,680.75",
              context: "As of 2 Aug 2026",
            }}
          />
          <MetricCard
            density="compact"
            metric={{
              label: "Day change",
              value: "+€612.18 (+0.49%)",
              context: "Since previous close",
              trend: "positive",
            }}
          />
          <MetricCard
            density="compact"
            metric={{
              label: "Cash balance",
              value: "€3,842.15",
              context: "Available to invest",
            }}
          />
        </div>
        <Card className="ds-overview-mobile-summary">
          <CardContent>
            <span className="ds-metric-card__label">Current value</span>
            <strong className="ds-overview-mobile-summary__value">
              €124,680.75
            </strong>
            <span className="ds-overview-mobile-summary__change">
              +€612.18 (+0.49%) <span>Today</span>
            </span>
            <div className="ds-overview-mobile-summary__cash">
              <span>Cash balance</span>
              <strong>€3,842.15</strong>
            </div>
          </CardContent>
        </Card>
      </div>
      <PerformanceChart state={state === "empty" ? "empty" : "ready"} />
      {state === "empty" ? (
        <EmptyState
          title="Your portfolio is ready for its first asset"
          description="Add an investment to start tracking value and allocation."
          action={<Button onClick={onAddAsset}>Add your first asset</Button>}
        />
      ) : (
        <>
          <DriftDiagnostic
            assetName="Equities"
            driftPp={4.8}
            status="over"
            onReview={() => undefined}
          />
          <PortfolioAssetCollection
            assets={portfolioAssets}
            onEdit={(asset) => onEditAsset(asset.id)}
            onMenu={(asset) => onAssetMenu(asset.id)}
          />
          <div className="ds-overview-mobile-fab">
            <FloatingActionMenu
              actions={[
                {
                  label: "Add asset",
                  icon: <Plus aria-hidden />,
                  onClick: onAddAsset,
                },
                {
                  label: "Allocate",
                  icon: <Scale aria-hidden />,
                  onClick: onAllocate,
                },
                {
                  label: "Rebalance",
                  icon: <RefreshCw aria-hidden />,
                  onClick: onRebalance,
                },
                {
                  label: "Edit portfolio",
                  icon: <Settings aria-hidden />,
                  onClick: () => undefined,
                },
              ]}
            />
          </div>
        </>
      )}
    </AppShell>
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
}: {
  asset: (typeof portfolioAssets)[number];
  selected?: boolean;
}) {
  return (
    <Button
      type="button"
      variant={selected ? "secondary" : "outline"}
      className="ds-search-result"
      pressed={selected}
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
}: AddAssetCompositionProps) {
  const isEdit = state === "edit";
  const isSearchResults = state === "search-results";
  const [dialogOpen, setDialogOpen] = React.useState(state !== "entry");
  const [feePolicy, setFeePolicy] = React.useState<FeePolicy>("portfolio");
  const [feeValues, setFeeValues] = React.useState<FeeValues>(defaultFeeValues);
  const [feeOverlayOpen, setFeeOverlayOpen] = React.useState(false);
  const feeTriggerRef = React.useRef<HTMLButtonElement>(null);
  const searchInputRef = React.useRef<HTMLInputElement>(null);

  React.useEffect(() => {
    setDialogOpen(state !== "entry");
  }, [state]);

  const handleFeePolicyChange = (nextPolicy: FeePolicy) => {
    setFeePolicy(nextPolicy);
  };

  return (
    <AppShell navigation={navigation}>
      <PageHeader title="Core portfolio" />
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <PortfolioAssetCollection
          assets={portfolioAssets.slice(0, 2)}
          headerActions={
            <DialogTrigger
              render={
                <Button
                  variant="ghost"
                  className="ds-add-asset-header-action"
                />
              }
            >
              Add asset
            </DialogTrigger>
          }
        />
        <FloatingActionMenu
          className="ds-add-asset-mobile-fab"
          label="Open add asset actions"
          actions={[
            {
              label: "Add asset",
              icon: <Plus aria-hidden />,
              onClick: () => setDialogOpen(true),
            },
          ]}
        />
        <DialogContent
          className="ds-add-asset-dialog"
          initialFocus={isEdit ? true : searchInputRef}
        >
          <DialogHeader>
            <div>
              <DialogTitle>{isEdit ? "Add VWCE.DE" : "Add asset"}</DialogTitle>
              <DialogDescription>
                {isEdit
                  ? "Configure this asset before adding it to your portfolio."
                  : "Search by name or ticker."}
              </DialogDescription>
            </div>
            <CloseDialogButton />
          </DialogHeader>
          <FieldGroup>
            {!isEdit ? (
              <Field>
                <FieldLabel htmlFor="asset-search">
                  Search by name or ticker
                </FieldLabel>
                <div className="ds-input-with-icon">
                  <Search size={16} aria-hidden />
                  <Input
                    ref={searchInputRef}
                    id="asset-search"
                    defaultValue={state === "entry" ? "" : "VW"}
                    placeholder="Search ETFs and more"
                  />
                </div>
              </Field>
            ) : null}
            {state === "loading" ? (
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
            {isSearchResults ? (
              <div
                className="ds-search-results"
                aria-label="Asset search results"
              >
                {searchResults.map((asset) => (
                  <SearchResultCard key={asset.id} asset={asset} />
                ))}
              </div>
            ) : null}
            {state === "empty" ? (
              <EmptyState
                title="No assets found"
                description="Try a different name or ticker."
              />
            ) : null}
            {state === "error" ? (
              <Alert variant="destructive">
                <CircleAlert size={18} aria-hidden />
                <div>
                  <strong>Search could not be completed</strong>
                  <p>Try again or check your connection.</p>
                </div>
              </Alert>
            ) : null}
            {state === "unpriced" ? (
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
            {isEdit ? (
              <>
                <div className="ds-asset-dialog-identity">
                  <span
                    className="ds-asset-card__dot ds-asset-card__dot--equities"
                    aria-hidden
                  />
                  <div>
                    <strong>Vanguard FTSE All-World UCITS ETF</strong>
                    <span>Equities · Global</span>
                  </div>
                  <AssetTickerChip ticker="VWCE.DE" />
                </div>
                <Field orientation="inline">
                  <FieldLabel htmlFor="asset-quantity">Quantity</FieldLabel>
                  <Input
                    id="asset-quantity"
                    defaultValue="0"
                    inputMode="decimal"
                  />
                </Field>
                <Field orientation="inline">
                  <FieldLabel htmlFor="asset-basis">
                    Basis cost / PMC
                  </FieldLabel>
                  <InputWithSuffix
                    id="asset-basis"
                    defaultValue="0.00"
                    inputMode="decimal"
                    suffix="EUR"
                  />
                </Field>
                <Field orientation="inline">
                  <FieldLabel htmlFor="asset-target">Target weight</FieldLabel>
                  <InputWithSuffix
                    id="asset-target"
                    defaultValue="0"
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
                          driftBandLabels[
                            value as keyof typeof driftBandLabels
                          ] ?? String(value ?? "")
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
              </>
            ) : null}
          </FieldGroup>
          <DialogFooter>
            {isEdit ? <Button>Save asset</Button> : null}
          </DialogFooter>
        </DialogContent>
      </Dialog>
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
    </AppShell>
  );
}

export type EditAssetStage = "details" | "fees" | "remove";

export interface EditRemoveAssetCompositionProps {
  state?: "editing" | "saved" | "error" | "remove-confirmation";
  stage?: EditAssetStage;
  open?: boolean;
}

function RemoveAssetDialog({
  defaultOpen = false,
  includeTrigger = false,
}: {
  defaultOpen?: boolean;
  includeTrigger?: boolean;
}) {
  return (
    <Dialog defaultOpen={defaultOpen}>
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
            <DialogTitle>Remove VWCE from this portfolio?</DialogTitle>
            <DialogDescription>
              This asset has a target weight of 30.0% and a current holding
              value of €3,522.77. All holding data, allocation, and historical
              performance will be removed.
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
  const showRemove = state === "remove-confirmation" || stage === "remove";
  const [editOpen, setEditOpen] = React.useState(stage === "details" && open);
  const [feePolicy, setFeePolicy] = React.useState<FeePolicy>("variable");
  const [feeValues, setFeeValues] = React.useState<FeeValues>(defaultFeeValues);
  const [feeOverlayOpen, setFeeOverlayOpen] = React.useState(stage === "fees");
  const feeTriggerRef = React.useRef<HTMLButtonElement>(null);
  const editOriginRef = React.useRef<HTMLElement | null>(null);
  React.useEffect(() => {
    setEditOpen(stage === "details" && open);
    setFeeOverlayOpen(stage === "fees");
  }, [open, stage]);

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
  return (
    <AppShell navigation={navigation}>
      <PageHeader title="Core portfolio" />
      {state === "saved" ? (
        <Alert variant="success">
          <Check size={18} aria-hidden />
          <strong>Asset changes saved</strong>
        </Alert>
      ) : null}
      {state === "error" ? (
        <Alert variant="destructive">
          <CircleAlert size={18} aria-hidden />
          <strong>Could not save asset changes</strong>
        </Alert>
      ) : null}
      <PortfolioAssetCollection
        assets={[portfolioAssets[0]]}
        onEdit={openEditDialog}
      />
      <Dialog open={editOpen} onOpenChange={handleEditOpenChange}>
        <DialogContent className="ds-edit-asset-dialog">
          <DialogHeader>
            <div>
              <DialogTitle>Edit VWCE</DialogTitle>
              <DialogDescription>
                Review and edit asset details.
              </DialogDescription>
            </div>
            <CloseDialogButton />
          </DialogHeader>
          <div className="ds-asset-dialog-identity">
            <span
              className="ds-asset-card__dot ds-asset-card__dot--equities"
              aria-hidden
            />
            <div>
              <strong>Vanguard FTSE All-World</strong>
              <span>Equities · Global</span>
            </div>
            <AssetTickerChip ticker="VWCE" />
          </div>
          <Separator />
          <FieldGroup>
            <Field orientation="inline">
              <FieldLabel htmlFor="edit-quantity">Quantity</FieldLabel>
              <Input id="edit-quantity" defaultValue="28.43" />
            </Field>
            <Field orientation="inline">
              <FieldLabel htmlFor="edit-basis">Basis cost / PMC</FieldLabel>
              <InputWithSuffix
                id="edit-basis"
                defaultValue="97.37"
                suffix="EUR"
              />
            </Field>
            <Field orientation="inline">
              <FieldLabel htmlFor="edit-target">Target weight</FieldLabel>
              <InputWithSuffix
                id="edit-target"
                defaultValue="30.0"
                suffix="%"
              />
            </Field>
            <Field orientation="inline">
              <FieldLabel>Current weight</FieldLabel>
              <span className="ds-value-readonly">34.8%</span>
            </Field>
            <Field orientation="inline">
              <FieldLabel>Drift</FieldLabel>
              <span className="ds-value-readonly ds-status-warning">
                +4.8 pp
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
          <DialogFooter className="ds-dialog-footer--stacked-mobile">
            <Button>Save changes</Button>
            <Button variant="destructive" type="button">
              Remove asset
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
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
      {showRemove ? <RemoveAssetDialog defaultOpen /> : null}
    </AppShell>
  );
}

export interface PortfolioSettingsCompositionProps {
  saveState?: "idle" | "success" | "error";
  view?: "settings" | "strategic";
  open?: boolean;
}

/** Settings and strategic allocation configuration stages. */
export function PortfolioSettingsComposition({
  saveState = "idle",
  view = "settings",
  open = false,
}: PortfolioSettingsCompositionProps) {
  const [method, setMethod] = React.useState<"simple" | "strategic">("simple");
  const [settingsFeePolicy, setSettingsFeePolicy] = React.useState<
    "zero" | "fixed" | "variable"
  >("zero");
  const [settingsFeeValues, setSettingsFeeValues] =
    React.useState<FeeValues>(defaultFeeValues);
  const strategic = view === "strategic";

  return (
    <AppShell navigation={navigation}>
      <PageHeader title="Core portfolio" />
      {saveState === "success" ? (
        <Alert variant="success">
          <Check size={18} aria-hidden />
          <strong>Portfolio settings saved</strong>
        </Alert>
      ) : null}
      {saveState === "error" ? (
        <Alert variant="destructive">
          <CircleAlert size={18} aria-hidden />
          <strong>Could not save portfolio settings</strong>
        </Alert>
      ) : null}
      <Dialog defaultOpen={open}>
        <DialogTrigger
          render={<Button variant="outline">Portfolio settings</Button>}
        />
        <DialogContent
          className={strategic ? "ds-strategic-dialog" : "ds-settings-dialog"}
        >
          <DialogHeader>
            <div>
              <DialogTitle>
                {strategic ? "Strategic allocation" : "Portfolio settings"}
              </DialogTitle>
              <DialogDescription>
                {strategic
                  ? "Set target weights and drift bands for each asset class."
                  : "Configure how this portfolio is built and how allocation suggestions work."}
              </DialogDescription>
            </div>
            <CloseDialogButton />
          </DialogHeader>
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
              <Button variant="link">
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
          <DialogFooter>
            <Button>
              {strategic ? "Save strategic allocation" : "Save changes"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </AppShell>
  );
}

export interface AllocateCompositionProps {
  state?: "options" | "result";
  mode?: "allocate" | "rebalance";
}

function AllocationOption({
  checked,
  children,
  description,
}: {
  checked?: boolean;
  children: React.ReactNode;
  description: string;
}) {
  return (
    <label className="ds-story-option ds-allocation-option">
      <Checkbox defaultChecked={checked} />
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
  const isResult = state === "result";
  const actionLabel = mode === "rebalance" ? "Rebalance" : "Allocate";
  return (
    <AppShell navigation={navigation}>
      <div className="ds-allocation-layout">
        <h1 className="ds-sr-only">Allocate and rebalance</h1>
        {!isResult ? (
          <Card className="ds-allocation-panel">
            <CardHeader>
              <Badge variant="primary">You came from: {actionLabel}</Badge>
              <CardTitle>How much do you want to allocate?</CardTitle>
            </CardHeader>
            <CardContent>
              <FieldGroup>
                <Field>
                  <FieldLabel htmlFor="allocation-amount">
                    Investment budget
                  </FieldLabel>
                  <InputWithSuffix
                    id="allocation-amount"
                    inputMode="decimal"
                    defaultValue="1,000"
                    suffix="EUR"
                  />
                  <p className="ds-description ds-allocation-note">
                    <ShieldCheck size={14} aria-hidden /> Your target allocation
                    is already set
                  </p>
                </Field>
                <AllocationOption
                  checked
                  description="Use buy-only recommendations where possible, preserving existing holdings."
                >
                  Tax-efficient algorithm
                </AllocationOption>
                <AllocationOption
                  checked
                  description="Use all available liquidity, even if it means using fractional quantities."
                >
                  Allocate the full budget
                </AllocationOption>
                <AllocationOption description="Suggest buying fractional quantities where supported.">
                  Use fractional quantities
                </AllocationOption>
              </FieldGroup>
            </CardContent>
            <CardFooter className="ds-allocation-footer ds-allocation-options__footer">
              <Button variant="ghost" size="sm">
                <ArrowLeft size={16} aria-hidden /> Back
              </Button>
              <Button>
                {actionLabel} budget <ArrowRight size={16} aria-hidden />
              </Button>
            </CardFooter>
          </Card>
        ) : (
          <div className="ds-allocation-result">
            <Card className="ds-allocation-panel">
              <CardHeader>
                <span className="ds-result-icon" aria-hidden>
                  <PartyPopper size={24} strokeWidth={1.8} />
                </span>
                <CardTitle>Your allocation is ready</CardTitle>
                <CardDescription>Review and confirm the plan.</CardDescription>
              </CardHeader>
              <CardContent className="ds-story-stack">
                <Card className="ds-review-card">
                  <CardHeader>
                    <CardTitle>Buy 6 VWCE.DE @ €162.96</CardTitle>
                    <CardDescription>
                      Vanguard FTSE All-World UCITS ETF
                    </CardDescription>
                  </CardHeader>
                  <CardContent>
                    <dl className="ds-review-list">
                      <div className="ds-review-row">
                        <dt>Amount</dt>
                        <dd>
                          <strong>€2,607.36</strong>
                          <span className="ds-status-success">+€977.76</span>
                        </dd>
                      </div>
                      <div className="ds-review-row">
                        <dt>Weight</dt>
                        <dd>
                          <strong>100.0%</strong>
                          <span className="ds-status-success">+70.0 pp</span>
                        </dd>
                      </div>
                      <div className="ds-review-row">
                        <dt>Commission charged</dt>
                        <dd>
                          <strong>€0.00</strong>
                        </dd>
                      </div>
                      <div className="ds-review-row">
                        <dt>Fee impact</dt>
                        <dd>
                          <strong>0.0%</strong>
                        </dd>
                      </div>
                    </dl>
                  </CardContent>
                </Card>
                <Card className="ds-review-card">
                  <CardHeader>
                    <CardTitle>
                      Unallocated cash <Badge variant="success">+€22.24</Badge>
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <p className="ds-description">
                      This remaining budget is kept aside for next time.
                    </p>
                  </CardContent>
                </Card>
              </CardContent>
              <CardFooter className="ds-allocation-footer">
                <Button variant="ghost" size="sm">
                  <ArrowLeft size={16} aria-hidden /> Back to portfolio
                </Button>
                <Button>Update portfolio</Button>
              </CardFooter>
            </Card>
          </div>
        )}
      </div>
    </AppShell>
  );
}
