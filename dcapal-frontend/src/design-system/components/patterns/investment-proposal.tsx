import * as React from "react";
import { CircleCheck, Info, TriangleAlert } from "lucide-react";
import { formatCurrency } from "../../fixtures/portfolio";
import {
  Alert,
  AlertDescription,
  AlertTitle,
  Badge,
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  DescriptionList,
  Skeleton,
  type AlertVariant,
} from "../ui";
import { AllocationAfterList } from "./allocation-after-list";
import { groupProposalByClass } from "./invest-classes";
import { formatPoints } from "./invest-format";
import type {
  AllocationMode,
  FundedBy,
  InvestmentPlan,
  InvestmentProposalModel,
  ProposalState,
} from "./invest-types";
import { MetricCard, MetricStrip } from "./metric-card";
import { ProposedTransactionCollection } from "./proposed-transaction-collection";
import { StickyActionBar } from "./sticky-action-bar";

/** Title and explanation shown for a proposal state that has no transactions to approve. */
export interface ProposalStateMessage {
  title: string;
  description: string;
}

type MessageState = Exclude<ProposalState, "ready" | "loading">;

const defaultMessages: Record<MessageState, ProposalStateMessage> = {
  blocked: {
    title: "Target weights must total 100%",
    description:
      "Adjust your target weights from the Portfolio page, then try again.",
  },
  "needs-amount": {
    title: "Nothing to invest yet",
    description:
      "Enter an amount to invest, or add holdings to your portfolio, to see proposed transactions.",
  },
  "on-target": {
    title: "You already reached your target allocation",
    description: "There are no proposed transactions.",
  },
  "fees-too-high": {
    title: "Transaction fees are too high",
    description:
      "Every proposed buy costs more in commissions than your maximum fee impact allows. Raise the limit or invest a larger amount.",
  },
  "price-too-high": {
    title: "Your amount is too small for whole units",
    description:
      "No asset can be bought with whole units for this amount. Invest more, or allow fractional units where the asset supports them.",
  },
  error: {
    title: "We could not calculate your transactions",
    description:
      "Something went wrong while preparing the proposal. Try again.",
  },
};

const messageVariant: Record<MessageState, AlertVariant> = {
  blocked: "destructive",
  "needs-amount": "warning",
  "on-target": "success",
  "fees-too-high": "warning",
  "price-too-high": "warning",
  error: "destructive",
};

export interface InvestmentProposalProps {
  /** Which step of the proposal the investor is looking at. */
  state: ProposalState;
  /** The recommendation; needed by the ready, fees-too-high and price-too-high states. */
  proposal: InvestmentProposalModel | null;
  /** Allocation mode: Strategic groups transactions and bars by asset class. Defaults to simple. */
  mode?: AllocationMode;
  /** The choices that produced the proposal, summarised under "Your rules". */
  plan: InvestmentPlan;
  /** One line describing the active fee policy. */
  commissionSummary: string;
  /** Suggested investment amount, offered as a recovery when the amount is too small. */
  suggestedAmount?: number;
  /** Overrides the default title and explanation of a state without transactions. */
  stateMessage?: ProposalStateMessage;
  /** Goes back to the plan step. */
  onBack?: () => void;
  /** Applies the proposal to the holdings. */
  onUpdate?: () => void;
  /** Reopens the rules summarised under "Your rules". */
  onChangeRules?: () => void;
  onEditCommissions?: () => void;
  onEditTargets?: () => void;
  onUseSuggested?: () => void;
  onTryAgain?: () => void;
  onBackToDrift?: () => void;
}

/** Describes where the money for the buys comes from in one sentence. */
function fundedByText(
  fundedBy: FundedBy,
  currency: string,
  salesCommissionNotEstimated: boolean
): string {
  const parts: string[] = [];
  if (fundedBy.newMoney > 0) {
    parts.push(`${formatCurrency(fundedBy.newMoney, currency)} of new money`);
  }
  if (fundedBy.saleProceeds > 0) {
    parts.push(`${formatCurrency(fundedBy.saleProceeds, currency)} from sales`);
  }
  if (fundedBy.cashUsed > 0) {
    parts.push(
      `${formatCurrency(fundedBy.cashUsed, currency)} from your cash balance`
    );
  }
  const joined =
    parts.length > 1
      ? `${parts.slice(0, -1).join(", ")} and ${parts[parts.length - 1]}`
      : parts[0];
  return `Funded by ${joined}.${
    salesCommissionNotEstimated && fundedBy.saleProceeds > 0
      ? " Sale commissions are not estimated."
      : ""
  }`;
}

