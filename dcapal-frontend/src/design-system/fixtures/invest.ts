import {
  createDriftSnapshot,
  totalDriftPp,
} from "../components/patterns/invest-plan";
import type {
  AllocationAfterRow,
  DriftRowModel,
  DriftSnapshot,
  InvestmentPlan,
  InvestmentProposalModel,
  InvestmentResultModel,
  ProposalState,
  ProposedTransaction,
  SuggestedAmountState,
} from "../components/patterns/invest-types";
import {
  formatCurrency,
  formatDrift,
  portfolioAssets,
  type PortfolioAsset,
} from "./portfolio";

/*
 * Deterministic Invest scenarios. The quantities and transactions below are
 * the output of the optimizer for the stated holdings (whole units, fixed fee
 * of EUR 1 per transaction), so the stories never claim behaviour the solver
 * does not have. The optimizer does not net commissions from the budget it
 * reports as left over, and it does not model commissions on sells, so
 * only buys carry a commission. Cash is a funding source, not a transaction.
 */

const CURRENCY = "EUR";
const FIXED_FEE = 1;

/** Holdings of the scenario, by Portfolio Asset id. Cash is an ordinary asset priced at 1. */
const holdings: Record<string, { quantity: number; price: number }> = {
  vwce: { quantity: 28, price: 124.8 },
  aggh: { quantity: 40, price: 5.12 },
  ieur: { quantity: 44, price: 33.2 },
  vig: { quantity: 12, price: 148.2 },
  cash: { quantity: 3887, price: 1 },
};

/** Quantity bought (positive) or sold (negative) per asset id by each strategy. */
const addMoneyDeltas: Record<string, number> = { aggh: 117, ieur: 12 };
const rebalanceDeltas: Record<string, number> = {
  vwce: -1,
  vig: -1,
  cash: -2804,
  aggh: 464,
  ieur: 21,
};

const catalogue = new Map<string, PortfolioAsset>(
  portfolioAssets.map((asset) => [asset.id, asset])
);

/** Target weights of the scenario, in percent, matching the glossary's Target weight. */
const targets: Record<string, number> = {
  vwce: 30,
  aggh: 25,
  ieur: 20,
  vig: 15,
  cash: 10,
};

const assetIds = Object.keys(holdings);

function asset(id: string): PortfolioAsset {
  const found = catalogue.get(id);
  if (!found) {
    throw new Error(`Unknown portfolio asset ${id}`);
  }
  return found;
}

function valueOf(id: string, quantity = holdings[id].quantity): number {
  return quantity * holdings[id].price;
}

const portfolioValue = assetIds.reduce((sum, id) => sum + valueOf(id), 0);

/** Formats Total drift in percentage points without a sign. */
function formatPp(value: number): string {
  return formatDrift(value).replace("+", "");
}

function weightOf(value: number, total: number): number {
  return (value / total) * 100;
}

/** Drift of the scenario's Portfolio before any investment. */
export const investDrift: DriftSnapshot = createDriftSnapshot(
  assetIds.map((id): DriftRowModel => {
    const base = asset(id);
    const currentWeight = weightOf(valueOf(id), portfolioValue);
    return {
      id,
      name: base.name,
      ticker: base.ticker,
      assetClass: base.assetClass,
      currentWeight,
      targetWeight: targets[id],
      driftPp: currentWeight - targets[id],
    };
  }),
  portfolioValue,
  CURRENCY
);

/** Default plan for adding EUR 1,000 of new money. */
export const addMoneyPlan: InvestmentPlan = {
  strategy: "add-money",
  amount: 1000,
  wholeUnits: true,
  useAllBudget: false,
};

/** Default plan for a Rebalancing that uses no new money. */
export const rebalancePlan: InvestmentPlan = {
  strategy: "rebalance",
  amount: 0,
  wholeUnits: true,
  useAllBudget: false,
};

