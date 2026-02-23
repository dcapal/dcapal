import { useEffect, useState } from "react";
import { replacer } from "@utils/index.js";
import { isWholeShares } from "@components/allocationFlow/portfolioSlice";

const buildProblemInput = (assets) => {
  return Object.values(assets).reduce(
    (as, a) => ({
      ...as,
      [a.symbol]: {
        symbol: a.symbol,
        target_weight: a.targetWeight / 100,
        shares: a.qty,
        price: a.price,
        is_whole_shares: isWholeShares(a.aclass),
        current_amount: a.amount,
      },
    }),
    {}
  );
};

export const useSuggestAmountToInvest = ({
  assets,
  totalAmount,
  isWorkerReady,
  worker,
}) => {
  const [solution, setSolution] = useState(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    if (totalAmount === 0) {
      setIsLoading(false);
      setSolution(null);
      return;
    }

    if (!isWorkerReady) {
      setIsLoading(true);
      return;
    }

    let isCancelled = false;

    const runAnalyze = async () => {
      setIsLoading(true);
      const input = buildProblemInput(assets);
      const sol = await worker.suggestAmountToInvest(input);
      if (isCancelled) return;

      console.debug(`solution=${JSON.stringify(sol, replacer)}`);
      setIsLoading(false);
      setSolution(sol);
    };

    runAnalyze();

    return () => {
      isCancelled = true;
    };
  }, [assets, isWorkerReady, worker, totalAmount]);

  return {
    solution,
    isLoading,
  };
};
