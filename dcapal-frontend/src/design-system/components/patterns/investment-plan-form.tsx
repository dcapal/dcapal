import * as React from "react";
import { Info, TriangleAlert } from "lucide-react";
import { formatCurrency } from "../../fixtures/portfolio";
import {
  Alert,
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  CheckboxCard,
  Field,
  FieldDescription,
  FieldLabel,
  InputWithSuffix,
  SegmentedControl,
  Skeleton,
} from "../ui";
import { DriftClassGroupList, DriftRowList } from "./drift-overview";
import { classTotalDriftPp, groupDriftByClass } from "./invest-classes";
import { canContinue } from "./invest-plan";
import { formatPoints } from "./invest-format";
import type {
  AllocationMode,
  DriftSnapshot,
  InvestmentPlan,
  InvestmentStrategy,
  SuggestedAmountState,
} from "./invest-types";
import { StickyActionBar } from "./sticky-action-bar";
import {
  TransactionFeeConfiguration,
  type TransactionFeeConfigurationProps,
} from "./transaction-fee-configuration";

export interface SuggestedAmountFieldProps {
  /** Label of the amount input. */
  label: string;
  /** What the investor has typed, exactly as typed. */
  value: string;
  onValueChange: (value: string) => void;
  currency: string;
  /** The Suggested investment amount: loading, a figure, or already on target. */
  suggested: SuggestedAmountState;
  /** Called when the investor accepts the suggested amount. */
  onUseSuggested?: () => void;
  /** Helper text shown under the input in addition to the suggestion. */
  hint?: string;
}

/**
 * An amount input with its Suggested investment amount hint. The hint is
 * announced politely as it moves between loading, an amount and "already on
 * target".
 */
export function SuggestedAmountField({
  label,
  value,
  onValueChange,
  currency,
  suggested,
  onUseSuggested,
  hint,
}: SuggestedAmountFieldProps) {
  return (
    <Field className="ds-suggested-amount">
      <FieldLabel>{label}</FieldLabel>
      <InputWithSuffix
        suffix={currency}
        inputMode="decimal"
        autoComplete="off"
        value={value}
        onChange={(event) => onValueChange(event.target.value)}
      />
      {hint ? <FieldDescription>{hint}</FieldDescription> : null}
      <div className="ds-suggested-amount__hint" aria-live="polite">
        {suggested.status === "loading" ? (
          <>
            <Skeleton className="ds-suggested-amount__skeleton" />
            <span className="ds-sr-only">
              Calculating the suggested investment amount
            </span>
          </>
        ) : suggested.status === "amount" ? (
          <>
            <span>
              Suggested investment amount:{" "}
              <strong>{formatCurrency(suggested.amount, currency)}</strong>
            </span>
            <Button variant="link" size="sm" onClick={onUseSuggested}>
              Use suggested
            </Button>
          </>
        ) : (
          <span>You already reached your target allocation.</span>
        )}
      </div>
    </Field>
  );
}

export interface CommissionSummaryRowProps {
  /** One line describing the active fee policy, such as "Portfolio default, €1 per transaction". */
  summary: string;
  /** Called when the investor wants to change the policy. */
  onEdit?: () => void;
  /** Receives focus when the editor closes. */
  editRef?: React.Ref<HTMLButtonElement>;
}

/** The active transaction fee policy in one line, with an action to edit it. */
export function CommissionSummaryRow({
  summary,
  onEdit,
  editRef,
}: CommissionSummaryRowProps) {
  return (
    <div className="ds-fee-summary">
      <span className="ds-fee-summary__copy">
        <strong>Commissions</strong>
        <span>{summary}</span>
      </span>
      <Button
        ref={editRef}
        variant="link"
        size="sm"
        aria-label="Edit commissions"
        onClick={onEdit}
      >
        Edit
      </Button>
    </div>
  );
}

/** Configuration of the fee editor that opens from the plan form. */
export type InvestmentPlanFormCommissions = Omit<
  TransactionFeeConfigurationProps,
  "open" | "onOpenChange" | "finalFocus"
