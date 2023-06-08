import React, { useEffect, useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { spawn, Thread, Worker } from "threads";
import { setAllocationFlowStep, Step } from "../../../app/appSlice";
import { roundAmount, roundDecimals, timeout } from "../../../utils";
import { Spinner } from "../../spinner/spinner";
import {
  ACLASS,
  clearBudget,
  isWholeShares,
} from "../portfolioStep/portfolioSlice";
import { AllocateCard } from "./allocateCard";

export const UNALLOCATED_CASH = "Unallocated cash";

const buildCards = (budget, assets, solution, pfolioCcy) => {
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
  }));

  if (!solution?.amounts) return cards;
  let totalAmount = 0;

  for (const a of solution.amounts.values()) {
    totalAmount += a;
  }

  for (const card of cards) {
    if (solution.amounts.has(card.symbol)) {
      card.amount = solution.amounts.get(card.symbol);
      card.weight = (100 * card.amount) / totalAmount;
    }

    if (solution.shares?.has(card.symbol)) {
      card.qty = solution.shares.get(card.symbol);
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
    });
  }

  return cards;
};

const buildProblemInput = (budget, pfolioAmount, assets, useWholeShares) => {
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

    return [problemBudget, as];
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
        },
      }),
      {}
    );

    return [budget, as];
  }
};

export const EndStep = ({ useTaxEfficient, useWholeShares }) => {
  const [solution, setSolution] = useState(null);
  const [isLoading, setIsLoading] = useState(true);
  const dispatch = useDispatch();

  const [budget, pfolioAmount, assets, quoteCcy] = useSelector((state) => {
    return [
      state.pfolio.budget,
      state.pfolio.totalAmount,
      state.pfolio.assets,
      state.pfolio.quoteCcy,
    ];
  });

  const cards = solution ? buildCards(budget, assets, solution, quoteCcy) : [];

  useEffect(() => {
    const launchSolver = async () => {
      const solver = await spawn(
        new Worker(new URL("../../../workers/solver.js", import.meta.url), {
          name: "wasm-solver-worker",
        })
      );

      const [inputBudget, as] = buildProblemInput(
        budget,
        pfolioAmount,
        assets,
        useWholeShares
      );

      const sol = await solver.makeAndSolve(
        inputBudget,
        as,
        quoteCcy,
        useTaxEfficient,
        useWholeShares
      );
      await Thread.terminate(solver);
      return sol;
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
            Hang on! We are allocating your budget
          </div>
          <Spinner />
        </>
      )}
      {!isLoading && solution && (
        <>
          <div className="mt-2 mb-8 text-3xl font-light">
            <span className="text-4xl">📊</span> Great Success! Your allocation
            is ready
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
              />
            ))}
          </div>
          <span
            className="mt-6 font-medium underline cursor-pointer"
            onClick={onClickGoBack}
          >
            Back to Portfolio
          </span>
        </>
      )}
      {!isLoading && !solution && (
        <>
          <div className="mt-2 mb-8 text-3xl font-light">
            <span className="text-4xl">⚠️</span> Oops! Something bad happened.
            Please review your portfolio.
          </div>
          <span
            className="mt-6 font-medium underline cursor-pointer"
            onClick={onClickGoBack}
          >
            Back to Portfolio
          </span>
        </>
      )}
    </div>
  );
};
