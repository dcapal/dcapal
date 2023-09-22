FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef
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
RUN cargo chef cook --profile release-with-debug --recipe-path recipe.json
# Build application
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./dcapal-backend ./dcapal-backend
COPY ./dcapal-optimizer-wasm ./dcapal-optimizer-wasm
RUN cargo build --profile release-with-debug --bin dcapal-backend

FROM debian:bookworm-slim AS runtime
# Install runtime dependencies
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update
RUN apt-get install -qq \
    ca-certificates \
    curl \
    libssl3 \
    jq
# Prepare binary
WORKDIR /var/dcapal/dcapal-backend
RUN mkdir bin
RUN mkdir data
COPY --from=builder /var/dcapal/target/release-with-debug/dcapal-backend /var/dcapal/dcapal-backend/bin
ENTRYPOINT ["/var/dcapal/dcapal-backend/bin/dcapal-backend"]