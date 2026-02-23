import { useCallback, useEffect, useMemo, useState } from "react";
import { wrap, type Remote } from "comlink";
import {
  type AllocationResult,
  type AnalyzeRequest,
  type AnalyzeResult,
  type SolveAssets,
  type TransactionFeesInput,
  isAnalyzeRequest,
  isSolveRequest,
} from "./types";

interface AnalyzerWorkerRpc {
  ping(): Promise<string>;
  init(): Promise<void>;
  analyzeAndSolve(assets: AnalyzeRequest): Promise<AnalyzeResult>;
}

interface PortfolioOptimizerWorkerRpc extends AnalyzerWorkerRpc {
  makeAndSolve(
    budget: number,
    assets: SolveAssets,
    pfolioCcy: string,
    fees: TransactionFeesInput | null | undefined,
    isBuyOnly: boolean,
    useAllBudget: boolean
  ): Promise<AllocationResult>;
}

type ComputeWorkerState<TWorker extends object> = {
  worker: Worker | null;
  proxy: Remote<TWorker> | null;
  initPromise: Promise<void> | null;
  createPromise: Promise<Remote<TWorker>> | null;
  onMessage: ((event: MessageEvent) => void) | null;
  onError: ((event: ErrorEvent) => void) | null;
  onMessageError: ((event: MessageEvent) => void) | null;
};

const createWorkerState = <TWorker extends object>(): ComputeWorkerState<TWorker> => ({
  worker: null,
  proxy: null,
  initPromise: null,
  createPromise: null,
  onMessage: null,
  onError: null,
  onMessageError: null,
});

const portfolioOptimizerState = createWorkerState<PortfolioOptimizerWorkerRpc>();

const createPortfolioOptimizerWorkerInstance = (): Worker => {
  return new Worker(
    new URL("./workers/portfolioOptimizer.worker.js?v=diag", import.meta.url)
  );
};

const destroyWorkerState = async <TWorker extends object>(
  state: ComputeWorkerState<TWorker>
): Promise<void> => {
  const worker = state.worker;
  const onMessage = state.onMessage;
  const onError = state.onError;
  const onMessageError = state.onMessageError;
  state.worker = null;
  state.proxy = null;
  state.initPromise = null;
  state.createPromise = null;
  state.onMessage = null;
  state.onError = null;
  state.onMessageError = null;

  if (!worker) return;

  if (onMessage && typeof worker.removeEventListener === "function") {
    worker.removeEventListener("message", onMessage);
  }
  if (onError && typeof worker.removeEventListener === "function") {
    worker.removeEventListener("error", onError);
  }
  if (onMessageError && typeof worker.removeEventListener === "function") {
    worker.removeEventListener("messageerror", onMessageError);
  }

  try {
    worker.terminate();
  } catch {
    // no-op
  }
};

const PING_TIMEOUT_MS = 4_000;
const INIT_TIMEOUT_MS = 15_000;

const withTimeout = async <T>(
  promise: Promise<T>,
  timeoutMs: number,
  label: string
): Promise<T> => {
  let timeoutId: ReturnType<typeof setTimeout> | null = null;
  const timeoutPromise = new Promise<never>((_, reject) => {
    timeoutId = setTimeout(() => {
      reject(new Error(`${label} timed out after ${timeoutMs}ms`));
    }, timeoutMs);
  });

  try {
    return await Promise.race([promise, timeoutPromise]);
  } finally {
    if (timeoutId) clearTimeout(timeoutId);
  }
};

