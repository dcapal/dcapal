FROM lukemathwalker/cargo-chef:latest-rust-1.65.0 AS chef
WORKDIR /var/dcapal

FROM chef AS planner
WORKDIR /var/dcapal
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./dcapal-backend ./dcapal-backend
COPY ./dcapal-optimizer-wasm ./dcapal-optimizer-wasm
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /var/dcapal
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./dcapal-backend ./dcapal-backend
COPY ./dcapal-optimizer-wasm ./dcapal-optimizer-wasm
COPY --from=planner /var/dcapal/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
RUN cargo build --profile release-with-debug --bin dcapal-backend

FROM debian:bullseye-slim AS runtime
# Install runtime dependencies
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update
RUN apt-get install -qq \
    ca-certificates \
    curl
# Prepare binary
WORKDIR /var/dcapal/dcapal-backend
RUN mkdir bin
RUN mkdir data
COPY --from=builder /var/dcapal/target/release/dcapal-backend /var/dcapal/dcapal-backend/bin
ENTRYPOINT ["/var/dcapal/dcapal-backend/bin/dcapal-backend"]