> & {
  /** One line describing the active fee policy. */
  summary: string;
};

export interface InvestmentPlanFormProps {
  /** The investor's choices; strategy, unit rule and budget use. */
  plan: InvestmentPlan;
  onPlanChange: (plan: InvestmentPlan) => void;
  /** The investment budget as typed; kept as text so partial input is not rewritten. */
  amountText: string;
  onAmountTextChange: (text: string) => void;
  /** The Suggested investment amount hint state. */
  suggested: SuggestedAmountState;
  onUseSuggested?: () => void;
  /** Where the Portfolio stands today, shown beside the form. */
  drift: DriftSnapshot;
  /** Allocation mode: Strategic lists asset classes with their assets nested. Defaults to simple. */
  mode?: AllocationMode;
  /** The parsed budget, or null when nothing valid was typed. */
  amount: number | null;
  /** Fee policy summary and editor configuration. */
  commissions: InvestmentPlanFormCommissions;
  /** Controls whether the fee editor is open; uncontrolled when omitted. */
  feesOpen?: boolean;
  onFeesOpenChange?: (open: boolean) => void;
  onBack?: () => void;
  onContinue?: () => void;
}

const strategyCopy: Record<
  InvestmentStrategy,
  { description: string; amountLabel: string }
> = {
  "add-money": {
    description:
      "Add money only buys. Your holdings are never sold, which also keeps it tax efficient.",
    amountLabel: "Amount to invest",
  },
  rebalance: {
    description:
      "New money is optional. Leave it at zero to rebalance with what you hold.",
    amountLabel: "New money to add (optional)",
  },
};

/** Lists tickers as plain text, such as "AGGH and IEUR". */
function joinTickers(tickers: readonly string[]): string {
  if (tickers.length <= 1) return tickers.join("");
  return `${tickers.slice(0, -1).join(", ")} and ${tickers[tickers.length - 1]}`;
}

/**
 * The plan step of the Invest journey: choose a path, set a budget and rules,
 * and see where the Portfolio stands. The form is controlled and does no
 * calculation of its own besides the rule that gates Continue.
 */