const getOrCreateWorker = async <Rpc extends { ping(): Promise<string>; init(): Promise<void> }>(
  state: ComputeWorkerState<Rpc>,
  createWorker: () => Worker,
  workerName: string
): Promise<Remote<Rpc>> => {
  if (state.proxy && state.initPromise) {
    await state.initPromise;
    return state.proxy;
  }

  if (!state.createPromise) {
    state.createPromise = (async () => {
      if (!state.worker || !state.proxy) {
        state.worker = createWorker();
        state.onMessage = (event: MessageEvent) => {
          console.info(`[${workerName}] worker message event`, event.data);
        };
        state.onError = (event: ErrorEvent) => {
          console.error(`[${workerName}] worker error event`, event);
        };
        state.onMessageError = (event: MessageEvent) => {
          console.error(`[${workerName}] worker messageerror event`, event);
        };
        if (typeof state.worker.addEventListener === "function") {
          state.worker.addEventListener("message", state.onMessage);
          state.worker.addEventListener("error", state.onError);
          state.worker.addEventListener("messageerror", state.onMessageError);
        }
        state.proxy = wrap<Rpc>(state.worker);
      }

      const proxy = state.proxy;
      if (!proxy) {
        throw new Error(`${workerName} worker proxy is not available`);
      }

      if (!state.initPromise) {
        state.initPromise = (async () => {
          const ping = await withTimeout(
            proxy.ping(),
            PING_TIMEOUT_MS,
            `${workerName} ping`
          );
          console.info(`[compute] ${workerName} worker ping=${ping}`);
          await withTimeout(proxy.init(), INIT_TIMEOUT_MS, `${workerName} init`);
        })().catch(async (error: unknown) => {
          await destroyWorkerState(state);
          throw error;
        });
      }

      await state.initPromise;
      return state.proxy;
    })().finally(() => {
      state.createPromise = null;
    });
  }

  return state.createPromise;
};

const runWithRetry = async <
  Rpc extends { ping(): Promise<string>; init(): Promise<void> },
  Result
>(
  getWorker: () => Promise<Remote<Rpc>>,
  state: ComputeWorkerState<Rpc>,
  operation: (worker: Remote<Rpc>) => Promise<Result>
): Promise<Result> => {
  try {
    const worker = await getWorker();
    return await operation(worker);
  } catch {
    await destroyWorkerState(state);
    const worker = await getWorker();
    return await operation(worker);
  }
};

const toError = (error: unknown): Error => {
  if (error instanceof Error) return error;
  return new Error(String(error));
};

const describeUnknownError = (error: unknown): string => {
  if (error instanceof Error) {
    const message = error.message ? error.message : "<empty message>";
    return `${error.name}: ${message}`;
  }

  if (typeof error === "string") return error;
  return String(error);
};

const getPortfolioOptimizerWorker = async (): Promise<
  Remote<PortfolioOptimizerWorkerRpc>
> => {
  return getOrCreateWorker(
    portfolioOptimizerState,
    createPortfolioOptimizerWorkerInstance,
    "portfolio-optimizer"
  );
};

const initializeWorkers = async (): Promise<void> => {
  const initializePortfolioOptimizer = getPortfolioOptimizerWorker()
    .then(() => {
      console.info("[compute] portfolio optimizer worker initialized");
    })
    .catch((error: unknown) => {
      const normalizedError = toError(error);
      console.error(
        "[compute] portfolio optimizer worker initialization failed:",
        describeUnknownError(error),
        normalizedError
      );
      throw normalizedError;
    });

  await initializePortfolioOptimizer;
};

const analyzeWithWorker = async (
  assets: AnalyzeRequest
): Promise<AnalyzeResult> => {
  if (!isAnalyzeRequest(assets)) return null;

  return runWithRetry<PortfolioOptimizerWorkerRpc, AnalyzeResult>(
    getPortfolioOptimizerWorker,
    portfolioOptimizerState,
    async (worker) => worker.analyzeAndSolve(assets)
  );
};

