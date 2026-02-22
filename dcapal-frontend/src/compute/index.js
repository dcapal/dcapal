import { spawn, Thread, Worker } from "threads";
import { isAnalyzeRequest, isSolveRequest } from "./types";

const createWorkerState = () => ({
  instancePromise: null,
  queueTail: Promise.resolve(),
});

const analyzerState = createWorkerState();
const solverState = createWorkerState();

const enqueueSerialized = async (state, task) => {
  const run = () => task();
  state.queueTail = state.queueTail.then(run, run);
  return state.queueTail;
};

const getWorker = async (state, workerPath, workerName) => {
  if (state.instancePromise) return state.instancePromise;

  state.instancePromise = spawn(
    new Worker(new URL(workerPath, import.meta.url), {
      name: workerName,
    })
  ).catch((error) => {
    state.instancePromise = null;
    throw error;
  });

  return state.instancePromise;
};

const terminateWorkerPromise = async (workerPromise) => {
  if (!workerPromise) return;

  try {
    const worker = await workerPromise;
    await Thread.terminate(worker);
  } catch {
    // no-op: if worker failed to spawn/terminate there's nothing else to do
  }
};

const runWithWorker = async (state, workerPath, workerName, operation) => {
  return enqueueSerialized(state, async () => {
    const workerPromise = getWorker(state, workerPath, workerName);

    try {
      const worker = await workerPromise;
      return await operation(worker);
    } catch (error) {
      state.instancePromise = null;
      await terminateWorkerPromise(workerPromise);
      throw error;
    }
  });
};

/**
 * @param {import('./types').AnalyzeRequest} assets
 * @returns {Promise<import('./types').ComputeSolution>}
 */
export const analyze = async (assets) => {
  if (!isAnalyzeRequest(assets)) return null;

  try {
    return await runWithWorker(
      analyzerState,
      "./workers/analyzer.worker.js",
      "wasm-analyzer-worker",
      async (worker) => worker.analyzeAndSolve(assets)
    );
  } catch (error) {
    console.error("Unexpected exception in dcapal-optimizer:", error);
    return null;
  }
};

/**
 * @param {number} budget
 * @param {Record<string, Object>} assets
 * @param {string} pfolioCcy
 * @param {Object|null|undefined} fees
 * @param {boolean} isBuyOnly
 * @param {boolean} useAllBudget
 * @returns {Promise<import('./types').ComputeSolution>}
 */
export const solve = async (
  budget,
  assets,
  pfolioCcy,
  fees,
  isBuyOnly,
  useAllBudget
) => {
  const request = {
    budget,
    assets,
    pfolioCcy,
    fees,
    isBuyOnly,
    useAllBudget,
  };

  if (!isSolveRequest(request)) return null;

  try {
    return await runWithWorker(
      solverState,
      "./workers/solver.worker.js",
      "wasm-solver-worker",
      async (worker) =>
        worker.makeAndSolve(
          budget,
          assets,
          pfolioCcy,
          fees,
          isBuyOnly,
          useAllBudget
        )
    );
  } catch (error) {
    console.error("Unexpected exception in dcapal-optimizer:", error);
    return null;
  }
};

export const __resetComputeWorkersForTests = async () => {
  const analyzerPromise = analyzerState.instancePromise;
  const solverPromise = solverState.instancePromise;

  analyzerState.instancePromise = null;
  solverState.instancePromise = null;
  analyzerState.queueTail = Promise.resolve();
  solverState.queueTail = Promise.resolve();

  await Promise.all([
    terminateWorkerPromise(analyzerPromise),
    terminateWorkerPromise(solverPromise),
  ]);
};
