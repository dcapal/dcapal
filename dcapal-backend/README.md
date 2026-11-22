# DcaPal backend service

The backend uses TimescaleDB PostgreSQL through SQLx. The Compose `db` service
is the application database for local development, migrations, and backend
tests. Supabase is a separate authentication stack used by the full-stack
smoke test; its own PostgreSQL database does not receive DcaPal migrations.

## Run backend tests

The test suite uses real PostgreSQL databases through `sqlx::test`. Start the
database first, then run the tests:

```bash
make backend-db-up
make test-backend
make backend-db-down
```

`make test-backend` checks the configured TimescaleDB `DATABASE_URL` and fails
if the database is not already running. To apply pending migrations manually,
run:

```bash
make backend-migrate
```

## Startup and Yahoo reachability

Before the backend binds its HTTP port it seeds the eight fiat assets and the
seven FX markets (idempotent) and makes sure every FX market has a stored price.
When all seven already have a price, even a stale one, it starts immediately
without contacting any provider. Otherwise it fetches the missing prices from
Yahoo, retrying each ticker with exponential backoff (1 s up to 30 s) for at
most two minutes, and then exits with a non-zero status so the orchestrator
restarts it. Yahoo must therefore be reachable on the first boot against an
empty database only; later boots do not depend on it. See ADR 0007.

## Local development modes

The root Makefile owns the complete local setup. It bootstraps the pinned
tooling, starts local Supabase and TimescaleDB, applies migrations, and
renders the ignored `dcapal.yml`. It does not read configuration from another
checkout.

Run the backend as a host Rust process for the fastest edit-and-run loop:

```bash
make local-up
```

Run the host backend and frontend together at `http://localhost:3000`:

```bash
make local-up-ui
```

Build the backend as `dcapal-backend:local` and include it in the Compose
stack:

```bash
make local-docker-up
```

Run that Docker backend with the frontend:

```bash
make local-docker-up-ui
```

Stop the selected mode:

```bash
make local-down
make local-docker-down
```

`make local-reset` removes only this worktree's local Compose volumes and
containers. Use it when the persisted database was initialized with different
`POSTGRES_*` values; local setup never changes existing data automatically.

The default host ports are:

| Service | Host port |
| --- | ---: |
| Frontend | 3000 |
| Backend HTTP | 8080 |
| Backend metrics | 9000 |
| TimescaleDB | 5433 |

Set these values in the ignored `dcapal-backend/docker/local.env` when another
local service uses one of the ports. The backend receives `db:5432`
inside the Docker network, while a host-run backend uses the
published host ports.