const solveWithWorker = async (
  budget: number,
  assets: SolveAssets,
  pfolioCcy: string,
  fees: TransactionFeesInput | null | undefined,
  isBuyOnly: boolean,
  useAllBudget: boolean
): Promise<AllocationResult> => {
  const request = {
    budget,
    assets,
    pfolioCcy,
    fees,
    isBuyOnly,
    useAllBudget,
  };

  if (!isSolveRequest(request)) return null;

  return runWithRetry<PortfolioOptimizerWorkerRpc, AllocationResult>(
    getPortfolioOptimizerWorker,
    portfolioOptimizerState,
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
};

export interface ComputeWorkerStatus {
  isLoading: boolean;
  isReady: boolean;
  error: Error | null;
}

export interface ComputeWorkerApi {
  suggestAmountToInvest: (assets: AnalyzeRequest) => Promise<AnalyzeResult>;
  solve: (
    budget: number,
    assets: SolveAssets,
    pfolioCcy: string,
    fees: TransactionFeesInput | null | undefined,
    isBuyOnly: boolean,
    useAllBudget: boolean
  ) => Promise<AllocationResult>;
}

export const useComputeWorker = (): [ComputeWorkerStatus, ComputeWorkerApi] => {
  const [isLoading, setIsLoading] = useState(true);
  const [isReady, setIsReady] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let isCancelled = false;

    const init = async () => {
      try {
        if (typeof Blob !== "undefined" && typeof URL.createObjectURL === "function") {
          const probeBlob = new Blob(
            ['self.onmessage = (event) => { self.postMessage({ type: "probe-ok", value: event.data }); };'],
            { type: "text/javascript" }
          );
          const probeWorker = new Worker(URL.createObjectURL(probeBlob));
          const probeTimer = setTimeout(() => {
            console.error("[compute] inline probe worker timed out");
            probeWorker.terminate();
          }, 1000);
          probeWorker.addEventListener("message", () => {
            clearTimeout(probeTimer);
            console.info("[compute] inline probe worker responded");
            probeWorker.terminate();
          });
          probeWorker.addEventListener("error", () => {
            clearTimeout(probeTimer);
            console.error("[compute] inline probe worker failed");
            probeWorker.terminate();
          });
          probeWorker.postMessage("ping");
        }

        setIsLoading(true);
        await initializeWorkers();
        if (isCancelled) return;
        setError(null);
        setIsReady(true);
      } catch (err: unknown) {
        if (isCancelled) return;
        const normalizedError = toError(err);
        setError(normalizedError);
        setIsReady(false);
        console.error("Failed to initialize compute workers:", normalizedError);
      } finally {
        if (!isCancelled) setIsLoading(false);
      }
    };

    init();

    return () => {
      isCancelled = true;
    };
  }, []);

  const suggestAmountToInvest = useCallback(async (assets: AnalyzeRequest) => {
    try {
      const result = await analyzeWithWorker(assets);
      setError(null);
      return result;
    } catch (err: unknown) {
      const normalizedError = toError(err);
      setError(normalizedError);
      console.error(
        "Unexpected exception in portfolio optimizer worker (analyze):",
        normalizedError
      );
      return null;
    }
  }, []);

  const solve = useCallback(
    async (
      budget: number,
      assets: SolveAssets,
      pfolioCcy: string,
      fees: TransactionFeesInput | null | undefined,
      isBuyOnly: boolean,
      useAllBudget: boolean
    ) => {
      try {
        const result = await solveWithWorker(
          budget,
          assets,
          pfolioCcy,
          fees,
          isBuyOnly,
          useAllBudget
        );
        setError(null);
        return result;
      } catch (err: unknown) {
        const normalizedError = toError(err);
        setError(normalizedError);
        console.error(
          "Unexpected exception in portfolio optimizer worker (solve):",
          normalizedError
        );
        return null;
      }
    },
    []
  );

  const status = useMemo(
    () => ({
      isLoading,
      isReady,
      error,
    }),
    [error, isLoading, isReady]
  );

  const worker = useMemo(
    () => ({
      suggestAmountToInvest,
      solve,
    }),
    [suggestAmountToInvest, solve]
  );

  return [status, worker];
};

export const __runAnalyzeForTests = analyzeWithWorker;
export const __runSolveForTests = solveWithWorker;

export const __resetComputeWorkersForTests = async (): Promise<void> => {
  await destroyWorkerState(portfolioOptimizerState);
};
