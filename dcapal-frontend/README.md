# DcaPal Frontend

The complete local setup is managed from the repository root. It bootstraps the
optimizer and JavaScript dependencies, starts local Supabase and the backend,
and passes the generated Supabase URL and anonymous key to the frontend.

For the normal UI development loop, run:

```shell
make local-up-ui
```

The frontend is available at `http://localhost:3000`. The Docker backend mode
uses the same frontend command and URL:

```shell
make local-docker-up-ui
```

The frontend watcher uses polling by default for reliable operation in fresh
worktrees and environments with a low file-descriptor limit. Set
`CHOKIDAR_USEPOLLING=false` when native watching is known to work.

To run only the frontend against an already-running backend, first run
`make bootstrap-local`, then:

```shell
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
service. Start the Docker-backed full stack with:

```shell
make local-docker-up-ui
```

The Make helper supplies the generated Supabase values automatically for the
development server. Capture them manually only when a test script needs the
values:

```shell
cd dcapal-backend
eval "$(XDG_CACHE_HOME=../.local/pnpm-cache \
  SUPABASE_HOME=../.local/supabase \
  SUPABASE_TELEMETRY_DISABLED=1 \
  pnpm dlx --package supabase@2.110.0 supabase status --workdir ./config -o env)"
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
