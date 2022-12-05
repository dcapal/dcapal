import React, { useEffect, useState } from "react";
import { useDispatch, useSelector } from "react-redux";
import { spawn, Thread, Worker } from "threads";
import { Step } from "..";
import { setAllocationFlowStep } from "../../../app/appSlice";
import { roundAmount, timeout } from "../../../utils";
import { Spinner } from "../../spinner/spinner";
import { clearBudget } from "../portfolioStep/portfolioSlice";
import { AllocateCard } from "./allocateCard";

const buildCards = (assets, solution) => {
  const cards = Object.values(assets).map((a) => ({
    symbol: a.symbol,
    name: a.name,
    amount: 0,
    oldAmount: a.amount,
    weight: 0,
    oldWeight: a.weight,
    targetWeight: a.targetWeight,
  }));

  if (!solution?.vars) return cards;
  let totalAmount = 0;

  for (const a of solution.vars.values()) {
    totalAmount += a;
  }

  for (const card of cards) {
    if (solution.vars.has(card.symbol)) {
      card.amount = solution.vars.get(card.symbol);
      card.weight = (100 * card.amount) / totalAmount;
    }
  }

  return cards;
};

export const EndStep = ({ ...props }) => {
  const [solution, setSolution] = useState(null);
  const [isLoading, setIsLoading] = useState(true);
  const dispatch = useDispatch();

  const [budget, assets] = useSelector((state) => {
    return [
      state.pfolio.budget + state.pfolio.totalAmount,
      state.pfolio.assets,
    ];
  });

  const cards = solution ? buildCards(assets, solution) : [];

  useEffect(() => {
    const launchSolver = async () => {
      const solver = await spawn(
        new Worker("../../../workers/solver", { name: "wasm-solver-worker" })
      );
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

      const sol = await solver.makeAndSolve(budget, as);
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
            <span className="text-4xl">📈</span> Great Success! Your allocation
            is ready
          </div>
          <div className="grow">
            {cards.map((c) => (
              <AllocateCard
                key={c.symbol}
                symbol={c.symbol}
                name={c.name}
                amount={roundAmount(c.amount)}
                oldAmount={roundAmount(c.oldAmount)}
                weight={c.weight}
                oldWeight={c.oldWeight}
                targetWeight={c.targetWeight}
              />
            ))}
          </div>
          <span
            className="mt-2 font-medium underline cursor-pointer"
            onClick={onClickGoBack}
          >
            Back to Portfolio
          </span>
        </>
      )}
    </div>
  );
};
