import { beforeEach, describe, expect, it, vi } from "vitest";

const {
  wrapMock,
  workerCtorMock,
  workerTerminateMock,
  portfolioOptimizerRpc,
} = vi.hoisted(() => ({
  wrapMock: vi.fn(),
  workerCtorMock: vi.fn(),
  workerTerminateMock: vi.fn(),
  portfolioOptimizerRpc: {
    ping: vi.fn(),
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

  constructor(url: URL, options?: WorkerOptions) {
    this.url = url;
    this.options = options;
    workerCtorMock(url, options);
  }

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
    vi.clearAllMocks();

    portfolioOptimizerRpc.init.mockResolvedValue(undefined);
    portfolioOptimizerRpc.ping.mockResolvedValue("pong");
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

  it("reuses the same worker across analyze and solve operations", async () => {
    portfolioOptimizerRpc.analyzeAndSolve.mockResolvedValueOnce(1);
    portfolioOptimizerRpc.makeAndSolve.mockResolvedValueOnce({ budget_left: 5 });

    const assets = { SPY: { symbol: "SPY", target_weight: 1, shares: 10, price: 500 } };
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
