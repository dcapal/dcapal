import { import_wasm } from "../wasm/import_wasm";
import { expose } from "threads";

expose({
  async makeAndSolve(budget, assets) {
    const { Solver } = await import_wasm();

    if (!budget || budget < 0) return null;
    if (!assets || Object.keys(assets).length === 0) return null;

    const handle = Solver.build_problem({
      budget: budget,
      assets: {
        ...assets,
      },
    });

    return Solver.solve(handle);
  },
});