export function InvestmentPlanForm({
  plan,
  onPlanChange,
  amountText,
  onAmountTextChange,
  suggested,
  onUseSuggested,
  drift,
  mode = "simple",
  amount,
  commissions,
  feesOpen,
  onFeesOpenChange,
  onBack,
  onContinue,
}: InvestmentPlanFormProps) {
  const [localFeesOpen, setLocalFeesOpen] = React.useState(false);
  const editRef = React.useRef<HTMLButtonElement>(null);
  const reasonId = React.useId();
  const open = feesOpen ?? localFeesOpen;
  const setOpen = (next: boolean) => {
    setLocalFeesOpen(next);
    onFeesOpenChange?.(next);
  };
  const check = canContinue(amount, drift.portfolioValue);
  const copy = strategyCopy[plan.strategy];
  const { summary, ...feeProps } = commissions;
  const rebalancing = plan.strategy === "rebalance";
  const strategic = mode === "strategic";
  const classGroups = strategic ? groupDriftByClass(drift.rows) : [];
  const above = strategic
    ? classGroups.filter((group) => group.driftPp > 0)
    : drift.rows.filter((row) => row.driftPp > 0);
  const below = strategic
    ? classGroups.filter((group) => group.driftPp < 0)
    : drift.rows.filter((row) => row.driftPp < 0);
  const totalDrift = strategic
    ? classTotalDriftPp(classGroups)
    : drift.totalDriftPp;
  const tickerOf = (
    item:
      { assetClass: string; name: string; ticker: string } | { label: string }
  ) =>
    "label" in item
      ? item.label
      : item.assetClass === "cash"
        ? item.name
        : item.ticker;

  return (
    <div className="ds-plan-form ds-invest-flow">
      <div className="ds-plan-form__layout">
        <Card className="ds-plan-form__rules">
          <CardHeader>
            <CardTitle>Budget and rules</CardTitle>
            <SegmentedControl
              size="md"
              block
              className="ds-plan-form__strategy"
              label="Investment path"
              value={plan.strategy}
              onValueChange={(value) =>
                onPlanChange({ ...plan, strategy: value as InvestmentStrategy })
              }
              options={[
                { value: "add-money", label: "Add money" },
                { value: "rebalance", label: "Rebalance" },
              ]}
            />
            <CardDescription>{copy.description}</CardDescription>
          </CardHeader>
          <CardContent className="ds-plan-form__fields">
            <SuggestedAmountField
              label={copy.amountLabel}
              value={amountText}
              onValueChange={onAmountTextChange}
              currency={drift.currency}
              suggested={suggested}
              onUseSuggested={onUseSuggested}
            />
            <CheckboxCard
              title="Allocate the full budget"
              description="Anything left after whole units stays as cash."
              checked={plan.useAllBudget}
              onCheckedChange={(checked) =>
                onPlanChange({ ...plan, useAllBudget: checked })
              }
            />
            <CheckboxCard
              title="Use fractional quantities"
              description="Equities always trade in whole units."
              checked={!plan.wholeUnits}
              onCheckedChange={(checked) =>
                onPlanChange({ ...plan, wholeUnits: !checked })
              }
            />
            <CommissionSummaryRow
              summary={
                rebalancing
                  ? `${summary}. Applies to buys; sale commissions are not estimated.`
                  : summary
              }
              editRef={editRef}
              onEdit={() => setOpen(true)}
            />
            {check.reason ? (
              <p className="ds-plan-form__reason" id={reasonId}>
                <Info aria-hidden size={16} /> {check.reason}
              </p>
            ) : null}
            <StickyActionBar
              label="Plan actions"
              secondary={
                <Button variant="link" onClick={onBack}>
                  Back to drift
                </Button>
              }
              primary={
                <Button
                  disabled={!check.allowed}
                  aria-describedby={check.reason ? reasonId : undefined}
                  onClick={onContinue}
                >
                  Continue
                </Button>
              }
            />
          </CardContent>
        </Card>
        <Card className="ds-plan-form__today">
          <CardHeader>
            <div className="ds-plan-form__today-heading">
              <CardTitle>Where you are today</CardTitle>
              <Badge variant={totalDrift > 0 ? "warning" : "success"}>
                {formatPoints(totalDrift)} drift
              </Badge>
            </div>
            <CardDescription>
              Your proposal appears after you continue. Marker = target.
            </CardDescription>
          </CardHeader>
          <CardContent className="ds-plan-form__today-body">
            {strategic ? (
              <DriftClassGroupList
                rows={drift.rows}
                density="compact"
                label="Asset classes and assets compared with their targets"
              />
            ) : (
              <DriftRowList
                rows={drift.rows}
                density="compact"
                label="Assets compared with their targets"
              />
            )}
            {rebalancing ? (
              <Alert variant="warning">
                <TriangleAlert aria-hidden size={16} />
                <p className="ds-card__description">
                  Rebalance may sell {joinTickers(above.map(tickerOf))}, which{" "}
                  {above.length === 1 ? "is" : "are"} above target. Selling can
                  have tax implications. Sale commissions are not estimated.
                </p>
              </Alert>
            ) : (
              <Alert variant="info">
                <Info aria-hidden size={16} />
                <p className="ds-card__description">
                  With add money, {strategic ? "classes" : "assets"} above
                  target ({above.map(tickerOf).join(", ")}) keep their units.
                  Only {joinTickers(below.map(tickerOf))} are candidates to buy.
                </p>
              </Alert>
            )}
          </CardContent>
        </Card>
      </div>
      <TransactionFeeConfiguration
        {...feeProps}
        open={open}
        onOpenChange={setOpen}
        finalFocus={editRef}
      />
    </div>
  );
}
