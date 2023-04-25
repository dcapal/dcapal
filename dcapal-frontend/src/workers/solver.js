import { Solver } from "dcapal-optimizer-wasm";
import { expose } from "threads/worker";

expose({
  async makeAndSolve(budget, assets, isBuyOnly) {
    if (!budget || budget < 0) return null;
    if (!assets || Object.keys(assets).length === 0) return null;

    const handle = Solver.build_problem({
      budget: budget,
      assets: {
        ...assets,
      },
      is_buy_only: isBuyOnly,
    });

    return Solver.solve(handle);
  },
});
