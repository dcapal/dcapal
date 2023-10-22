import { Solver } from "dcapal-optimizer-wasm";
import { expose } from "threads/worker";

const buildProblemInput = (assets, isSuggestionRequested) => {
  if (isSuggestionRequested) {
    return {
      type: "analyze",
      assets: {
        ...assets,
      },
    };
  }
};

expose({
  async analyzeAndSolve(assets) {
    if (!assets || Object.keys(assets).length === 0) return null;

    const input = buildProblemInput(assets, true);

    const handle = Solver.build_problem(input);

    return Solver.solve(handle);
  },
});
