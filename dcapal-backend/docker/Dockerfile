# syntax=docker/dockerfile:1.7

# Bookworm, so it's the same as the runtime
FROM rust:bookworm AS chef
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    cargo install cargo-chef
WORKDIR /var/dcapal

FROM chef AS planner
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./dcapal-backend ./dcapal-backend
COPY ./dcapal-optimizer-wasm ./dcapal-optimizer-wasm
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# Build dependencies
COPY --from=planner /var/dcapal/recipe.json recipe.json
# Install rquest dependencies
RUN apt-get update
RUN apt-get install build-essential cmake perl pkg-config libclang-dev musl-tools git -y

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/var/dcapal/target \
    cargo chef cook --profile release-with-debug --recipe-path recipe.json
# Build application
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./dcapal-backend ./dcapal-backend
COPY ./dcapal-optimizer-wasm ./dcapal-optimizer-wasm
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/var/dcapal/target \
    cargo build --profile release-with-debug --bin dcapal-backend --bin migration \
    && mkdir -p /var/dcapal/output \
    && cp /var/dcapal/target/release-with-debug/dcapal-backend /var/dcapal/output/dcapal-backend \
    && cp /var/dcapal/target/release-with-debug/migration /var/dcapal/output/migration

FROM debian:bookworm-slim AS runtime
# Install runtime dependencies
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update
RUN apt-get install -y ca-certificates curl libssl3 jq postgresql-client
# Prepare binary and migrations
WORKDIR /var/dcapal/dcapal-backend
RUN mkdir bin
RUN mkdir data
COPY --from=builder /var/dcapal/output/dcapal-backend /var/dcapal/dcapal-backend/bin
COPY --from=builder /var/dcapal/output/migration /var/dcapal/dcapal-backend/bin
# Create start script
COPY ./dcapal-backend/scripts/start.sh ./start.sh
RUN chmod +x ./start.sh
ENTRYPOINT ["./start.sh"]