/** Counts transactions per action for the card badge, such as "4 transactions · 2 sells". */
function summarise(proposal: InvestmentProposalModel): string {
  const real = proposal.transactions.filter((t) => t.action !== "hold");
  const sells = real.filter((t) => t.action === "sell").length;
  const noun = real.length === 1 ? "transaction" : "transactions";
  const base = `${real.length} ${noun}`;
  return sells > 0
    ? `${base} · ${sells} ${sells === 1 ? "sell" : "sells"}`
    : base;
}

const strategyName = {
  "add-money": "Add money",
  rebalance: "Rebalance",
} as const;

/** The summary rows of "Your rules". */
function rulesItems(
  plan: InvestmentPlan,
  currency: string,
  commission: string
) {
  const rebalancing = plan.strategy === "rebalance";
  return [
    { id: "strategy", label: "Strategy", value: strategyName[plan.strategy] },
    {
      id: "amount",
      label: rebalancing ? "New money" : "Amount",
      value: formatCurrency(plan.amount, currency),
    },
    ...(rebalancing
      ? [{ id: "sells", label: "Sells", value: "Allowed" }]
      : [
          {
            id: "budget",
            label: "Full budget",
            value: plan.useAllBudget ? "On" : "Off",
          },
        ]),
    {
      id: "fractional",
      label: "Fractional",
      value: plan.wholeUnits ? "Off" : "On",
    },
    { id: "commission", label: "Commission", value: commission },
  ];
}

/**
 * The recommendation of an Allocation or a Rebalancing, or the reason there is
 * none. It shows what to do, where the money comes from, how the allocation
 * changes, and the rules behind it. Nothing changes until the investor updates
 * their portfolio.
 */
