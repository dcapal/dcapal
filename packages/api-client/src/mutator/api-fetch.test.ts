import { http, HttpResponse, delay } from "msw";
import { setupServer } from "msw/node";
import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from "vitest";

import {
  ApiClientError,
  apiFetch,
  configureApiClientAuth,
  configureApiClientBaseUrl,
  resetApiClientConfiguration,
} from "./api-fetch";

const API_ROOT = "http://api.test";
const server = setupServer();

beforeAll(() => server.listen({ onUnhandledRequest: "error" }));
afterEach(() => {
  server.resetHandlers();
  resetApiClientConfiguration();
  vi.restoreAllMocks();
});
afterAll(() => server.close());

describe("apiFetch", () => {
  it("resolves the configured base URL and adds a bearer token", async () => {
    let receivedRequest: Request | undefined;
    server.use(
      http.get(`${API_ROOT}/api/assets/fiat`, ({ request }) => {
        receivedRequest = request;
        return HttpResponse.json({ ok: true });
      })
    );
    configureApiClientBaseUrl(`${API_ROOT}/api/`);
    configureApiClientAuth({ getAccessToken: () => "access-token" });

    await expect(apiFetch("/assets/fiat", { method: "GET" })).resolves.toMatchObject({
      data: { ok: true },
      status: 200,
    });

    expect(receivedRequest).toBeDefined();
    expect(receivedRequest?.headers.get("authorization")).toBe(
      "Bearer access-token"
    );
  });

  it("preserves absolute URLs when a base URL is configured", async () => {
    server.use(
      http.get(`${API_ROOT}/absolute`, () => HttpResponse.json({ ok: true }))
    );
    configureApiClientBaseUrl(`${API_ROOT}/api`);

    await expect(
      apiFetch(`${API_ROOT}/absolute`, { method: "GET" })
    ).resolves.toMatchObject({ data: { ok: true }, status: 200 });
  });

  it("preserves explicit authorization and supports requests without a token", async () => {
    const authorizationHeaders: Array<string | null> = [];
    server.use(
      http.get(`${API_ROOT}/explicit`, ({ request }) => {
        authorizationHeaders.push(request.headers.get("authorization"));
        return HttpResponse.json({ ok: true });
      }),
      http.get(`${API_ROOT}/anonymous`, ({ request }) => {
        authorizationHeaders.push(request.headers.get("authorization"));
        return HttpResponse.json({ ok: true });
      })
    );
    configureApiClientBaseUrl(API_ROOT);
    configureApiClientAuth({ getAccessToken: () => "ignored-token" });

    await apiFetch("/explicit", {
      method: "GET",
      headers: { Authorization: "Custom credentials" },
    });
    configureApiClientAuth({});
    await apiFetch("/anonymous", { method: "GET" });

    expect(authorizationHeaders).toStrictEqual(["Custom credentials", null]);
  });

  it.each([
    ["json", HttpResponse.json({ value: 42 }), { value: 42 }, 200],
    ["text", new HttpResponse("plain text", { status: 200 }), "plain text", 200],
    ["empty", new HttpResponse(null, { status: 200 }), {}, 200],
    ["204", new HttpResponse(null, { status: 204 }), {}, 204],
    ["205", new HttpResponse(null, { status: 205 }), {}, 205],
  ])("parses %s responses", async (_name, response, expected, status) => {
    server.use(http.get(`${API_ROOT}/response`, () => response));
    configureApiClientBaseUrl(API_ROOT);

    await expect(apiFetch("/response", { method: "GET" })).resolves.toMatchObject({
      data: expected,
      status,
    });
  });

  it("normalizes a non-success response, including an empty 304 body", async () => {
    server.use(
      http.get(`${API_ROOT}/bad`, () =>
        HttpResponse.json({ message: "bad request" }, { status: 400 })
      ),
      http.get(`${API_ROOT}/not-modified`, () =>
        new HttpResponse(null, { status: 304 })
      )
    );
    configureApiClientBaseUrl(API_ROOT);

    await expect(apiFetch("/bad", { method: "GET" })).rejects.toMatchObject({
      name: "ApiClientError",
      status: 400,
      payload: { message: "bad request" },
    });
    await expect(
      apiFetch("/not-modified", { method: "GET" })
    ).rejects.toMatchObject({
      status: 304,
      payload: {},
    });
  });

  it("refreshes once after a 401 and retries with the refreshed token", async () => {
    const requestHeaders: Array<string | null> = [];
    let requestCount = 0;
    server.use(
      http.get(`${API_ROOT}/resource`, ({ request }) => {
        requestHeaders.push(request.headers.get("authorization"));
        requestCount += 1;
        return requestCount === 1
          ? HttpResponse.json({ type: "unauthorized" }, { status: 401 })
          : HttpResponse.json({ value: 42 });
      })
    );
    const refreshAccessToken = vi.fn().mockResolvedValue("refreshed-token");
    configureApiClientBaseUrl(API_ROOT);
    configureApiClientAuth({
      getAccessToken: () => "expired-token",
      refreshAccessToken,
    });

    await expect(apiFetch("/resource", { method: "GET" })).resolves.toMatchObject({
      data: { value: 42 },
      status: 200,
    });

    expect(refreshAccessToken).toHaveBeenCalledOnce();
    expect(requestHeaders).toStrictEqual([
      "Bearer expired-token",
      "Bearer refreshed-token",
    ]);
  });

  it("calls the auth failure hook when token refresh throws", async () => {
    server.use(
      http.get(`${API_ROOT}/resource`, () =>
        HttpResponse.json({ type: "unauthorized" }, { status: 401 })
      )
    );
    const refreshError = new Error("refresh failed");
    const onAuthFailure = vi.fn();
    configureApiClientBaseUrl(API_ROOT);
    configureApiClientAuth({
      getAccessToken: () => "expired-token",
      refreshAccessToken: vi.fn().mockRejectedValue(refreshError),
      onAuthFailure,
    });

    await expect(apiFetch("/resource", { method: "GET" })).rejects.toBe(
      refreshError
    );
    expect(onAuthFailure).toHaveBeenCalledWith(refreshError);
  });

  it("fails safely when refresh returns no token", async () => {
    server.use(
      http.get(`${API_ROOT}/resource`, () =>
        HttpResponse.json({ type: "unauthorized" }, { status: 401 })
      )
    );
    const onAuthFailure = vi.fn();
    configureApiClientBaseUrl(API_ROOT);
    configureApiClientAuth({
      getAccessToken: () => "expired-token",
      refreshAccessToken: vi.fn().mockResolvedValue(null),
      onAuthFailure,
    });

    await expect(apiFetch("/resource", { method: "GET" })).rejects.toMatchObject({
      status: 401,
      payload: { type: "unauthorized" },
    });
    expect(onAuthFailure).toHaveBeenCalledOnce();
  });

  it("calls the auth failure hook after a retried 401", async () => {
    server.use(
      http.get(`${API_ROOT}/resource`, () =>
        HttpResponse.json({ type: "unauthorized" }, { status: 401 })
      )
    );
    const onAuthFailure = vi.fn();
    configureApiClientBaseUrl(API_ROOT);
    configureApiClientAuth({
      getAccessToken: () => "expired-token",
      refreshAccessToken: vi.fn().mockResolvedValue("refreshed-token"),
      onAuthFailure,
    });

    await expect(apiFetch("/resource", { method: "GET" })).rejects.toMatchObject({
      status: 401,
    });
    expect(onAuthFailure).toHaveBeenCalledOnce();
  });

  it("propagates abort signals to the real fetch call", async () => {
    const controller = new AbortController();
    server.use(
      http.get(`${API_ROOT}/slow`, async () => {
        await delay(100);
        return HttpResponse.json({ ok: true });
      })
    );
    configureApiClientBaseUrl(API_ROOT);

    const request = apiFetch("/slow", {
      method: "GET",
      signal: controller.signal,
    });
    controller.abort();

    await expect(request).rejects.toMatchObject({ name: "AbortError" });
  });
});
