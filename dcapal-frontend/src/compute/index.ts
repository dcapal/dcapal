import { spawn, Thread, Worker } from "threads";
import {
  type AnalyzeRequest,
  type ComputeSolution,
  type SolveAssets,
  type TransactionFeesInput,
  isAnalyzeRequest,
  isSolveRequest,
} from "./types";

interface AnalyzerWorkerRpc {
  analyzeAndSolve(assets: AnalyzeRequest): Promise<ComputeSolution>;
}

interface SolverWorkerRpc {
  makeAndSolve(
    budget: number,
    assets: SolveAssets,
    pfolioCcy: string,
    fees: TransactionFeesInput | null | undefined,
    isBuyOnly: boolean,
    useAllBudget: boolean
  ): Promise<ComputeSolution>;
}

type WorkerRpc = AnalyzerWorkerRpc | SolverWorkerRpc;

interface WorkerState<TWorker extends WorkerRpc> {
  instancePromise: Promise<TWorker> | null;
  queueTail: Promise<unknown>;
}

const createWorkerState = <TWorker extends WorkerRpc>(): WorkerState<TWorker> => ({
  instancePromise: null,
  queueTail: Promise.resolve(),
});

const analyzerState = createWorkerState<AnalyzerWorkerRpc>();
const solverState = createWorkerState<SolverWorkerRpc>();

const enqueueSerialized = async <TWorker extends WorkerRpc, TResult>(
  state: WorkerState<TWorker>,
  task: () => Promise<TResult>
): Promise<TResult> => {
  const run = (): Promise<TResult> => task();
  state.queueTail = state.queueTail.then(run, run);
  return state.queueTail as Promise<TResult>;
};

const getWorker = async <TWorker extends WorkerRpc>(
  state: WorkerState<TWorker>,
  workerPath: string,
  workerName: string
): Promise<TWorker> => {
  if (state.instancePromise) return state.instancePromise;

  // TODO(migrate): threads' constructor types only accept string paths, but webpack worker loading uses URL.
  const workerUrl = new URL(workerPath, import.meta.url) as unknown as string;
  state.instancePromise = (spawn(
    new Worker(workerUrl, {
      name: workerName,
    })
  ) as unknown as Promise<TWorker>).catch((error: unknown) => {
    state.instancePromise = null;
    throw error;
  });

  return state.instancePromise;
};

const terminateWorkerPromise = async <TWorker extends WorkerRpc>(
  workerPromise: Promise<TWorker> | null
): Promise<void> => {
  if (!workerPromise) return;

  try {
    const worker = await workerPromise;
    // TODO(migrate): narrow spawned thread proxy type from threads to avoid this cast.
    await Thread.terminate(worker as unknown as Parameters<
      typeof Thread.terminate
    >[0]);
  } catch {
    // no-op: if worker failed to spawn/terminate there's nothing else to do
  }
};

const runWithWorker = async <TWorker extends WorkerRpc, TResult>(
  state: WorkerState<TWorker>,
  workerPath: string,
  workerName: string,
  operation: (worker: TWorker) => Promise<TResult>
): Promise<TResult> => {
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

export const analyze = async (
  assets: AnalyzeRequest
): Promise<ComputeSolution> => {
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

export const solve = async (
  budget: number,
  assets: SolveAssets,
  pfolioCcy: string,
  fees: TransactionFeesInput | null | undefined,
  isBuyOnly: boolean,
  useAllBudget: boolean
): Promise<ComputeSolution> => {
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

export const __resetComputeWorkersForTests = async (): Promise<void> => {
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
