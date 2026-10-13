import * as React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { Button, Card, CardContent } from "../../components/ui";
import {
  AllocationAfterList,
  AllocationBar,
  DriftOverview,
  InvestmentPathActions,
  InvestmentPlanForm,
  InvestmentProposal,
  InvestmentResult,
  PageHeader,
  parseAmountInput,
  PortfolioWorkspace,
  ProposedTransactionCollection,
  StickyActionBar,
  type InvestmentPlan,
  type ProposalState,
  type SuggestedAmountState,
} from "../../components/patterns";
import {
  addMoneyPlan,
  addMoneyProposal,
  addMoneyResult,
  feeRejectedTransaction,
  feesTooHighPlan,
  feesTooHighProposal,
  investDrift,
  onTargetDrift,
  priceTooHighPlan,
  priceTooHighProposal,
  proposalStateMessages,
  rebalancePlan,
  rebalanceProposal,
  rebalanceResult,
  suggestedAmountStates,
} from "../../fixtures/invest";

const meta = {
  title: "Patterns / Invest",
  component: DriftOverview,
  args: { snapshot: investDrift },
  tags: ["autodocs"],
  parameters: {
    layout: "padded",
    docs: { story: { inline: false, iframeHeight: 900 } },
  },
} satisfies Meta<typeof DriftOverview>;
export default meta;
type Story = StoryObj<typeof meta>;

const mobile = { viewport: { defaultViewport: "mobile" } } as const;
const commissionSummary = "Portfolio default, €1 per transaction";

/* AllocationBar */

export const AllocationBarStates: Story = {
  render: () => (
    <div className="dcapal-theme ds-story-stack">
      <AllocationBar
        name="Cash"
        assetClass="cash"
        today={35.5}
        target={10}
        scaleMax={40}
      />
      <AllocationBar
        name="VWCE"
        assetClass="equities"
        today={32.5}
        after={29.8}
        target={30}
        scaleMax={40}
      />
      <AllocationBar
        name="AGGH"
        assetClass="bonds"
        today={1.9}
        after={8.2}
        target={25}
        scaleMax={40}
      />
    </div>
  ),
};

/* DriftOverview */

export const DriftOverviewDesktop: Story = {};

/** On a phone the metrics pair up and each row stacks its bar under its name. */
export const DriftOverviewMobile: Story = {
  parameters: mobile,
};

export const DriftOverviewOnTarget: Story = {
  args: { snapshot: onTargetDrift },
  tags: ["!autodocs"],
};

/** Metric values never break mid-number, even in a narrow container. */
export const DriftOverviewNarrowMetrics: Story = {
  tags: ["!autodocs"],
  decorators: [
    (Story) => (
      <div style={{ maxWidth: "36rem" }}>
        <Story />
      </div>
    ),
  ],
};

/* InvestmentPathActions */

/** Both paths available: Add money is primary, Rebalance secondary. Place them in the page header. */
export const InvestmentPathActionButtons: Story = {
  render: () => (
    <div className="dcapal-theme ds-story-stack" style={{ maxWidth: "26rem" }}>
      <InvestmentPathActions />
    </div>
  ),
};

/** Paths unavailable, for example while targets do not total 100%. */
export const InvestmentPathActionsDisabled: Story = {
  render: () => (
    <div className="dcapal-theme ds-story-stack" style={{ maxWidth: "26rem" }}>
      <InvestmentPathActions disabled />
    </div>
  ),
};

/* InvestmentPlanForm */

interface PlanFormDemoProps {
  initialPlan: InvestmentPlan;
  initialAmountText: string;
  suggested: SuggestedAmountState;
  portfolioValue?: number;
}

/** Holds the plan state a real screen would own, so the form can be tried out. */
function PlanFormDemo({
  initialPlan,
  initialAmountText,
  suggested,
  portfolioValue = investDrift.portfolioValue,
}: PlanFormDemoProps) {
  const [plan, setPlan] = React.useState(initialPlan);
  const [amountText, setAmountText] = React.useState(initialAmountText);
  const amount = parseAmountInput(amountText);

  return (
    <div className="dcapal-theme">
      <InvestmentPlanForm
        plan={{ ...plan, amount: amount ?? 0 }}
        onPlanChange={setPlan}
        amountText={amountText}
        onAmountTextChange={setAmountText}
        amount={amount}
        suggested={suggested}
        onUseSuggested={() =>
          suggested.status === "amount"
            ? setAmountText(String(suggested.amount))
            : undefined
        }
        drift={{ ...investDrift, portfolioValue }}
        commissions={{
          summary: commissionSummary,
          policy: "fixed",
          initialAmount: "1.00",
        }}
      />
    </div>
  );
}

