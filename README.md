<p align="center">
  <img src="https://ik.imagekit.io/dcapal/dcapal-og-bg-white-focused.png" width="800" />
</p>

<p align="center">
  <a href="https://github.com/leonardoarcari/dcapal/actions/workflows/build-test.yml"><img src="https://github.com/leonardoarcari/dcapal/actions/workflows/build-test.yml/badge.svg?branch=master"/></a>
</p>

## Build locally

**Start Docker environment**

```bash
$ cd dcapal-backend
$ docker compose -f docker-compose.yml -f docker/docker-compose.dev.yml up -d
```

**Run DcaPal backend**

```bash
$ cd dcapal-backend
$ cargo run --release
```

**Run DcaPal frontend**

- Build DcaPal Optimizer

```bash
$ cd dcapal-optimizer-wasm
$ curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
$ wasm-pack build --release
```

- Install NPM dependencies. Note: this is installing `dcapal-optimizer-wasm` package as well.

```bash![badge](https://user-images.githubusercontent.com/811969/224559373-35dc57a9-688d-49c9-a95f-df7d2bef7a47.svg)

$ cd ../dcapal-frontend
$ npm install # Install dcapal-optimizer-wasm pkg as well
```

- Run frontend server

```bash
$ npm run start
```
