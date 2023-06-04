import { Solver } from "dcapal-optimizer-wasm";
import { expose } from "threads/worker";

const buildProblemInput = (
  budget,
  assets,
  pfolioCcy,
  isBuyOnly,
  isAdvancedAlgorithm
) => {
  if (isAdvancedAlgorithm) {
    return {
      budget: budget,
      pfolio_ccy: pfolioCcy,
      assets: {
        ...assets,
      },
      is_buy_only: isBuyOnly,
    };
  } else {
    return {
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
    isAdvancedAlgorithm
  ) {
    if (!budget || budget < 0) return null;
    if (!assets || Object.keys(assets).length === 0) return null;

    const input = buildProblemInput(
      budget,
      assets,
      pfolioCcy,
      isBuyOnly,
      isAdvancedAlgorithm
    );

    const handle = Solver.build_problem(input);

    return Solver.solve(handle);
  },
});
