# DcaPal Frontend

Set up the [local Supabase environment](https://supabase.com/docs/guides/local-development):

```shell
make supabase-up
```

Once the docker instance is started, copy the displayed anon key and replace the `VITE_SUPABASE_ANON_KEY` in the
`.env`
file.

```dotenv
REACT_APP_ENABLE_COOKIE_BUTTON=0
VITE_SUPABASE_URL=http://127.0.0.1:54321
VITE_SUPABASE_ANON_KEY=<anon_key>
```

From the repository root, build the local optimizer package, install the workspace dependencies, and run the frontend server:

```shell
(cd dcapal-optimizer-wasm/crates/optimizer && wasm-pack build --dev --out-dir ../../pkg)
pnpm install --frozen-lockfile
pnpm frontend:dev
```

After the root install, the frontend package can also be run from its directory:

```shell
cd dcapal-frontend
pnpm dev
```

## E2E tests

The default Playwright journeys use deterministic MSW fixtures for backend APIs
(`/api/assets/*`, `/api/price/*`, import endpoints, and sync endpoints):

```shell
pnpm frontend:test:e2e
```

These smoke tests do not require local backend containers.

The full-stack browser smoke uses the browser, local Supabase Auth, the
frontend development server, the backend container, and the TimescaleDB Compose
service.
Start the local full stack with one command. It starts Supabase, reads its
signing keys, renders the ignored backend `dcapal.yml`, and starts the
TimescaleDB, Redis, and backend containers:

```shell
make local-up
```

Capture the local Supabase values and export them before running Playwright:

```shell
cd dcapal-backend
eval "$(npx supabase status --workdir ./config -o env)"
export SUPABASE_URL="$API_URL"
export SUPABASE_ANON_KEY="$ANON_KEY"
export SUPABASE_SERVICE_ROLE_KEY="$SERVICE_ROLE_KEY"
export VITE_SUPABASE_URL="$API_URL"
export VITE_SUPABASE_ANON_KEY="$ANON_KEY"
cd ..
```

The service-role key is used only by the local test setup to create the
disposable smoke user. Supabase's own PostgreSQL instance is not used for
DcaPal migrations or application data.

Run the full-stack browser smoke with:

```shell
pnpm frontend:test:e2e:smoke
```

The journey seeds one local portfolio, waits for the frontend's actual
`POST /api/v1/sync/portfolios` request, and verifies that Supabase Auth accepts
the browser session. In CI, the following Timescale assertion runs after the
browser test:

```shell
SMOKE_USER_EMAIL=smoke@example.com \
SMOKE_PORTFOLIO_ID=11111111-1111-4111-8111-111111111111 \
SMOKE_PORTFOLIO_NAME="Smoke portfolio" \
dcapal-backend/scripts/assert-smoke-data.sh
```