export const PlanFormAddMoney: Story = {
  render: () => (
    <PlanFormDemo
      initialPlan={addMoneyPlan}
      initialAmountText="1,000"
      suggested={suggestedAmountStates.amount}
    />
  ),
};

export const PlanFormRebalance: Story = {
  render: () => (
    <PlanFormDemo
      initialPlan={rebalancePlan}
      initialAmountText="0"
      suggested={suggestedAmountStates.amount}
    />
  ),
};

export const PlanFormAddMoneyMobile: Story = {
  parameters: mobile,
  render: () => (
    <PlanFormDemo
      initialPlan={addMoneyPlan}
      initialAmountText="1,000"
      suggested={suggestedAmountStates.amount}
    />
  ),
};

export const PlanFormSuggestedLoading: Story = {
  render: () => (
    <PlanFormDemo
      initialPlan={addMoneyPlan}
      initialAmountText="1,000"
      suggested={suggestedAmountStates.loading}
    />
  ),
  tags: ["!autodocs"],
};

export const PlanFormAlreadyOnTarget: Story = {
  render: () => (
    <PlanFormDemo
      initialPlan={addMoneyPlan}
      initialAmountText="1,000"
      suggested={suggestedAmountStates.onTarget}
    />
  ),
  tags: ["!autodocs"],
};

/** An empty portfolio with no budget cannot continue, and says why. */
export const PlanFormNothingToInvest: Story = {
  render: () => (
    <PlanFormDemo
      initialPlan={{ ...addMoneyPlan, amount: 0 }}
      initialAmountText="0"
      suggested={suggestedAmountStates.onTarget}
      portfolioValue={0}
    />
  ),
  tags: ["!autodocs"],
};

/* ProposedTransactionCollection */

export const ProposedTransactionsExpanded: Story = {
  render: () => (
    <div className="dcapal-theme">
      <ProposedTransactionCollection
        transactions={rebalanceProposal.transactions}
        currency="EUR"
        defaultExpandedIds={["vwce", "aggh"]}
      />
    </div>
  ),
};

export const ProposedTransactionsFeeRejected: Story = {
  render: () => (
    <div className="dcapal-theme">
      <ProposedTransactionCollection
        transactions={[feeRejectedTransaction]}
        currency="EUR"
        defaultExpandedIds={[feeRejectedTransaction.id]}
      />
    </div>
  ),
};

export const ProposedTransactionsMobile: Story = {
  parameters: mobile,
  render: () => (
    <div className="dcapal-theme">
      <ProposedTransactionCollection
        transactions={[
          ...rebalanceProposal.transactions,
          feeRejectedTransaction,
        ].map((transaction, index) =>
          index === 4 ? { ...transaction, id: "aggh-skipped" } : transaction
        )}
        currency="EUR"
        defaultExpandedIds={["vwce", "aggh-skipped"]}
      />
    </div>
  ),
};

export const AllocationAfter: Story = {
  render: () => (
    <div className="dcapal-theme" style={{ maxWidth: "48rem" }}>
      <AllocationAfterList rows={addMoneyProposal.allocationAfter} />
    </div>
  ),
};

/* InvestmentProposal */

interface ProposalDemoProps {
  state: ProposalState;
  strategy: "add-money" | "rebalance";
}

const proposalFixtures = {
  ready: {
    "add-money": { plan: addMoneyPlan, proposal: addMoneyProposal },
    rebalance: { plan: rebalancePlan, proposal: rebalanceProposal },
  },
  "fees-too-high": {
    "add-money": { plan: feesTooHighPlan, proposal: feesTooHighProposal },
  },
  "price-too-high": {
    "add-money": { plan: priceTooHighPlan, proposal: priceTooHighProposal },
  },
} as const;

