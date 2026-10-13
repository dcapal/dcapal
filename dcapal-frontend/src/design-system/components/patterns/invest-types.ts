import type { AssetClass } from "../../fixtures/portfolio";

/** The path an investor takes once they have seen their drift. */
export type InvestmentStrategy = "add-money" | "rebalance";

/** One Portfolio Asset compared with its Target weight. */
export interface DriftRowModel {
  id: string;
  name: string;
  ticker: string;
  assetClass: AssetClass;
  /** Current weight, in percent of the Portfolio. */
  currentWeight: number;
  /** Target weight, in percent of the Portfolio. */
  targetWeight: number;
  /** Current weight minus Target weight, in percentage points. */
  driftPp: number;
}

/** How far a Portfolio currently sits from its Target weights. */
export interface DriftSnapshot {
  currency: string;
  portfolioValue: number;
  rows: DriftRowModel[];
  /** Total drift, in percentage points. */
  totalDriftPp: number;
  /** The Portfolio Asset with the widest gap, or null when there are none. */
  largestGap: DriftRowModel | null;
}

/** The investor's inputs for one Invest journey. */
export interface InvestmentPlan {
  strategy: InvestmentStrategy;
  /** Investment budget, in the quote currency. Zero is valid for a Rebalancing. */
  amount: number;
  /** Unit rule: true recommends whole units only. */
  wholeUnits: boolean;
  /** Budget-use preference: true asks to use as much of the budget as constraints allow. */
  useAllBudget: boolean;
}

/** Why a Proposed transaction was not recommended because of its fee. */
export interface FeeRejection {
  /** Quantity the investor would have been advised to transact without the fee limit. */
  shouldHaveQuantity: number;
  /** Amount the investor would have been advised to transact without the fee limit. */
  shouldHaveAmount: number;
  /** Commission as a percentage of that amount. */
  commissionImpact: number;
  /** Maximum fee impact the investor accepts, as a percentage. */
  maxFeeImpact: number;
}

/** One recommended buy or sell of a Portfolio Asset. It is not an executed trade. */
export interface ProposedTransaction {
  id: string;
  name: string;
  ticker: string;
  assetClass: AssetClass;
  action: "buy" | "sell" | "hold";
  quantity: number;
  price: number;
  /** Transaction amount, in the quote currency. */
  amount: number;
  /** Buy commission. Absent for sells: the optimizer does not model them. */
  commission?: number;
  /** Buy commission as the cost over the first year of holding, in the quote currency. Absent for sells. */
  firstYearFeeImpact?: number;
  /** Current weight before the transaction, in percent. */
  weightBefore: number;
  /** Projected weight after the transaction, in percent. */
  weightAfter: number;
  targetWeight: number;
  /** Present when the transaction was dropped because of its fee. */
  feeRejected?: FeeRejection;
}

/** One summary figure shown above a proposal. */
export interface InvestmentMetric {
  label: string;
  value: string;
  context?: string;
}

/** A Portfolio Asset's weight before and after a proposal. */
export interface AllocationAfterRow {
  id: string;
  name: string;
  ticker: string;
  assetClass: AssetClass;
  weightBefore: number;
  weightAfter: number;
  targetWeight: number;
}

/** Where the money for the buys of a proposal comes from. */
export interface FundedBy {
  /** New money from the investment budget. */
  newMoney: number;
  /** Proceeds of selling Portfolio Assets other than cash. */
  saleProceeds: number;
  /** Cash balance used. Cash is a funding source, never a Proposed transaction. */
  cashUsed: number;
}

/** The recommendation shown for an Allocation or a Rebalancing. */
export interface InvestmentProposalModel {
  strategy: InvestmentStrategy;
  currency: string;
  metrics: InvestmentMetric[];
  /** Where the money for the buys comes from, or null when nothing is bought. */
  fundedBy: FundedBy | null;
  /** True when sell commissions are not part of the figures, so the UI can say so. */
  salesCommissionNotEstimated: boolean;
  transactions: ProposedTransaction[];
  allocationAfter: AllocationAfterRow[];
  /** Total drift before the proposal, in percentage points. */
  driftBefore: number;
  /** Total drift after the proposal, in percentage points. */
  driftAfter: number;
  /** Unallocated cash: the part of the budget no transaction uses. */
  leftoverCash: number;
  /** Cash held once the proposal is applied. */
  cashAfter: number;
}

/** A quantity of one Portfolio Asset before and after the investor applies a proposal. */
export interface InvestmentResultChange {
  id: string;
  name: string;
  ticker: string;
  quantityBefore: number;
  quantityAfter: number;
}

/** The summary shown after the investor updates their holdings. */
export interface InvestmentResultModel {
  strategy: InvestmentStrategy;
  currency: string;
  changes: InvestmentResultChange[];
  /** Total drift now, in percentage points. */
  driftNow: number;
  cashNow: number;
}

/** The states an investment proposal can be in. */
export type ProposalState =
  | "ready"
  | "loading"
  | "blocked"
  | "needs-amount"
  | "on-target"
  | "fees-too-high"
  | "price-too-high"
  | "error";

/** The states of the Suggested investment amount hint. */
export type SuggestedAmountState =
  | { status: "loading" }
  | { status: "amount"; amount: number }
  | { status: "on-target" };
