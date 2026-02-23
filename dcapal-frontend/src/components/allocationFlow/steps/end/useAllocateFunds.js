import { useEffect, useState } from "react";
import { replacer } from "@utils/index.js";
import {
  feeTypeToString,
  isWholeShares,
} from "@components/allocationFlow/portfolioSlice";

const buildFeesInput = (fees) => {
  if (!fees) {
    return null;
  }

  const input = {
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

const buildProblemInput = (budget, assets, fees, useWholeShares) => {
  const as = Object.values(assets).reduce(
    (acc, a) => ({
      ...acc,
      [a.symbol]: {
        symbol: a.symbol,
        shares: a.qty,
        price: a.price,
        target_weight: a.targetWeight / 100,
        is_whole_shares: useWholeShares ? isWholeShares(a.aclass) : false,
        fees: buildFeesInput(a.fees),
      },
    }),
    {}
  );

  return [budget, as, buildFeesInput(fees)];
};

export const useAllocateFunds = ({
  budget,
  assets,
  fees,
  quoteCcy,
  useTaxEfficient,
  useAllBudget,
  useWholeShares,
  isWorkerReady,
  worker,
}) => {
  const [solution, setSolution] = useState(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    if (!isWorkerReady) {
      setIsLoading(true);
      return;
    }

    let isCancelled = false;

    const loadSolution = async () => {
      setIsLoading(true);

      const [inputBudget, as, inputFees] = buildProblemInput(
        budget,
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

      const sol = await worker.solve(
        inputBudget,
        as,
        quoteCcy,
        inputFees,
        useTaxEfficient,
        useAllBudget
      );
      if (isCancelled) return;

      console.debug(`solution=${JSON.stringify(sol, replacer)}`);
      setIsLoading(false);
      setSolution(sol);
    };

    loadSolution();

    return () => {
      isCancelled = true;
    };
  }, [
    assets,
    budget,
    fees,
    isWorkerReady,
    quoteCcy,
    worker,
    useAllBudget,
    useTaxEfficient,
    useWholeShares,
  ]);

  return {
    solution,
    isLoading,
  };
};
