import type { Page } from "@playwright/test";

type FixtureSession = {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
  expires_at: number;
  user: {
    id: string;
    aud: string;
    role: string;
    email: string;
    user_metadata: { name: string };
  };
  [key: string]: unknown;
};

type SessionOverrides = Partial<FixtureSession>;

/** Creates a deterministic Supabase session for browser journeys. */
export const fixtureSession = (
  overrides: SessionOverrides = {}
): FixtureSession => ({
  access_token: "fixture-token.fixture-session.signature",
  refresh_token: "fixture-refresh-token",
  token_type: "bearer",
  expires_in: 3600,
  expires_at: 4102444800,
  user: {
    id: "fixture-user",
    aud: "authenticated",
    role: "authenticated",
    email: "fixture@example.com",
    user_metadata: { name: "Fixture User" },
  },
  ...overrides,
});

/** Seeds the browser storage entries read by the Supabase client on startup. */
export const seedAuthenticatedSession = async (
  page: Page,
  overrides: SessionOverrides = {}
): Promise<void> => {
  const session = fixtureSession(overrides);
  await page.addInitScript((value) => {
    const serialized = JSON.stringify(value);
    window.localStorage.setItem("supabase.auth.token", serialized);
    window.localStorage.setItem("sb-127-auth-token", serialized);
    window.localStorage.setItem("sb-127-0-0-1-auth-token", serialized);
  }, session);
};