export function InvestmentProposal({
  state,
  proposal,
  mode = "simple",
  plan,
  commissionSummary,
  suggestedAmount,
  stateMessage,
  onBack,
  onUpdate,
  onChangeRules,
  onEditCommissions,
  onEditTargets,
  onUseSuggested,
  onTryAgain,
  onBackToDrift,
}: InvestmentProposalProps) {
  const needsProposal =
    state === "ready" ||
    state === "fees-too-high" ||
    state === "price-too-high";
  const effective: ProposalState = needsProposal && !proposal ? "error" : state;

  if (effective === "loading") {
    return (
      <Card
        className="ds-proposal ds-proposal--loading ds-invest-flow"
        role="status"
        aria-busy="true"
        aria-label="Calculating your proposal"
      >
        <CardHeader>
          <CardTitle>Calculating your proposal&hellip;</CardTitle>
          <p className="ds-card__description">
            Running in your browser. This takes a moment.
          </p>
        </CardHeader>
        <CardContent className="ds-proposal__skeleton">
          <Skeleton style={{ width: "90%" }} />
          <Skeleton style={{ width: "70%" }} />
          <Skeleton style={{ width: "80%" }} />
          <Skeleton style={{ width: "55%" }} />
        </CardContent>
      </Card>
    );
  }

  if (effective !== "ready") {
    const message = stateMessage ?? defaultMessages[effective];
    const variant = messageVariant[effective];
    const Icon = variant === "success" ? CircleCheck : TriangleAlert;
    const actions: Record<
      MessageState,
      { secondary?: React.ReactNode; primary: React.ReactNode }
    > = {
      blocked: {
        secondary: (
          <Button variant="link" onClick={onBack}>
            Back to rules
          </Button>
        ),
        primary: <Button onClick={onEditTargets}>Edit targets</Button>,
      },
      "needs-amount": {
        primary: <Button onClick={onBack}>Back to rules</Button>,
      },
      "on-target": {
        secondary: (
          <Button variant="link" onClick={onBack}>
            Back to rules
          </Button>
        ),
        primary: (
          <Button variant="outline" onClick={onBackToDrift}>
            Back to drift
          </Button>
        ),
      },
      "fees-too-high": {
        secondary: (
          <Button variant="link" onClick={onBack}>
            Back to rules
          </Button>
        ),
        primary: <Button onClick={onEditCommissions}>Edit commissions</Button>,
      },
      "price-too-high": {
        secondary: (
          <Button variant="link" onClick={onBack}>
            Back to rules
          </Button>
        ),
        primary:
          suggestedAmount !== undefined ? (
            <Button onClick={onUseSuggested}>
              Use suggested{" "}
              {formatCurrency(suggestedAmount, proposal?.currency)}
            </Button>
          ) : (
            <Button onClick={onBack}>Change amount</Button>
          ),
      },
      error: {
        secondary: (
          <Button variant="link" onClick={onBack}>
            Back to rules
          </Button>
        ),
        primary: <Button onClick={onTryAgain}>Try again</Button>,
      },
    };

    return (
      <div className="ds-proposal ds-proposal--state ds-invest-flow">
        {proposal &&
        (effective === "fees-too-high" || effective === "price-too-high") ? (
          <MetricStrip label="Proposal summary" metrics={proposal.metrics} />
        ) : null}
        <Card>
          <CardContent className="ds-proposal__state">
            <Alert variant={variant}>
              <Icon aria-hidden size={18} />
              <div>
                <AlertTitle aria-level={2} role="heading">
                  {message.title}
                </AlertTitle>
                <AlertDescription>{message.description}</AlertDescription>
              </div>
            </Alert>
            {proposal && effective === "fees-too-high" ? (
              <ProposedTransactionCollection
                transactions={proposal.transactions}
                currency={proposal.currency}
                defaultExpandedIds={proposal.transactions
                  .filter((t) => t.feeRejected)
                  .map((t) => t.id)}
              />
            ) : null}
            <StickyActionBar
              label="Proposal actions"
              secondary={actions[effective].secondary}
              primary={actions[effective].primary}
            />
          </CardContent>
        </Card>
      </div>
    );
  }

  // `ready` always has a proposal: otherwise `effective` fell back to error.
  const model = proposal as InvestmentProposalModel;

  return (
    <div className="ds-proposal ds-invest-flow">
      <MetricStrip label="Proposal summary" metrics={model.metrics} />
      {model.fundedBy ? (
        <Alert variant="info">
          <Info aria-hidden size={18} />
          <p className="ds-card__description">
            {fundedByText(
              model.fundedBy,
              model.currency,
              model.salesCommissionNotEstimated
            )}
          </p>
        </Alert>
      ) : null}
      <div className="ds-proposal__layout">
        <div className="ds-proposal__main">
          <Card>
            <CardHeader className="ds-proposal__card-heading">
              <CardTitle>Proposed transactions</CardTitle>
              <Badge variant={strategyBadge(model)}>{summarise(model)}</Badge>
            </CardHeader>
            <CardContent className="ds-proposal__card-body">
              <ProposedTransactionCollection
                transactions={model.transactions}
                currency={model.currency}
                groups={
                  mode === "strategic" ? groupProposalByClass(model) : undefined
                }
              />
              {model.salesCommissionNotEstimated &&
              model.transactions.some((t) => t.action === "sell") ? (
                <p className="ds-proposal__note">
                  Sale commissions are not estimated.
                </p>
              ) : null}
            </CardContent>
          </Card>
          <Card>
            <CardContent>
              <AllocationAfterList rows={model.allocationAfter} mode={mode} />
            </CardContent>
          </Card>
          <StickyActionBar
            label="Proposal actions"
            secondary={
              <Button variant="link" onClick={onBack}>
                Back to rules
              </Button>
            }
            primary={<Button onClick={onUpdate}>Update portfolio</Button>}
          />
        </div>
        <div className="ds-proposal__aside">
          <MetricCard
            metric={{
              label: "Drift",
              value: `${formatPoints(model.driftBefore)} → ${formatPoints(model.driftAfter)}`,
              context:
                mode === "strategic"
                  ? "Total distance from your class targets, before and after."
                  : "Total distance from your targets, before and after.",
            }}
          />
          <Card density="compact">
            <CardHeader className="ds-proposal__card-heading">
              <CardTitle>Your rules</CardTitle>
              <Button variant="link" size="sm" onClick={onChangeRules}>
                Change
              </Button>
            </CardHeader>
            <CardContent>
              <DescriptionList
                aria-label="Your rules"
                items={rulesItems(plan, model.currency, commissionSummary)}
              />
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}

function strategyBadge(model: InvestmentProposalModel) {
  return model.strategy === "rebalance" ? "warning" : "success";
}
