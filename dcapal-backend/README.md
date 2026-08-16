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

## How-to

### Run as a Docker container locally

`make local-up` renders the ignored `dcapal.yml` from the checked-in template,
using the local Supabase signing keys, and starts the application stack. Do
not copy the template directly: it contains renderer placeholders.

Build the backend image:

```bash
make docker-local-build
```

Start the container stack:

```bash
make local-up
```

Stop the container stack:

```bash
make local-down
```
