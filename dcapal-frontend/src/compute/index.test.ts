import { beforeEach, describe, expect, it, vi } from "vitest";

type PingBehavior =
  | "success"
  | "timeout"
  | "wrong-response"
  | "error"
  | "messageerror";

const {
  wrapMock,
  workerCtorMock,
  workerTerminateMock,
  workerAddEventListenerMock,
  workerRemoveEventListenerMock,
  workerPostMessageMock,
  pingBehavior,
  portfolioOptimizerRpc,
} = vi.hoisted(() => ({
  wrapMock: vi.fn(),
  workerCtorMock: vi.fn(),
  workerTerminateMock: vi.fn(),
  workerAddEventListenerMock: vi.fn(),
  workerRemoveEventListenerMock: vi.fn(),
  workerPostMessageMock: vi.fn(),
  pingBehavior: { mode: "success" as PingBehavior },
  portfolioOptimizerRpc: {
    init: vi.fn(),
    analyzeAndSolve: vi.fn(),
    makeAndSolve: vi.fn(),
  },
}));

vi.mock("comlink", () => ({
  wrap: wrapMock,
}));

class MockWorker {
  url: URL;
  options: WorkerOptions | undefined;
  listeners: Map<string, Set<(event: unknown) => void>>;

  constructor(url: URL, options?: WorkerOptions) {
    this.url = url;
    this.options = options;
    this.listeners = new Map();
    workerCtorMock(url, options);
  }

  private emit = (type: string, event: unknown) => {
    const listeners = this.listeners.get(type);
    if (!listeners) return;

    for (const listener of listeners) {
      listener(event);
    }
  };

  addEventListener = (type: string, listener: (event: unknown) => void) => {
    workerAddEventListenerMock(type, listener);
    if (!this.listeners.has(type)) {
      this.listeners.set(type, new Set());
    }
    this.listeners.get(type)?.add(listener);
  };

  removeEventListener = (type: string, listener: (event: unknown) => void) => {
    workerRemoveEventListenerMock(type, listener);
    this.listeners.get(type)?.delete(listener);
  };

  postMessage = (payload: unknown) => {
    workerPostMessageMock(payload);

    const message = payload as { type?: string; requestId?: number };
    if (message.type !== "ping") return;

    switch (pingBehavior.mode) {
      case "success":
        queueMicrotask(() => {
          this.emit("message", {
            data: { type: "pong", requestId: message.requestId },
          });
        });
        return;
      case "wrong-response":
        queueMicrotask(() => {
          this.emit("message", { data: { type: "pong", requestId: -1 } });
        });
        return;
      case "error":
        queueMicrotask(() => {
          this.emit("error", { message: "boom" });
        });
        return;
      case "messageerror":
        queueMicrotask(() => {
          this.emit("messageerror", { data: "bad" });
        });
        return;
      case "timeout":
      default:
        return;
    }
  };

  terminate = workerTerminateMock;
}

vi.stubGlobal("Worker", MockWorker as unknown as typeof Worker);

import {
  __resetComputeWorkersForTests,
  __runAnalyzeForTests,
  __runSolveForTests,
} from "./index";

