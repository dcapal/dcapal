# DcaPal backend service

TBD

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