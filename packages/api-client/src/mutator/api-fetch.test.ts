import { afterEach, describe, expect, it, vi } from "vitest";

import {
  ApiClientError,
  apiFetch,
  configureApiClientAuth,
  configureApiClientBaseUrl,
  resetApiClientConfiguration,
} from "./api-fetch";

describe("apiFetch", () => {
  afterEach(() => {
    resetApiClientConfiguration();
    vi.restoreAllMocks();
  });

  it("adds the configured base URL and bearer token", async () => {
    configureApiClientBaseUrl("/api/");
    configureApiClientAuth({ getAccessToken: () => "access-token" });
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ ok: true }), { status: 200 }),
    );

    await apiFetch("/assets/fiat", { method: "GET" });

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/assets/fiat",
      expect.objectContaining({ method: "GET" }),
    );
    const firstRequest = fetchMock.mock.calls[0]?.[1];
    expect(new Headers(firstRequest?.headers).get("Authorization")).toBe(
      "Bearer access-token",
    );
  });

  it("refreshes a token once after a 401", async () => {
    const refreshAccessToken = vi.fn().mockResolvedValue("refreshed-token");
    configureApiClientAuth({
      getAccessToken: () => "expired-token",
      refreshAccessToken,
    });
    const fetchMock = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ type: "unauthorized" }), {
          status: 401,
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ value: 42 }), { status: 200 }),
      );

    const result = await apiFetch<{ data: { value: number } }>("/resource", {
      method: "GET",
    });

    expect(result).toMatchObject({ data: { value: 42 }, status: 200 });
    expect(refreshAccessToken).toHaveBeenCalledOnce();
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/resource",
      expect.objectContaining({ method: "GET" }),
    );
    const retriedRequest = fetchMock.mock.calls[1]?.[1];
    expect(new Headers(retriedRequest?.headers).get("Authorization")).toBe(
      "Bearer refreshed-token",
    );
  });

  it("throws a normalized error for non-success responses", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ message: "bad request" }), {
        status: 400,
      }),
    );

    const error = await apiFetch("/resource", { method: "GET" }).catch(
      (value) => value,
    );

    expect(error).toBeInstanceOf(ApiClientError);
    expect(error).toMatchObject({
      status: 400,
      payload: { message: "bad request" },
    });
  });

  it("passes TanStack Query cancellation through to fetch", async () => {
    const controller = new AbortController();
    const abortError = new Error("aborted");
    abortError.name = "AbortError";
    const fetchMock = vi
      .spyOn(globalThis, "fetch")
      .mockImplementation((_input, init) => {
        expect(init?.signal).toBe(controller.signal);
        return Promise.reject(abortError);
      });

    const request = apiFetch("/resource", {
      method: "GET",
      signal: controller.signal,
    });
    controller.abort();

    await expect(request).rejects.toBe(abortError);
    expect(fetchMock).toHaveBeenCalledOnce();
  });
});
