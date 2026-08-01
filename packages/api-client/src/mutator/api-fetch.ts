type MaybePromise<T> = T | Promise<T>;

type ApiFetchRequestInit = RequestInit & {
  __apiFetchRetryAttempted?: boolean;
  __apiFetchSkipAuthHandling?: boolean;
};

export interface ApiClientAuthHooks {
  getAccessToken?: () => MaybePromise<string | null | undefined>;
  refreshAccessToken?: () => MaybePromise<string | null | undefined>;
  onAuthFailure?: (error: unknown) => MaybePromise<void>;
}

export class ApiClientError extends Error {
  readonly status: number;
  readonly payload: unknown;

  constructor(status: number, payload: unknown) {
    super(`Request failed with status ${status}`);
    this.name = "ApiClientError";
    this.status = status;
    this.payload = payload;
  }
}

let apiBaseUrl = "";
let authHooks: ApiClientAuthHooks | null = null;

function normalizeBaseUrl(baseUrl: string | null | undefined): string {
  const trimmed = baseUrl?.trim();
  return trimmed ? trimmed.replace(/\/+$/, "") : "";
}

function resolveUrl(url: string): string {
  if (!apiBaseUrl || /^https?:\/\//i.test(url) || url.startsWith("//")) {
    return url;
  }

  return `${apiBaseUrl}${url.startsWith("/") ? url : `/${url}`}`;
}

export function configureApiClientBaseUrl(
  baseUrl: string | null | undefined,
): void {
  apiBaseUrl = normalizeBaseUrl(baseUrl);
}

export function configureApiClientAuth(
  hooks: ApiClientAuthHooks | null,
): void {
  authHooks = hooks;
}

export function resetApiClientConfiguration(): void {
  apiBaseUrl = "";
  authHooks = null;
}

function hasAuthorizationHeader(headers?: HeadersInit): boolean {
  return headers ? new Headers(headers).has("Authorization") : false;
}

function withAuthorizationHeader(
  headers: HeadersInit | undefined,
  accessToken: string,
): Headers {
  const resolvedHeaders = new Headers(headers);
  resolvedHeaders.set("Authorization", `Bearer ${accessToken}`);
  return resolvedHeaders;
}

function toNativeRequestInit(options: ApiFetchRequestInit): RequestInit {
  const {
    __apiFetchRetryAttempted: _retryAttempted,
    __apiFetchSkipAuthHandling: _skipAuthHandling,
    ...nativeOptions
  } = options;

  return nativeOptions;
}

async function resolveRequestInit(
  options: ApiFetchRequestInit,
): Promise<ApiFetchRequestInit> {
  const getAccessToken = authHooks?.getAccessToken;
  if (
    !getAccessToken ||
    options.__apiFetchSkipAuthHandling ||
    hasAuthorizationHeader(options.headers)
  ) {
    return options;
  }

  const accessToken = await getAccessToken();
  if (!accessToken) return options;

  return {
    ...options,
    headers: withAuthorizationHeader(options.headers, accessToken),
  };
}

function parseResponseBody(body: string | null): unknown {
  if (!body) return {};

  try {
    return JSON.parse(body);
  } catch {
    return body;
  }
}

async function readResponseBody(response: Response): Promise<unknown> {
  if ([204, 205, 304].includes(response.status)) return {};
  return parseResponseBody(await response.text());
}

function toUnauthorizedError(): ApiClientError {
  return new ApiClientError(401, { type: "unauthorized" });
}

export async function apiFetch<T>(
  url: string,
  options: RequestInit,
): Promise<T> {
  const requestOptions = options as ApiFetchRequestInit;
  const resolvedOptions = await resolveRequestInit(requestOptions);
  const response = await fetch(
    resolveUrl(url),
    toNativeRequestInit(resolvedOptions),
  );
  const payload = await readResponseBody(response);

  if (!response.ok) {
    const shouldRefresh =
      response.status === 401 &&
      !resolvedOptions.__apiFetchRetryAttempted &&
      !resolvedOptions.__apiFetchSkipAuthHandling &&
      !!authHooks?.refreshAccessToken;

    if (shouldRefresh) {
      let refreshedToken: string | null | undefined;

      try {
        refreshedToken = await authHooks?.refreshAccessToken?.();
      } catch (error) {
        await authHooks?.onAuthFailure?.(error);
        throw error;
      }

      if (!refreshedToken) {
        const error = toUnauthorizedError();
        await authHooks?.onAuthFailure?.(error);
        throw error;
      }

      try {
        return await apiFetch<T>(url, {
          ...requestOptions,
          __apiFetchRetryAttempted: true,
          headers: withAuthorizationHeader(
            requestOptions.headers,
            refreshedToken,
          ),
        } as RequestInit);
      } catch (error) {
        if (error instanceof ApiClientError && error.status === 401) {
          await authHooks?.onAuthFailure?.(error);
        }
        throw error;
      }
    }

    throw new ApiClientError(response.status, payload);
  }

  return {
    data: payload,
    status: response.status,
    headers: response.headers,
  } as T;
}