describe("compute boundary", () => {
  beforeEach(async () => {
    await __resetComputeWorkersForTests();
    vi.useRealTimers();
    vi.clearAllMocks();

    pingBehavior.mode = "success";
    portfolioOptimizerRpc.init.mockResolvedValue(undefined);
    portfolioOptimizerRpc.analyzeAndSolve.mockReset();
    portfolioOptimizerRpc.makeAndSolve.mockReset();

    wrapMock.mockReturnValue(portfolioOptimizerRpc);
  });

  it("reuses portfolio optimizer worker across analyze calls", async () => {
    portfolioOptimizerRpc.analyzeAndSolve
      .mockResolvedValueOnce(123.45)
      .mockResolvedValueOnce(678.9);

    const assets = {
      SPY: {
        symbol: "SPY",
        target_weight: 1,
        shares: 10,
        price: 500,
      },
    };

    const result1 = await __runAnalyzeForTests(assets);
    const result2 = await __runAnalyzeForTests(assets);

    expect(result1).toBe(123.45);
    expect(result2).toBe(678.9);
    expect(workerCtorMock).toHaveBeenCalledTimes(1);
    expect(wrapMock).toHaveBeenCalledTimes(1);
    expect(portfolioOptimizerRpc.init).toHaveBeenCalledTimes(1);
    expect(portfolioOptimizerRpc.analyzeAndSolve).toHaveBeenCalledTimes(2);
    expect(workerTerminateMock).not.toHaveBeenCalled();
    expect(String(workerCtorMock.mock.calls[0]?.[0])).toContain(
      "portfolioOptimizer.worker.js"
    );
  });

  it("cleans up ping listeners after successful ping handshake", async () => {
    portfolioOptimizerRpc.analyzeAndSolve.mockResolvedValueOnce(1);

    const assets = {
      SPY: {
        symbol: "SPY",
        target_weight: 1,
        shares: 10,
        price: 500,
      },
    };

    await __runAnalyzeForTests(assets);

    expect(workerAddEventListenerMock).toHaveBeenCalledTimes(3);
    expect(workerRemoveEventListenerMock).toHaveBeenCalledTimes(3);
    expect(workerPostMessageMock).toHaveBeenCalledTimes(1);
  });

  it("reuses the same worker across analyze and solve operations", async () => {
    portfolioOptimizerRpc.analyzeAndSolve.mockResolvedValueOnce(1);
    portfolioOptimizerRpc.makeAndSolve.mockResolvedValueOnce({
      budget_left: 5,
    });

    const assets = {
      SPY: { symbol: "SPY", target_weight: 1, shares: 10, price: 500 },
    };
    await __runAnalyzeForTests(assets);
    await __runSolveForTests(1000, assets, "USD", null, true, false);

    expect(workerCtorMock).toHaveBeenCalledTimes(1);
    expect(wrapMock).toHaveBeenCalledTimes(1);
    expect(portfolioOptimizerRpc.init).toHaveBeenCalledTimes(1);
    expect(portfolioOptimizerRpc.analyzeAndSolve).toHaveBeenCalledTimes(1);
    expect(portfolioOptimizerRpc.makeAndSolve).toHaveBeenCalledTimes(1);
  });

  it("does not serialize solver calls", async () => {
    let releaseFirstCall: (value: { budget_left: number }) => void = () => {};

    portfolioOptimizerRpc.makeAndSolve
      .mockImplementationOnce(
        () => new Promise((resolve) => (releaseFirstCall = resolve))
      )
      .mockResolvedValueOnce({ budget_left: 5 });

    const budget = 1000;
    const assets = {
      VWCE: {
        symbol: "VWCE",
        shares: 0,
        price: 123,
        target_weight: 1,
      },
    };

    const call1 = __runSolveForTests(budget, assets, "EUR", null, true, false);
    const call2 = __runSolveForTests(budget, assets, "EUR", null, true, false);

    await vi.waitFor(() => {
      expect(workerCtorMock).toHaveBeenCalledTimes(1);
      expect(portfolioOptimizerRpc.makeAndSolve).toHaveBeenCalledTimes(2);
    });

    releaseFirstCall({ budget_left: 10 });

    const results = await Promise.all([call1, call2]);
    expect(results).toContainEqual({ budget_left: 10 });
    expect(results).toContainEqual({ budget_left: 5 });
  });

  it("fails and retries once when ping emits worker error event", async () => {
    pingBehavior.mode = "error";

    const assets = {
      SPY: { symbol: "SPY", target_weight: 1, shares: 10, price: 500 },
    };
    await expect(__runAnalyzeForTests(assets)).rejects.toThrow("ping failed");

    expect(workerCtorMock).toHaveBeenCalledTimes(2);
    expect(wrapMock).not.toHaveBeenCalled();
    expect(portfolioOptimizerRpc.init).not.toHaveBeenCalled();
    expect(workerTerminateMock).toHaveBeenCalledTimes(2);
  });

  it("fails and retries once when ping emits messageerror", async () => {
    pingBehavior.mode = "messageerror";

    const assets = {
      SPY: { symbol: "SPY", target_weight: 1, shares: 10, price: 500 },
    };
    await expect(__runAnalyzeForTests(assets)).rejects.toThrow(
      "messageerror event"
    );

    expect(workerCtorMock).toHaveBeenCalledTimes(2);
    expect(wrapMock).not.toHaveBeenCalled();
    expect(portfolioOptimizerRpc.init).not.toHaveBeenCalled();
    expect(workerTerminateMock).toHaveBeenCalledTimes(2);
  });

  it("times out when ping receives malformed response and retries once", async () => {
    vi.useFakeTimers();
    pingBehavior.mode = "wrong-response";

    const assets = {
      SPY: { symbol: "SPY", target_weight: 1, shares: 10, price: 500 },
    };
    const promise = __runAnalyzeForTests(assets);
    const assertion = expect(promise).rejects.toThrow("timed out");

    await vi.advanceTimersByTimeAsync(50_000);
    await assertion;

    expect(workerCtorMock).toHaveBeenCalledTimes(2);
    expect(wrapMock).not.toHaveBeenCalled();
    expect(portfolioOptimizerRpc.init).not.toHaveBeenCalled();
    expect(workerTerminateMock).toHaveBeenCalledTimes(2);
  });

  it("recreates worker and retries once after analyze failure", async () => {
    portfolioOptimizerRpc.analyzeAndSolve
      .mockRejectedValueOnce(new Error("boom"))
      .mockResolvedValueOnce(42);

    const assets = { SPY: { symbol: "SPY" } };
    const result = await __runAnalyzeForTests(assets);

    expect(result).toBe(42);
    expect(workerCtorMock).toHaveBeenCalledTimes(2);
    expect(wrapMock).toHaveBeenCalledTimes(2);
    expect(workerTerminateMock).toHaveBeenCalledTimes(1);
    expect(portfolioOptimizerRpc.analyzeAndSolve).toHaveBeenCalledTimes(2);
  });

  it("returns null and does not spawn workers for invalid payloads", async () => {
    const analyzeResult = await __runAnalyzeForTests(
      null as unknown as Record<string, never>
    );
    const solveResult = await __runSolveForTests(
      "100" as unknown as number,
      { SPY: {} },
      "USD",
      null,
      true,
      false
    );

    expect(analyzeResult).toBeNull();
    expect(solveResult).toBeNull();
    expect(workerCtorMock).not.toHaveBeenCalled();
    expect(wrapMock).not.toHaveBeenCalled();
  });
});
