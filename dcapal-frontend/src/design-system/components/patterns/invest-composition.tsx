import * as React from "react";
import { ArrowLeftRight, Plus } from "lucide-react";
import { formatCurrency } from "../../fixtures/portfolio";
import {
  addMoneyPlan,
  addMoneyProposal,
  addMoneyProposalStrategic,
  addMoneyResult,
  addMoneyResultStrategic,
  blockedInvestTargets,
  feesTooHighPlan,
  feesTooHighProposal,
  investDrift,
  investTargets,
  priceTooHighPlan,
  priceTooHighProposal,
  proposalStateMessages,
  rebalancePlan,
  rebalanceProposal,
  rebalanceProposalStrategic,
  rebalanceResult,
  rebalanceResultStrategic,
  suggestedAmountStates,
} from "../../fixtures/invest";
import {
  PortfolioCompositionShell,
  useCompositionPortfolio,
} from "./compositions";
import { DriftOverview } from "./drift-overview";
import { canContinue, parseAmountInput } from "./invest-plan";
import {
  checkTargetTotals,
  targetTotalsMessage,
  type TargetDefinition,
} from "./invest-targets";
import type {
  AllocationMode,
  InvestmentPlan,
  InvestmentProposalModel,
  InvestmentStrategy,
  ProposalState,
} from "./invest-types";
import { Alert, AlertDescription, AlertTitle, Button } from "../ui";
import { InvestmentPathActions } from "./investment-path-actions";
import { InvestmentPlanForm } from "./investment-plan-form";
import { InvestmentProposal } from "./investment-proposal";
import { InvestmentResult } from "./investment-result";
import { PageHeader } from "./page-header";
import type { TransactionFeeConfigurationValues } from "./transaction-fee-configuration";

/** The steps of the Invest journey. */
export type InvestScreen = "drift" | "plan" | "proposal" | "updated";

export interface InvestCompositionProps {
  /**
   * How targets are defined. Strategic groups every step by asset class; the
   * default Simple allocation lists assets only.
   */
  mode?: AllocationMode;
  /**
   * The target weights behind the journey. When they do not total 100% at
   * every level, Add money and Rebalance are unavailable and the proposal is
   * blocked. Defaults to complete targets, or incomplete ones when the
   * proposal state is forced to blocked.
   */
  targets?: TargetDefinition;
  /** Called when the investor asks to edit targets from the blocked state. */
  onEditTargets?: () => void;
  /** The step the journey starts on. */
  screen?: InvestScreen;
  /** The path preselected on the plan step. */
  strategy?: InvestmentStrategy;
  /**
   * Forces the state the proposal step ends in, so every edge state can be
   * shown. When omitted the proposal is ready.
   */
  proposalState?: ProposalState;
  /** Opens the commissions editor on the plan step. */
  feeDialogOpen?: boolean;
  /**
   * How long Continue shows the loading state, in milliseconds. Zero skips the
   * loading state, which keeps stories and tests deterministic.
   */
  loadingDelayMs?: number;
}

/** The fee policy the investor chose, with the values behind it. */
interface FeeSettings {
  policy: TransactionFeeConfigurationValues["policy"];
  values: Omit<TransactionFeeConfigurationValues, "policy">;
}

const defaultFees: FeeSettings = {
  policy: "fixed",
  values: {
    maximumImpact: "0.5",
    amount: "1.00",
    feeRate: "0.25",
    minFee: "1.00",
    maxFee: "25.00",
  },
};

/** Default budget for each path, as the investor would type it. */
const defaultAmountText: Record<InvestmentStrategy, string> = {
  "add-money": "1000",
  rebalance: "0",
};

/** The suggested amount offered once the analysis has run. */
const suggestedAmount =
  suggestedAmountStates.amount.status === "amount"
    ? suggestedAmountStates.amount.amount
    : 0;

/** Describes the active fee policy in one line for the plan and proposal. */
function describeFees({ policy, values }: FeeSettings): string {
  if (policy === "zero") return "No commissions";
  if (policy === "variable") {
    return `Variable, ${values.feeRate}% (min €${values.minFee}, max €${values.maxFee})`;
  }
  return `${policy === "fixed" ? "Fixed" : "Portfolio default"}, ${formatCurrency(
    Number(values.amount) || 0,
    investDrift.currency
  )} per transaction`;
}