/** Wraps the proposal in the theme with fixture data for the state under test. */
function ProposalDemo({ state, strategy }: ProposalDemoProps) {
  const fixture =
    state === "ready"
      ? proposalFixtures.ready[strategy]
      : state === "fees-too-high"
        ? proposalFixtures["fees-too-high"]["add-money"]
        : state === "price-too-high"
          ? proposalFixtures["price-too-high"]["add-money"]
          : {
              plan: strategy === "add-money" ? addMoneyPlan : rebalancePlan,
              proposal: null,
            };
  const message =
    state === "ready" || state === "loading"
      ? undefined
      : proposalStateMessages[state];

  return (
    <div className="dcapal-theme">
      <InvestmentProposal
        state={state}
        proposal={fixture.proposal}
        plan={fixture.plan}
        commissionSummary={commissionSummary}
        suggestedAmount={820}
        stateMessage={message}
      />
    </div>
  );
}

export const ProposalAddMoney: Story = {
  render: () => <ProposalDemo state="ready" strategy="add-money" />,
};

export const ProposalRebalance: Story = {
  render: () => <ProposalDemo state="ready" strategy="rebalance" />,
};

export const ProposalAddMoneyMobile: Story = {
  parameters: mobile,
  render: () => <ProposalDemo state="ready" strategy="add-money" />,
};

export const ProposalRebalanceMobile: Story = {
  parameters: mobile,
  render: () => <ProposalDemo state="ready" strategy="rebalance" />,
  tags: ["!autodocs"],
};

export const ProposalLoading: Story = {
  render: () => <ProposalDemo state="loading" strategy="add-money" />,
  tags: ["!autodocs"],
};

export const ProposalBlocked: Story = {
  render: () => <ProposalDemo state="blocked" strategy="add-money" />,
  tags: ["!autodocs"],
};

export const ProposalNeedsAmount: Story = {
  render: () => <ProposalDemo state="needs-amount" strategy="add-money" />,
  tags: ["!autodocs"],
};

export const ProposalOnTarget: Story = {
  render: () => <ProposalDemo state="on-target" strategy="add-money" />,
  tags: ["!autodocs"],
};

export const ProposalFeesTooHigh: Story = {
  render: () => <ProposalDemo state="fees-too-high" strategy="add-money" />,
  tags: ["!autodocs"],
};

export const ProposalPriceTooHigh: Story = {
  render: () => <ProposalDemo state="price-too-high" strategy="add-money" />,
  tags: ["!autodocs"],
};

export const ProposalError: Story = {
  render: () => <ProposalDemo state="error" strategy="add-money" />,
  tags: ["!autodocs"],
};

/* InvestmentResult */

export const ResultAddMoney: Story = {
  render: () => (
    <div className="dcapal-theme">
      <InvestmentResult result={addMoneyResult} />
    </div>
  ),
};

export const ResultRebalance: Story = {
  render: () => (
    <div className="dcapal-theme">
      <InvestmentResult result={rebalanceResult} />
    </div>
  ),
  tags: ["!autodocs"],
};

export const ResultMobile: Story = {
  parameters: mobile,
  render: () => (
    <div className="dcapal-theme">
      <InvestmentResult result={addMoneyResult} />
    </div>
  ),
};

/* StickyActionBar */

/**
 * On small screens the bar is pinned to the bottom edge and the portfolio
 * navigation button sits above it instead of overlapping.
 */
export const StickyActionBarWithNavigationButton: Story = {
  parameters: mobile,
  render: () => (
    <PortfolioWorkspace portfolioName="Core portfolio" selectedItem="invest">
      <PageHeader title="Plan your investment" />
      <div className="ds-invest-flow">
        <Card>
          <CardContent>
            <p>
              The actions below stay at the bottom of the screen. The navigation
              button moves above them.
            </p>
            <StickyActionBar
              label="Example actions"
              secondary={<Button variant="link">Back to drift</Button>}
              primary={<Button>Continue</Button>}
            />
          </CardContent>
        </Card>
      </div>
    </PortfolioWorkspace>
  ),
  tags: ["!autodocs"],
};
