const fs = require("node:fs");
const path = require("node:path");

const FRONTEND_ORIGIN = "http://127.0.0.1:3000";
const DEFAULT_SUPABASE_URL = "http://127.0.0.1:54321";
const DEFAULT_USER_EMAIL = "smoke@example.com";
const DEFAULT_USER_PASSWORD = "smoke-password-123456";
const DEFAULT_USER_NAME = "Smoke User";

const requiredEnvironment = (name, fallback) => {
  const value = process.env[name] || fallback;
  if (!value) throw new Error(`The full-stack smoke setup requires ${name}.`);
  return value;
};

const userMetadata = (email) => ({
  email,
  full_name: DEFAULT_USER_NAME,
  name: DEFAULT_USER_NAME,
});

const requestJson = async (url, options) => {
  const response = await fetch(url, options);
  const responseText = await response.text();
  let responseBody;

  try {
    responseBody = responseText ? JSON.parse(responseText) : undefined;
  } catch {
    responseBody = undefined;
  }

  if (!response.ok) {
    const message =
      responseBody?.msg ||
      responseBody?.message ||
      responseBody?.error_description ||
      responseText ||
      response.statusText;
    throw new Error(`${response.status} ${message}`);
  }

  return responseBody;
};

/** Creates a repeatable local Supabase user and browser session for full-stack smoke. */
module.exports = async (config) => {
  const supabaseUrl = requiredEnvironment(
    "SUPABASE_URL",
    process.env.VITE_SUPABASE_URL || DEFAULT_SUPABASE_URL
  );
  const anonKey = requiredEnvironment(
    "SUPABASE_ANON_KEY",
    process.env.VITE_SUPABASE_ANON_KEY
  );
  const serviceRoleKey = requiredEnvironment(
    "SUPABASE_SERVICE_ROLE_KEY",
    undefined
  );
  const email = process.env.SMOKE_USER_EMAIL || DEFAULT_USER_EMAIL;
  const password = process.env.SMOKE_USER_PASSWORD || DEFAULT_USER_PASSWORD;

  const adminHeaders = {
    apikey: serviceRoleKey,
    Authorization: `Bearer ${serviceRoleKey}`,
    "Content-Type": "application/json",
  };
  const authApiUrl = `${supabaseUrl}/auth/v1`;
  const userData = await requestJson(
    `${authApiUrl}/admin/users?page=1&per_page=1000`,
    { headers: adminHeaders }
  );

  const users = userData?.users || [];
  const existingUser = users.find((user) => user.email === email);
  const metadata = userMetadata(email);

  if (existingUser) {
    await requestJson(`${authApiUrl}/admin/users/${existingUser.id}`, {
      method: "PUT",
      headers: adminHeaders,
      body: JSON.stringify({
        password,
        email_confirm: true,
        user_metadata: metadata,
      }),
    });
  } else {
    await requestJson(`${authApiUrl}/admin/users`, {
      method: "POST",
      headers: adminHeaders,
      body: JSON.stringify({
        email,
        password,
        email_confirm: true,
        user_metadata: metadata,
      }),
    });
  }

  const session = await requestJson(
    `${authApiUrl}/token?grant_type=password`,
    {
      method: "POST",
      headers: {
        apikey: anonKey,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ email, password }),
    }
  );
  if (!session?.access_token) throw new Error("Supabase returned no access token.");

  const supabaseHost = new URL(supabaseUrl).hostname.split(".")[0];
  const storageKey = `sb-${supabaseHost}-auth-token`;
  const storageStatePath = path.resolve(
    config.configDir || process.cwd(),
    "test-results/smoke/storage-state.json"
  );
  fs.mkdirSync(path.dirname(storageStatePath), { recursive: true });
  fs.writeFileSync(
    storageStatePath,
    JSON.stringify(
      {
        cookies: [],
        origins: [
          {
            origin: FRONTEND_ORIGIN,
            localStorage: [
              { name: storageKey, value: JSON.stringify(session) },
            ],
          },
        ],
      },
      null,
      2
    )
  );
};