/**
 * Picks the fixture scenario behind a proposal. The figures are static: the
 * optimizer is not wired in Storybook. Two budgets map to the edge scenarios
 * so that they can be reached by typing: the fixture amount that is too small
 * for a whole unit and the one the fee limit drops.
 */
function scenarioFor(
  strategy: InvestmentStrategy,
  amount: number | null,
  forced: ProposalState | undefined,
  mode: AllocationMode,
  targetsComplete: boolean
): {
  state: ProposalState;
  plan: InvestmentPlan;
  proposal: InvestmentProposalModel | null;
} {
  if (!targetsComplete || forced === "blocked") {
    const plan = strategy === "rebalance" ? rebalancePlan : addMoneyPlan;
    return { state: "blocked", plan, proposal: null };
  }
  if (forced === "fees-too-high") {
    return {
      state: forced,
      plan: feesTooHighPlan,
      proposal: feesTooHighProposal,
    };
  }
  if (forced === "price-too-high") {
    return {
      state: forced,
      plan: priceTooHighPlan,
      proposal: priceTooHighProposal,
    };
  }
  if (forced && forced !== "ready" && forced !== "loading") {
    const plan = strategy === "rebalance" ? rebalancePlan : addMoneyPlan;
    return { state: forced, plan, proposal: null };
  }
  if (strategy === "add-money" && amount === priceTooHighPlan.amount) {
    return {
      state: "price-too-high",
      plan: priceTooHighPlan,
      proposal: priceTooHighProposal,
    };
  }
  if (strategy === "add-money" && amount === feesTooHighPlan.amount) {
    return {
      state: "fees-too-high",
      plan: feesTooHighPlan,
      proposal: feesTooHighProposal,
    };
  }
  const strategic = mode === "strategic";
  return strategy === "rebalance"
    ? {
        state: "ready",
        plan: rebalancePlan,
        proposal: strategic ? rebalanceProposalStrategic : rebalanceProposal,
      }
    : {
        state: "ready",
        plan: addMoneyPlan,
        proposal: strategic ? addMoneyProposalStrategic : addMoneyProposal,
      };
}

const screenCopy: Record<
  InvestScreen,
  { eyebrow: string; description: string }
> = {
  drift: {
    eyebrow: "Step 1 of 3 · Drift",
    description:
      "See how far your portfolio sits from its targets, then choose how to invest.",
  },
  plan: {
    eyebrow: "Step 2 of 3 · Plan",
    description: "Set an amount and your rules. Nothing changes yet.",
  },
  proposal: {
    eyebrow: "Step 3 of 3 · Proposal",
    description:
      "Review the proposed transactions. DcaPal does not place trades for you.",
  },
  updated: {
    eyebrow: "Done",
    description: "Your holdings now match the proposal.",
  },
};

/**
 * The Invest journey: see the drift, pick Add money or Rebalance, set an
 * amount and rules, review the proposed transactions and update the
 * portfolio. It is a demo of the Invest patterns driven by fixtures, so the
 * figures of a proposal are fixed per path.
 */
