# SQLx migration runner

The migration crate embeds the SQL files in `dcapal-backend/migrations` and
runs them against the URL in `DATABASE_URL`:

```sh
DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:5433/postgres \
  cargo run -p migration
```

The deployment startup script also supports the existing explicit form:

```sh
cargo run -p migration -- up -u "$DATABASE_URL"
```

The shared asset migration creates `assets_data` records, and the following
identity migration rewrites Portfolio Asset relationship IDs with PostgreSQL
18 `uuidv7()`. They must run on PostgreSQL 18 or newer.

The SQLx runner records its history in `_sqlx_migrations`. It does not modify
the existing SeaORM `seaql_migrations` table. The historical up migrations are
idempotent so they can adopt tables already created in production.

Migration files are added manually with a numeric timestamp prefix and paired
`.up.sql` and `.down.sql` files. Down migrations are for local development and
test reset only. The shared asset migration has a development-only down path;
production recovery remains forward-only and uses the pre-deployment backup.
