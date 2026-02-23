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

const createWorkerState = <
  TWorker extends object,
>(): ComputeWorkerState<TWorker> => ({
  worker: null,
  proxy: null,
  initPromise: null,
  createPromise: null,
  onMessage: null,
  onError: null,
  onMessageError: null,
});

const portfolioOptimizerState =
  createWorkerState<PortfolioOptimizerWorkerRpc>();
const isStaticWorkerMode = import.meta.env.VITE_E2E_STATIC_WORKER === "1";
let pingRequestCounter = 0;

const createPortfolioOptimizerWorkerInstance = (): Worker => {
  return import.meta.env.DEV
    ? new Worker(
        new URL("./workers/portfolioOptimizer.worker.js", import.meta.url),
        {
          type: "module",
          name: "portfolio-optimizer",
        }
      )
    : new Worker(
        new URL("./workers/portfolioOptimizer.worker.js", import.meta.url),
        {
          type: "classic",
          name: "portfolio-optimizer",
        }
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

const PING_TIMEOUT_MS = isStaticWorkerMode ? 2_000 : 20_000;
const INIT_TIMEOUT_MS = isStaticWorkerMode ? 3_000 : 30_000;

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

type PingRequestMessage = {
  type: "ping";
  requestId: number;
};

type PingResponseMessage = {
  type: "pong";
  requestId: number;
};

const isPingResponseMessage = (data: unknown): data is PingResponseMessage => {
  if (typeof data !== "object" || data === null) return false;
  const candidate = data as Partial<PingResponseMessage>;
  return candidate.type === "pong" && typeof candidate.requestId === "number";
};

const pingWorkerDirect = async (
  worker: Worker,
  workerName: string
): Promise<string> => {
  const requestId = pingRequestCounter++;
  const request: PingRequestMessage = { type: "ping", requestId };

  const pingPromise = new Promise<string>((resolve, reject) => {
    let isSettled = false;
    const cleanUp = () => {
      worker.removeEventListener("message", onMessage);
      worker.removeEventListener("error", onError);
      worker.removeEventListener("messageerror", onMessageError);
    };

    const finish = (next: () => void) => {
      if (isSettled) return;
      isSettled = true;
      cleanUp();
      next();
    };

    const onMessage = (event: MessageEvent) => {
      if (!isPingResponseMessage(event.data)) return;
      if (event.data.requestId !== requestId) return;
      finish(() => resolve("pong"));
    };

    const onError = (event: ErrorEvent) => {
      const message = event.message || "worker emitted error event during ping";
      finish(() => reject(new Error(`${workerName} ping failed: ${message}`)));
    };

    const onMessageError = () => {
      finish(() =>
        reject(new Error(`${workerName} ping failed: messageerror event`))
      );
    };

    worker.addEventListener("message", onMessage);
    worker.addEventListener("error", onError);
    worker.addEventListener("messageerror", onMessageError);

    try {
      worker.postMessage(request);
    } catch (error: unknown) {
      finish(() => reject(toError(error)));
    }
  });

  return withTimeout(pingPromise, PING_TIMEOUT_MS, `${workerName} ping`);
};

const getOrCreateWorker = async <Rpc extends { init(): Promise<void> }>(
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
      try {
        if (!state.worker || !state.proxy) {
          console.info(`[compute] creating ${workerName} worker instance`);
          state.worker = createWorker();
          console.info(`[compute] ${workerName} worker instance created`);
          // state.onMessage = (event: MessageEvent) => {
          //   console.info(`[${workerName}] worker message event`, event.data);
          // };
          // state.onError = (event: ErrorEvent) => {
          //   const details = [
          //     `message=${event.message || "<empty>"}`,
          //     `filename=${event.filename || "<empty>"}`,
          //     `lineno=${String(event.lineno ?? "<none>")}`,
          //     `colno=${String(event.colno ?? "<none>")}`,
          //   ].join(" ");
          //   console.error(`[${workerName}] worker error event ${details}`);
          // };
          // state.onMessageError = (event: MessageEvent) => {
          //   const details = [
          //     `origin=${event.origin || "<empty>"}`,
          //     `lastEventId=${event.lastEventId || "<empty>"}`,
          //     `dataType=${typeof event.data}`,
          //   ].join(" ");
          //   console.error(`[${workerName}] worker messageerror event ${details}`);
          // };
          // if (typeof state.worker.addEventListener === "function") {
          //   state.worker.addEventListener("message", state.onMessage);
          //   state.worker.addEventListener("error", state.onError);
          //   state.worker.addEventListener("messageerror", state.onMessageError);
          // }
          const pingStart = Date.now();
          console.info(`[compute] ${workerName} ping start`);
          await pingWorkerDirect(state.worker, workerName);
          console.info(
            `[compute] ${workerName} ping resolved in ${Date.now() - pingStart}ms`
          );
          debugger;
          console.info(`[compute] wrapping ${workerName} worker with Comlink`);
          state.proxy = wrap<Rpc>(state.worker);
          console.info(`[compute] ${workerName} worker wrapped`);
        }

        const proxy = state.proxy;
        if (!proxy) {
          throw new Error(`${workerName} worker proxy is not available`);
        }

        if (!state.initPromise) {
          state.initPromise = (async () => {
            const initStart = Date.now();
            console.info(`[compute] ${workerName} init start`);
            await withTimeout(
              proxy.init(),
              INIT_TIMEOUT_MS,
              `${workerName} init`
            );
            console.info(
              `[compute] ${workerName} init resolved in ${Date.now() - initStart}ms`
            );
          })();
        }

        await state.initPromise;
        return state.proxy;
      } catch (error: unknown) {
        console.error(
          `[compute] ${workerName} init pipeline failed`,
          describeUnknownError(error)
        );
        await destroyWorkerState(state);
        throw error;
      }
    })().finally(() => {
      state.createPromise = null;
    });
  }

  return state.createPromise;
};

const runWithRetry = async <Rpc extends { init(): Promise<void> }, Result>(
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
