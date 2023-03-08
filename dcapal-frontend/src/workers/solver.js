import { import_wasm } from "../wasm/import_wasm";
import { expose } from "threads";

expose({
  async makeAndSolve(budget, assets, isBuyOnly) {
    const { Solver } = await import_wasm();

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
