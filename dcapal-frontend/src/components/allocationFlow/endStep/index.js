import React, { useEffect, useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { spawn, Thread, Worker } from "threads";
import { setAllocationFlowStep, Step } from "../../../app/appSlice";
import { replacer, roundAmount, roundDecimals, timeout } from "../../../utils";
import { Spinner } from "../../spinner/spinner";
import {
  ACLASS,
  FeeType,
  clearBudget,
  feeTypeToString,
  isWholeShares,
} from "../portfolioStep/portfolioSlice";
import { AllocateCard } from "./allocateCard";
import { useTranslation } from "react-i18next";

export const UNALLOCATED_CASH = "Unallocated cash";

const buildCards = (assets, solution, pfolioCcy, pfolioFees) => {
  const cards = Object.values(assets).map((a) => ({
    symbol: a.symbol,
    name: a.name,
    aclass: a.aclass,
    qty: -1,
    oldQty: a.qty,
    price: a.price,
    amount: 0,
    oldAmount: a.amount,
    weight: 0,
    oldWeight: a.weight,
    targetWeight: a.targetWeight,
    fees: a.fees ? a.fees : pfolioFees,
    theoAlloc: null,
  }));

  if (!solution?.amounts) return cards;

  let totalAmount = solution.budget_left || 0;
  for (const a of solution.amounts.values()) {
    totalAmount += a;
  }

  for (const card of cards) {
    if (solution.amounts.has(card.symbol)) {
      card.amount = solution.amounts.get(card.symbol);
      card.weight = (100 * card.amount) / totalAmount;
    }

    if (solution?.shares?.has(card.symbol)) {
      card.qty = solution.shares.get(card.symbol);
    }

    if (solution?.theo_allocs?.has(card.symbol)) {
      card.theoAlloc = solution.theo_allocs.get(card.symbol);
    }
  }

  if (solution.budget_left) {
    cards.push({
      symbol: pfolioCcy,
      name: UNALLOCATED_CASH,
      aclass: ACLASS.CURRENCY,
      qty: -1,
      oldQty: 0,
      price: 0,
      amount: solution.budget_left,
      oldAmount: 0,
      weight: 0,
      oldWeight: 0,
      targetWeight: 0,
      fees: null,
      theo_alloc: null,
    });
  }

  return cards;
};

const buildFeesInput = (fees) => {
  if (!fees) {
    return null;
  }

  let input = {
    ...fees,
    feeStructure: {
      ...fees.feeStructure,
      type: feeTypeToString(fees.feeStructure.type),
    },
  };

  if (input.maxFeeImpact == null) {
    delete input.maxFeeImpact;
  } else if (input.maxFeeImpact) {
    input.maxFeeImpact /= 100;
  }

  if (input.feeStructure.feeRate == null) {
    delete input.feeStructure.feeRate;
  } else if (input.feeStructure.feeRate) {
    input.feeStructure.feeRate /= 100;
  }

  return input;
};

const buildProblemInput = (
  budget,
  pfolioAmount,
  assets,
  fees,
  useWholeShares
) => {
  if (!useWholeShares) {
    const problemBudget = budget + pfolioAmount;
    const as = Object.values(assets).reduce(
      (as, a) => ({
        ...as,
        [a.symbol]: {
          symbol: a.symbol,
          target_weight: a.targetWeight / 100,
          current_amount: a.amount,
        },
      }),
      {}
    );

    return [problemBudget, as, buildFeesInput(fees)];
  } else {
    const as = Object.values(assets).reduce(
      (as, a) => ({
        ...as,
        [a.symbol]: {
          symbol: a.symbol,
          shares: a.qty,
          price: a.price,
          target_weight: a.targetWeight / 100,
          is_whole_shares: isWholeShares(a.aclass),
          fees: buildFeesInput(a.fees),
        },
      }),
      {}
    );

    return [budget, as, buildFeesInput(fees)];
  }
};

export const EndStep = ({ useTaxEfficient, useWholeShares }) => {
  const [solution, setSolution] = useState(null);
  const [isLoading, setIsLoading] = useState(true);
  const dispatch = useDispatch();

  const { t } = useTranslation();
  const budget = useSelector((state) => state.pfolio.budget);
  const pfolioAmount = useSelector((state) => state.pfolio.totalAmount);
  const assets = useSelector((state) => state.pfolio.assets);
  const quoteCcy = useSelector((state) => state.pfolio.quoteCcy);
  const fees = useSelector((state) => state.pfolio.fees);

  const cards = solution ? buildCards(assets, solution, quoteCcy, fees) : [];

  useEffect(() => {
    const launchSolver = async () => {
      const solver = await spawn(
        new Worker(new URL("../../../workers/solver.js", import.meta.url), {
          name: "wasm-solver-worker",
        })
      );

      const [inputBudget, as, inputFees] = buildProblemInput(
        budget,
        pfolioAmount,
        assets,
        fees,
        useWholeShares
      );

      console.debug(
        `inputBudget=${inputBudget} as=${JSON.stringify(
          as
        )} quoteCcy=${quoteCcy} useTaxEfficient=${useTaxEfficient} useWholeShares=${useWholeShares} inputFees=${JSON.stringify(
          inputFees
        )}`
      );

      try {
        const sol = await solver.makeAndSolve(
          inputBudget,
          as,
          quoteCcy,
          useTaxEfficient,
          useWholeShares,
          inputFees
        );

        await Thread.terminate(solver);

        console.debug(`solution=${JSON.stringify(sol, replacer)}`);

        return sol;
      } catch (error) {
        console.error("Unexpected exception in dcapal-optimizer:", error);
        return null;
      }
    };

    const solve = async () => {
      const [sol] = await Promise.all([launchSolver(), timeout(1000)]);
      setIsLoading(false);
      if (sol) {
        setSolution(sol);
      }
    };

    solve();
  }, []);

  const onClickGoBack = () => {
    dispatch(clearBudget({}));
    dispatch(setAllocationFlowStep({ step: Step.PORTFOLIO }));
  };

  return (
    <div className="w-full h-full flex flex-col items-center">
      {isLoading && (
        <>
          <div className="mt-2 mb-8 text-3xl font-light">
            {t("endStep.budgetAllocated")}
          </div>
          <Spinner />
        </>
      )}
      {!isLoading && solution && (
        <>
          <div className="mt-2 mb-8 text-3xl font-light">
            <span className="text-4xl">📊</span> {t("endStep.allocationReady")}
          </div>
          <div className="w-full flex flex-col items-center">
            {cards.map((c) => (
              <AllocateCard
                key={c.symbol}
                symbol={c.symbol}
                name={c.name}
                aclass={c.aclass}
                qty={c.qty}
                oldQty={c.oldQty}
                price={c.price}
                amount={roundAmount(c.amount)}
                oldAmount={roundAmount(c.oldAmount)}
                weight={c.weight}
                oldWeight={c.oldWeight}
                targetWeight={c.targetWeight}
                fees={c.fees}
                theoAlloc={c.theoAlloc}
              />
            ))}
          </div>
          <span
            className="mt-6 font-medium underline cursor-pointer"
            onClick={onClickGoBack}
          >
            {t("endStep.backToPortfolio")}
          </span>
        </>
      )}
      {!isLoading && !solution && (
        <>
          <div className="mt-2 mb-8 text-3xl font-light">
            <span className="text-4xl">⚠️</span> {t("endStep.opsBadHappened")}
            {t("endStep.reviewPortfolio")}
          </div>
          <span
            className="mt-6 font-medium underline cursor-pointer"
            onClick={onClickGoBack}
          >
            {t("endStep.backToPortfolio")}
          </span>
        </>
      )}
    </div>
  );
};
