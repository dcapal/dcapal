import { beforeEach, describe, expect, it, vi } from "vitest";

const { spawnMock, workerCtorMock, terminateMock } = vi.hoisted(() => ({
  spawnMock: vi.fn(),
  workerCtorMock: vi.fn(),
  terminateMock: vi.fn(),
}));

vi.mock("threads", () => ({
  spawn: spawnMock,
  Thread: {
    terminate: terminateMock,
  },
  Worker: function Worker(url: URL, options: { name: string }) {
    workerCtorMock(url, options);
    return { url, options };
  },
}));

import { __resetComputeWorkersForTests, analyze, solve } from "./index";

describe("compute boundary", () => {
  beforeEach(async () => {
    await __resetComputeWorkersForTests();
    vi.clearAllMocks();
    terminateMock.mockResolvedValue(undefined);
  });

  it("reuses analyzer worker across calls", async () => {
    const analyzerRpc = {
      analyzeAndSolve: vi
        .fn()
        .mockResolvedValueOnce(123.45)
        .mockResolvedValueOnce(678.9),
    };
    spawnMock.mockResolvedValue(analyzerRpc);

    const assets = {
      SPY: {
        symbol: "SPY",
        target_weight: 1,
        shares: 10,
        price: 500,
      },
    };

    const result1 = await analyze(assets);
    const result2 = await analyze(assets);

    expect(result1).toBe(123.45);
    expect(result2).toBe(678.9);
    expect(spawnMock).toHaveBeenCalledTimes(1);
    expect(workerCtorMock).toHaveBeenCalledTimes(1);
    expect(workerCtorMock.mock.calls[0][1]).toStrictEqual({
      name: "wasm-analyzer-worker",
    });
    expect(analyzerRpc.analyzeAndSolve).toHaveBeenCalledTimes(2);
    expect(analyzerRpc.analyzeAndSolve).toHaveBeenNthCalledWith(1, assets);
    expect(analyzerRpc.analyzeAndSolve).toHaveBeenNthCalledWith(2, assets);
    expect(terminateMock).not.toHaveBeenCalled();
  });

  it("serializes solve calls on the same worker", async () => {
    let releaseFirstCall: () => void = () => {};
    const firstCall = new Promise<void>((resolve) => {
      releaseFirstCall = resolve;
    });

    const solverRpc = {
      makeAndSolve: vi
        .fn()
        .mockImplementationOnce(async () => {
          await firstCall;
          return { budget_left: 10 };
        })
        .mockResolvedValueOnce({ budget_left: 5 }),
    };
    spawnMock.mockResolvedValue(solverRpc);

    const budget = 1000;
    const assets = {
      VWCE: {
        symbol: "VWCE",
        shares: 0,
        price: 123,
        target_weight: 1,
      },
    };

    const call1 = solve(budget, assets, "EUR", null, true, false);
    const call2 = solve(budget, assets, "EUR", null, true, false);

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(spawnMock).toHaveBeenCalledTimes(1);
    expect(solverRpc.makeAndSolve).toHaveBeenCalledTimes(1);

    releaseFirstCall();

    await expect(call1).resolves.toStrictEqual({ budget_left: 10 });
    await expect(call2).resolves.toStrictEqual({ budget_left: 5 });

    expect(solverRpc.makeAndSolve).toHaveBeenCalledTimes(2);
    expect(terminateMock).not.toHaveBeenCalled();
  });

  it("terminates failed analyzer worker and recreates it on next call", async () => {
    const failingRpc = {
      analyzeAndSolve: vi.fn().mockRejectedValue(new Error("boom")),
    };
    const healthyRpc = {
      analyzeAndSolve: vi.fn().mockResolvedValue(42),
    };
    spawnMock
      .mockResolvedValueOnce(failingRpc)
      .mockResolvedValueOnce(healthyRpc);
    vi.spyOn(console, "error").mockImplementation(() => {});

    const assets = { SPY: { symbol: "SPY" } };

    const firstResult = await analyze(assets);
    const secondResult = await analyze(assets);

    expect(firstResult).toBeNull();
    expect(secondResult).toBe(42);
    expect(spawnMock).toHaveBeenCalledTimes(2);
    expect(terminateMock).toHaveBeenCalledTimes(1);
    expect(terminateMock).toHaveBeenCalledWith(failingRpc);
  });

  it("returns null and does not spawn workers for invalid payloads", async () => {
    const analyzeResult = await analyze(
      null as unknown as Record<string, never>
    );
    const solveResult = await solve(
      "100" as unknown as number,
      { SPY: {} },
      "USD",
      null,
      true,
      false
    );

    expect(analyzeResult).toBeNull();
    expect(solveResult).toBeNull();
    expect(spawnMock).not.toHaveBeenCalled();
  });
});
