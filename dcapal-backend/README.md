# DcaPal backend service

The backend uses PostgreSQL through SQLx. Local PostgreSQL is provided by the
Supabase CLI, and the migration runner keeps the existing production tables in
place while creating SQLx migration history in `_sqlx_migrations`.

## Run backend tests

The test suite uses real PostgreSQL databases through `sqlx::test`. Start the
database first, then run the tests:

```bash
make backend-db-up
make test-backend
make backend-db-down
```

`make test-backend` checks the configured `DATABASE_URL` and fails if the
database is not already running. To apply pending migrations manually, run:

```bash
make backend-migrate
```

## How-to

### Run as Docker container locally

- Build backend image

```bash
make docker-local-build
```

- Update `dcapal.yml` config

```yml
app:
# App configs

server:
redis:
  hostname: redis # IMPORTANT!
  port: 6379
  user: dcapal
  password: dcapal
postgres:
  hostname: postgres # IMPORTANT!
  port: 5432 # IMPORTANT!
  user: postgres
  password: postgres
  database: postgres

```

- Start the container stack

```bash
make local-up
```

- Stop the container stack

```bash
make local-down
```
