import { expose } from "comlink";
import {
  Solver,
  __wbg_set_wasm,
} from "dcapal-optimizer-wasm/dcapal_optimizer_wasm_bg.js";
import wasmUrl from "dcapal-optimizer-wasm/dcapal_optimizer_wasm_bg.wasm?url";

let solverModulePromise = null;

const getSolver = async () => {
  if (!solverModulePromise) {
    solverModulePromise = (async () => {
      const loadViaStreaming = async () => {
        if (typeof WebAssembly.instantiateStreaming !== "function") {
          return null;
        }

        try {
          const response = await fetch(wasmUrl);
          return await WebAssembly.instantiateStreaming(response, {});
        } catch {
          return null;
        }
      };

      const loadViaBuffer = async () => {
        const response = await fetch(wasmUrl);
        const wasmBytes = await response.arrayBuffer();
        return await WebAssembly.instantiate(wasmBytes, {});
      };

      const result = (await loadViaStreaming()) ?? (await loadViaBuffer());
      __wbg_set_wasm(result.instance.exports);
      result.instance.exports.__wbindgen_start();
      return Solver;
    })().catch((error) => {
      solverModulePromise = null;
      throw error;
    });
  }

  return solverModulePromise;
};

const buildAnalyzeProblemInput = (assets) => ({
  type: "analyze",
  assets: {
    ...assets,
  },
});

const buildAdvancedProblemInput = (
  budget,
  assets,
  pfolioCcy,
  fees,
  isBuyOnly,
  useAllBudget
) => ({
  type: "advanced",
  budget: budget,
  pfolio_ccy: pfolioCcy,
  assets: {
    ...assets,
  },
  is_buy_only: isBuyOnly,
  use_all_budget: useAllBudget,
  fees: fees,
});

const solve = (Solver, input) => {
  const handle = Solver.build_problem(input);
  const solution = Solver.solve(handle);
  if (!Solver.delete_problem(handle)) {
    console.error(`Failed to delete solved problem (handle=${handle})`);
  }

  return solution;
};

expose({
  async ping() {
    return "pong";
  },
  async init() {
    await getSolver();
  },
  async analyzeAndSolve(assets) {
    if (!assets || Object.keys(assets).length === 0) return null;

    const Solver = await getSolver();
    const input = buildAnalyzeProblemInput(assets);
    return solve(Solver, input);
  },
  async makeAndSolve(budget, assets, pfolioCcy, fees, isBuyOnly, useAllBudget) {
    if (Number.isNaN(budget) || budget < 0) return null;
    if (!assets || Object.keys(assets).length === 0) return null;

    const Solver = await getSolver();
    const input = buildAdvancedProblemInput(
      budget,
      assets,
      pfolioCcy,
      fees,
      isBuyOnly,
      useAllBudget
    );
    return solve(Solver, input);
  },
});
