import { Solver } from "dcapal-optimizer-wasm";
import { expose } from "threads/worker";

const buildProblemInput = (
  budget,
  assets,
  pfolioCcy,
  isBuyOnly,
  isAdvancedAlgorithm,
  fees
) => {
  if (isAdvancedAlgorithm) {
    return {
      type: "analyze",
      budget: budget,
      pfolio_ccy: pfolioCcy,
      assets: {
        ...assets,
      },
      is_buy_only: isBuyOnly,
      fees: fees,
    };
  } else {
    return {
      type: "analyze",
      budget: budget,
      assets: {
        ...assets,
      },
      is_buy_only: isBuyOnly,
    };
  }
};

expose({
  async makeAndSolve(
    budget,
    assets,
    pfolioCcy,
    isBuyOnly,
    isAdvancedAlgorithm,
    fees
  ) {
    if (Number.isNaN(budget) || budget < 0) return null;
    if (!assets || Object.keys(assets).length === 0) return null;

    const input = buildProblemInput(
      budget,
      assets,
      pfolioCcy,
      isBuyOnly,
      isAdvancedAlgorithm,
      fees
    );

    const handle = Solver.build_problem(input);

    return Solver.solve(handle);
  },
});