export function InvestComposition({
  mode = "simple",
  targets,
  onEditTargets,
  screen = "drift",
  strategy = "add-money",
  proposalState,
  feeDialogOpen = false,
  loadingDelayMs = 1000,
}: InvestCompositionProps) {
  const { portfolio, setPortfolioId } = useCompositionPortfolio();
  const [currentScreen, setCurrentScreen] = React.useState(screen);
  const [plan, setPlan] = React.useState<InvestmentPlan>({
    ...(strategy === "rebalance" ? rebalancePlan : addMoneyPlan),
    strategy,
  });
  const [amountText, setAmountText] = React.useState(
    defaultAmountText[strategy]
  );
  const [amountEdited, setAmountEdited] = React.useState(false);
  const [fees, setFees] = React.useState(defaultFees);
  const [feesOpen, setFeesOpen] = React.useState(feeDialogOpen);
  const targetDefinition =
    targets ??
    (proposalState === "blocked"
      ? blockedInvestTargets[mode]
      : investTargets[mode]);
  const targetCheck = checkTargetTotals(targetDefinition);
  // The drift view shows the same class targets and shares the gate checks.
  const strategicTargets =
    mode === "strategic" && targetDefinition.classTargets
      ? {
          classTargets: targetDefinition.classTargets,
          shares: Object.fromEntries(
            targetDefinition.assets.map((entry) => [entry.id, entry.weight])
          ),
        }
      : undefined;
  const blockedMessage = targetTotalsMessage(targetCheck);
  const [shownState, setShownState] = React.useState<ProposalState>(
    proposalState ?? (targetCheck.ok ? "ready" : "blocked")
  );
  const timer = React.useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined
  );
  const content = React.useRef<HTMLDivElement>(null);
  const firstRender = React.useRef(true);

  React.useEffect(() => {
    setCurrentScreen(screen);
  }, [screen]);
  React.useEffect(() => {
    setPlan((current) => ({ ...current, strategy }));
    setAmountText(defaultAmountText[strategy]);
    setAmountEdited(false);
  }, [strategy]);
  React.useEffect(() => {
    setShownState(proposalState ?? (targetCheck.ok ? "ready" : "blocked"));
  }, [proposalState, targetCheck.ok]);
  React.useEffect(() => {
    setFeesOpen(feeDialogOpen);
  }, [feeDialogOpen]);
  React.useEffect(() => () => clearTimeout(timer.current), []);
  // Moves focus to the new step so keyboard and screen-reader users follow it.
  React.useEffect(() => {
    if (firstRender.current) {
      firstRender.current = false;
      return;
    }
    content.current?.focus({ preventScroll: true });
  }, [currentScreen]);

  const amount = parseAmountInput(amountText);
  const scenario = scenarioFor(
    plan.strategy,
    amount,
    proposalState,
    mode,
    targetCheck.ok
  );
  const target = scenario.state;

  /** Shows the loading state, then the proposal state once the delay passes. */
  const calculate = () => {
    clearTimeout(timer.current);
    if (loadingDelayMs <= 0) {
      setShownState(target);
      return;
    }
    setShownState("loading");
    timer.current = setTimeout(() => setShownState(target), loadingDelayMs);
  };

  const startPath = (next: InvestmentStrategy) => {
    setPlan((current) => ({ ...current, strategy: next }));
    if (!amountEdited) setAmountText(defaultAmountText[next]);
    setCurrentScreen("plan");
  };

  const changeStrategy = (next: InvestmentPlan) => {
    setPlan(next);
    if (!amountEdited && next.strategy !== plan.strategy) {
      setAmountText(defaultAmountText[next.strategy]);
    }
  };

  const handleAmountText = (text: string) => {
    // Only digits and separators are kept, so negative amounts cannot be typed.
    setAmountText(text.replace(/[^0-9.,]/g, ""));
    setAmountEdited(true);
  };

  const useSuggested = () => {
    setAmountText(String(suggestedAmount));
    setAmountEdited(true);
  };

  const goToPlan = () => setCurrentScreen("plan");
  const goToDrift = () => setCurrentScreen("drift");

  const proceed = () => {
    if (!canContinue(amount, investDrift.portfolioValue).allowed) return;
    setCurrentScreen("proposal");
    calculate();
  };

  /** Clears the budget, as updating the portfolio spends it. */
  const resetBudget = () => {
    setAmountText(defaultAmountText[plan.strategy]);
    setAmountEdited(false);
  };

  const planAnother = () => {
    resetBudget();
    setCurrentScreen("plan");
  };

  const backToPortfolio = () => {
    resetBudget();
    goToDrift();
  };

  const copy = screenCopy[currentScreen];
  const feeSummary = describeFees(fees);
  const strategic = mode === "strategic";
  const rebalancing = plan.strategy === "rebalance";
  const result = strategic
    ? rebalancing
      ? rebalanceResultStrategic
      : addMoneyResultStrategic
    : rebalancing
      ? rebalanceResult
      : addMoneyResult;

  return (
    <PortfolioCompositionShell
      portfolio={portfolio}
      onPortfolioChange={(value) => {
        setPortfolioId(value);
        setCurrentScreen("drift");
      }}
      selectedItem="invest"
      // Steps with a sticky action bar carry their own actions, so only the
      // drift view offers the floating page-actions menu.
      pageActions={
        currentScreen === "drift"
          ? [
              {
                label: "Add money",
                icon: <Plus aria-hidden />,
                primary: true,
                disabled: !targetCheck.ok,
                onClick: () => startPath("add-money"),
              },
              {
                label: "Rebalance",
                icon: <ArrowLeftRight aria-hidden />,
                disabled: !targetCheck.ok,
                onClick: () => startPath("rebalance"),
              },
            ]
          : undefined
      }
      onSelectItem={(item) => {
        if (item.id === "invest") {
          setCurrentScreen("drift");
          return true;
        }
        return false;
      }}
    >
      <div
        ref={content}
        tabIndex={-1}
        className="ds-invest-composition"
        data-screen={currentScreen}
      >
        <PageHeader
          eyebrow={copy.eyebrow}
          title="Invest"
          description={`${portfolio.label} · ${copy.description}`}
          actions={
            currentScreen === "drift" ? (
              <InvestmentPathActions
                onAddMoney={() => startPath("add-money")}
                onRebalance={() => startPath("rebalance")}
                disabled={!targetCheck.ok}
              />
            ) : undefined
          }
        />
        {currentScreen === "drift" && blockedMessage ? (
          <Alert variant="destructive">
            <div>
              <AlertTitle>{blockedMessage.title}</AlertTitle>
              <AlertDescription>{blockedMessage.description}</AlertDescription>
              <Button variant="link" size="sm" onClick={onEditTargets}>
                Edit targets
              </Button>
            </div>
          </Alert>
        ) : null}
        {currentScreen === "drift" ? (
          <DriftOverview
            snapshot={investDrift}
            mode={mode}
            targets={strategicTargets}
          />
        ) : null}
        {currentScreen === "plan" ? (
          <InvestmentPlanForm
            plan={{ ...plan, amount: amount ?? 0 }}
            onPlanChange={changeStrategy}
            amountText={amountText}
            onAmountTextChange={handleAmountText}
            amount={amount}
            suggested={suggestedAmountStates.amount}
            onUseSuggested={useSuggested}
            drift={investDrift}
            mode={mode}
            commissions={{
              summary: feeSummary,
              policy: fees.policy,
              initialMaximumImpact: fees.values.maximumImpact,
              initialAmount: fees.values.amount,
              initialFeeRate: fees.values.feeRate,
              initialMinFee: fees.values.minFee,
              initialMaxFee: fees.values.maxFee,
              onSave: ({ policy, ...values }) => setFees({ policy, values }),
            }}
            feesOpen={feesOpen}
            onFeesOpenChange={setFeesOpen}
            onBack={goToDrift}
            onContinue={proceed}
          />
        ) : null}
        {currentScreen === "proposal" ? (
          <InvestmentProposal
            state={shownState}
            mode={mode}
            proposal={scenario.proposal}
            plan={{
              ...scenario.plan,
              wholeUnits: plan.wholeUnits,
              useAllBudget: plan.useAllBudget,
            }}
            commissionSummary={feeSummary}
            suggestedAmount={suggestedAmount}
            stateMessage={
              shownState === "ready" ||
              shownState === "loading" ||
              shownState === "fees-too-high" ||
              shownState === "price-too-high"
                ? undefined
                : shownState === "blocked"
                  ? (blockedMessage ?? undefined)
                  : proposalStateMessages[shownState]
            }
            onBack={goToPlan}
            onChangeRules={goToPlan}
            onUpdate={() => setCurrentScreen("updated")}
            onEditCommissions={() => {
              setFeesOpen(true);
              goToPlan();
            }}
            onUseSuggested={() => {
              useSuggested();
              goToPlan();
            }}
            onTryAgain={calculate}
            onBackToDrift={goToDrift}
            onEditTargets={onEditTargets}
          />
        ) : null}
        {currentScreen === "updated" ? (
          <InvestmentResult
            result={result}
            mode={mode}
            onBackToPortfolio={backToPortfolio}
            onPlanAnother={planAnother}
          />
        ) : null}
      </div>
    </PortfolioCompositionShell>
  );
}
