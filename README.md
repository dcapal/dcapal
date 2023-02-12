# DcaPal - A smart assistant for your periodic investments

TBD

## Run DcaPal locally

### Start Docker environment

```bash
$ cd dcapal-backend
$ docker compose -f docker-compose.yml -f docker/docker-compose.dev.yml up -d
```

### Run DcaPal backend

```bash
$ cd dcapal-backend
$ cargo run --release
```

### Run DcaPal frontend

- Build DcaPal Optimizer

```bash
$ cd dcapal-optimizer-wasm
$ curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
$ wasm-pack build --release
```

- Install NPM dependencies. Note: this is installing `dcapal-optimizer-wasm` package as well.

```bash
$ cd ../dcapal-frontend
$ npm install # Install dcapal-optimizer-wasm pkg as well
```

- Run frontend server

```bash
$ npm run start
```

- Run frontend e2e test
```bash
$ npm run cypress
```
