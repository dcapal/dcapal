export const fixtureSession = (overrides = {}) => ({
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

export const seedAuthenticatedSession = async (page, overrides = {}) => {
  const session = fixtureSession(overrides);
  await page.addInitScript((value) => {
    const serialized = JSON.stringify(value);
    window.localStorage.setItem("supabase.auth.token", serialized);
    window.localStorage.setItem("sb-127-auth-token", serialized);
    window.localStorage.setItem("sb-127-0-0-1-auth-token", serialized);
  }, session);
};
