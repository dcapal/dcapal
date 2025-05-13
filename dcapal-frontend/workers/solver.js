import { Solver } from "dcapal-optimizer-wasm";
import { expose } from "threads/worker";

const buildProblemInput = (
  budget,
  assets,
  pfolio_ccy,
  fees,
  is_buy_only,
  use_all_budget
) => ({
  type: "advanced",
  budget,
  pfolio_ccy,
  assets,
  is_buy_only,
  use_all_budget,
  fees,
});

expose({
  async makeAndSolve(budget, assets, pfolioCcy, fees, isBuyOnly, useAllBudget) {
    if (Number.isNaN(budget) || budget < 0) return null;
    if (!assets || Object.keys(assets).length === 0) return null;

    const input = buildProblemInput(
      budget,
      assets,
      pfolioCcy,
      fees,
      isBuyOnly,
      useAllBudget
    );

    const handle = Solver.build_problem(input);
    const solution = Solver.solve(handle);
    if (!Solver.delete_problem(handle))
      console.error(`Failed to delete solved problem (handle=${handle})`);

    return solution;
  },
});