function buildProposal(
  plan: InvestmentPlan,
  deltas: Record<string, number>
): InvestmentProposalModel {
  const budget = plan.amount;
  const afterTotal = portfolioValue + budget;

  const transactions = Object.entries(deltas)
    .filter(([id]) => id !== "cash")
    .map(([id, delta]): ProposedTransaction => {
      const base = asset(id);
      const { price } = holdings[id];
      const quantity = Math.abs(delta);
      const amount = quantity * price;
      const isBuy = delta > 0;
      return {
        id,
        name: base.name,
        ticker: base.ticker,
        assetClass: base.assetClass,
        action: delta > 0 ? "buy" : "sell",
        quantity,
        price,
        amount,
        ...(isBuy
          ? { commission: FIXED_FEE, firstYearFeeImpact: FIXED_FEE }
          : {}),
        weightBefore: weightOf(valueOf(id), portfolioValue),
        weightAfter: weightOf(
          valueOf(id, holdings[id].quantity + delta),
          afterTotal
        ),
        targetWeight: targets[id],
      };
    });

  const cashUsed = -(deltas.cash ?? 0);

  const bought = transactions
    .filter((t) => t.action === "buy")
    .reduce((sum, t) => sum + t.amount, 0);
  const sold = transactions
    .filter((t) => t.action === "sell")
    .reduce((sum, t) => sum + t.amount, 0);
  const commissions = transactions.reduce(
    (sum, t) => sum + (t.commission ?? 0),
    0
  );
  const leftoverCash =
    Math.round((budget + sold + cashUsed - bought) * 100) / 100;

  const allocationAfter = assetIds.map((id): AllocationAfterRow => {
    const base = asset(id);
    const quantityAfter = holdings[id].quantity + (deltas[id] ?? 0);
    return {
      id,
      name: base.name,
      ticker: base.ticker,
      assetClass: base.assetClass,
      weightBefore: weightOf(valueOf(id), portfolioValue),
      weightAfter: weightOf(valueOf(id, quantityAfter), afterTotal),
      targetWeight: targets[id],
    };
  });

  const driftAfter = totalDriftPp(
    allocationAfter.map((row) => ({
      currentWeight: row.weightAfter,
      targetWeight: row.targetWeight,
    }))
  );
  const cashAfter = holdings.cash.quantity - cashUsed + leftoverCash;

  return {
    strategy: plan.strategy,
    currency: CURRENCY,
    metrics: [
      {
        label: "Total drift",
        value: `${formatPp(investDrift.totalDriftPp)} → ${formatPp(driftAfter)}`,
        context: "Before and after these transactions",
      },
      {
        label: "Transactions",
        value: String(transactions.length),
        context: `${transactions.filter((t) => t.action === "buy").length} buy, ${
          transactions.filter((t) => t.action === "sell").length
        } sell`,
      },
      {
        label: "Commissions",
        value: formatCurrency(commissions, CURRENCY),
        context: "Estimated with your fee policy",
      },
      {
        label: "Unallocated cash",
        value: formatCurrency(leftoverCash, CURRENCY),
        context: "Not used by any transaction",
      },
    ],
    fundedBy: { newMoney: budget, saleProceeds: sold, cashUsed },
    salesCommissionNotEstimated: true,
    transactions,
    allocationAfter,
    driftBefore: investDrift.totalDriftPp,
    driftAfter,
    leftoverCash,
    cashAfter,
  };
}

/** Allocation of EUR 1,000 of new money: buys only. */
export const addMoneyProposal: InvestmentProposalModel = buildProposal(
  addMoneyPlan,
  addMoneyDeltas
);

/**
 * Rebalancing with no new money: sells overweight assets, including the cash
 * position, and buys the underweight ones.
 */
export const rebalanceProposal: InvestmentProposalModel = buildProposal(
  rebalancePlan,
  rebalanceDeltas
);

function buildResult(
  proposal: InvestmentProposalModel,
  deltas: Record<string, number>
): InvestmentResultModel {
  return {
    strategy: proposal.strategy,
    currency: CURRENCY,
    changes: Object.entries(deltas).map(([id, delta]) => ({
      id,
      name: asset(id).name,
      ticker: asset(id).ticker,
      quantityBefore: holdings[id].quantity,
      quantityAfter: holdings[id].quantity + delta,
    })),
    driftNow: proposal.driftAfter,
    cashNow: proposal.cashAfter,
  };
}

/** Summary after applying the Add money proposal. */
export const addMoneyResult: InvestmentResultModel = buildResult(
  addMoneyProposal,
  addMoneyDeltas
);

/** Summary after applying the Rebalancing proposal. */
export const rebalanceResult: InvestmentResultModel = buildResult(
  rebalanceProposal,
  rebalanceDeltas
);

/** Suggested investment amount hint, in each of its three states. */
export const suggestedAmountStates: Record<
  "loading" | "amount" | "onTarget",
  SuggestedAmountState
