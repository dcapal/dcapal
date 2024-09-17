import { Solver } from "dcapal-optimizer-wasm";
import { expose } from "threads/worker";

const buildProblemInput = (assets) => {
  return {
    type: "analyze",
    assets: {
      ...assets,
    },
  };
};

const buildAbpProblemInput = (assets, investmentAmount) => {
  return {
    type: "averageBuyPrice",
    assets: {
      ...assets,
    },
    investment_amount: investmentAmount,
  };
};

expose({
  async analyzeAndSolve(assets) {
    if (!assets || Object.keys(assets).length === 0) return null;

    const input = buildProblemInput(assets);

    const handle = Solver.build_problem(input);
    const solution = Solver.solve(handle);
    if (!Solver.delete_problem(handle)) {
      console.error(`Failed to delete solved problem (handle=${handle})`);
    }

    return solution;
  },
  async analyzeAndSuggest(assets, investmentAmount) {
    if (!assets || Object.keys(assets).length === 0) return null;

    const input = buildAbpProblemInput(assets, investmentAmount);

    const handle = Solver.build_problem(input);
    const solution = Solver.solve(handle);
    if (!Solver.delete_problem(handle)) {
      console.error(`Failed to delete solved problem (handle=${handle})`);
    }

    return solution;
  },
});
