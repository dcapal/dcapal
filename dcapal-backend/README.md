# DcaPal backend service

TBD

## How-to

### Run as Docker container locally

- Build backend image

```bash
cd dcapal-backend
docker compose -f docker-compose.yml -f ./docker/docker-compose.local.yml build
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
```

- Start the container stack

```bash
docker compose -f docker-compose.yml -f ./docker/docker-compose.local.yml up -d
```