> = {
  loading: { status: "loading" },
  amount: { status: "amount", amount: 1000 },
  onTarget: { status: "on-target" },
};

/** Portfolio-value-aware copy used when an investor has reached their targets. */
export const onTargetMessage = "You already reached your target allocation.";

/** A Portfolio whose weights match its targets. */
export const onTargetDrift: DriftSnapshot = createDriftSnapshot(
  investDrift.rows.map((row) => ({
    ...row,
    currentWeight: row.targetWeight,
    driftPp: 0,
  })),
  portfolioValue,
  CURRENCY
);

/**
 * A buy that the maximum fee impact dropped. The "should have" figures are the
 * theoretical allocation the optimizer reports: EUR 1 on a EUR 148.48 buy is
 * 0.67%, above the accepted 0.50%.
 */
export const feeRejectedTransaction: ProposedTransaction = {
  id: "aggh",
  name: asset("aggh").name,
  ticker: "AGGH",
  assetClass: "bonds",
  action: "hold",
  quantity: 0,
  price: 5.12,
  amount: 0,
  weightBefore: weightOf(valueOf("aggh"), portfolioValue),
  weightAfter: weightOf(valueOf("aggh"), portfolioValue),
  targetWeight: targets.aggh,
  feeRejected: {
    shouldHaveQuantity: 29,
    shouldHaveAmount: 29 * 5.12,
    commissionImpact: (FIXED_FEE / (29 * 5.12)) * 100,
    maxFeeImpact: 0.5,
  },
};

/** A proposal whose only buy was dropped for fees; the budget is left unallocated. */
export const feesTooHighProposal: InvestmentProposalModel = {
  ...addMoneyProposal,
  metrics: [
    {
      label: "Total drift",
      value: `${formatPp(investDrift.totalDriftPp)} → ${formatPp(investDrift.totalDriftPp)}`,
      context: "No change",
    },
    { label: "Transactions", value: "0", context: "Dropped for fees" },
    {
      label: "Commissions",
      value: formatCurrency(0, CURRENCY),
      context: "Nothing to pay",
    },
    {
      label: "Unallocated cash",
      value: formatCurrency(150, CURRENCY),
      context: "Not used by any transaction",
    },
  ],
  fundedBy: null,
  salesCommissionNotEstimated: false,
  transactions: [feeRejectedTransaction],
  driftAfter: investDrift.totalDriftPp,
  leftoverCash: 150,
  cashAfter: holdings.cash.quantity + 150,
};

/** A proposal where every asset costs more than the remaining budget. */
export const priceTooHighProposal: InvestmentProposalModel = {
  ...feesTooHighProposal,
  metrics: [
    {
      label: "Total drift",
      value: `${formatPp(investDrift.totalDriftPp)} → ${formatPp(investDrift.totalDriftPp)}`,
      context: "No change",
    },
    { label: "Transactions", value: "0", context: "Price above budget" },
    {
      label: "Commissions",
      value: formatCurrency(0, CURRENCY),
      context: "Nothing to pay",
    },
    {
      label: "Unallocated cash",
      value: formatCurrency(100, CURRENCY),
      context: "Not used by any transaction",
    },
  ],
  transactions: [],
  leftoverCash: 100,
  cashAfter: holdings.cash.quantity + 100,
};

/** Investment budget that is too small to buy a whole unit of any underweight asset. */
export const priceTooHighPlan: InvestmentPlan = {
  ...addMoneyPlan,
  amount: 100,
};

/** Investment budget small enough for the fee limit to drop its only buy. */
export const feesTooHighPlan: InvestmentPlan = {
  ...addMoneyPlan,
  amount: 150,
};

/** Target weights that total 95% and so cannot be solved. */
export const blockedTargetTotal = 95;

/** Copy for each proposal state that has no transactions to show. */
export const proposalStateMessages: Record<
  Exclude<ProposalState, "ready" | "loading">,
  { title: string; description: string }
> = {
  blocked: {
    title: "Target weights must total 100%",
    description: `Your target weights total ${blockedTargetTotal}%. Adjust them in the portfolio settings, then try again.`,
  },
  "needs-amount": {
    title: "Nothing to invest yet",
    description:
      "Enter an amount to invest, or add holdings to your portfolio, to see proposed transactions.",
  },
  "on-target": {
    title: "You already reached your target allocation",
    description: `Total drift is ${formatPp(0)}. There are no proposed transactions.`,
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
