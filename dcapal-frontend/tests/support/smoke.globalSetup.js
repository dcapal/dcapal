const fs = require("node:fs");
const path = require("node:path");
const { createClient } = require("@supabase/supabase-js");

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

  const adminClient = createClient(supabaseUrl, serviceRoleKey, {
    auth: {
      autoRefreshToken: false,
      persistSession: false,
      detectSessionInUrl: false,
    },
  });

  const { data: userData, error: listError } =
    await adminClient.auth.admin.listUsers({ page: 1, perPage: 1000 });
  if (listError)
    throw new Error(`Could not list smoke users: ${listError.message}`);

  const users = userData?.users || [];
  const existingUser = users.find((user) => user.email === email);
  const metadata = userMetadata(email);

  if (existingUser) {
    const { error } = await adminClient.auth.admin.updateUserById(
      existingUser.id,
      {
        password,
        email_confirm: true,
        user_metadata: metadata,
      }
    );
    if (error) throw new Error(`Could not reset smoke user: ${error.message}`);
  } else {
    const { error } = await adminClient.auth.admin.createUser({
      email,
      password,
      email_confirm: true,
      user_metadata: metadata,
    });
    if (error) throw new Error(`Could not create smoke user: ${error.message}`);
  }

  const authClient = createClient(supabaseUrl, anonKey, {
    auth: {
      autoRefreshToken: false,
      persistSession: false,
      detectSessionInUrl: false,
    },
  });
  const {
    data: { session },
    error: signInError,
  } = await authClient.auth.signInWithPassword({ email, password });
  if (signInError || !session) {
    throw new Error(
      `Could not sign in the smoke user: ${signInError?.message || "no session returned"}`
    );
  }

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
