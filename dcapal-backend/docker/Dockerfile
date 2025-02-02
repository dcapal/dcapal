FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef
WORKDIR /var/dcapal

FROM chef AS planner
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./dcapal-backend ./dcapal-backend
COPY ./dcapal-optimizer-wasm ./dcapal-optimizer-wasm
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# Install sqlx-cli
RUN cargo install sqlx-cli --no-default-features --features postgres
RUN cargo install sea-orm-cli
# Build dependencies
COPY --from=planner /var/dcapal/recipe.json recipe.json
RUN cargo chef cook --profile release-with-debug --recipe-path recipe.json
# Build application
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./dcapal-backend ./dcapal-backend
COPY ./dcapal-optimizer-wasm ./dcapal-optimizer-wasm
RUN cargo build --profile release-with-debug --bin dcapal-backend --bin migration

FROM debian:bookworm-slim AS runtime
# Install runtime dependencies
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update
RUN apt-get install -y ca-certificates curl libssl3 jq postgresql-client
# Prepare binary and migrations
WORKDIR /var/dcapal/dcapal-backend
RUN mkdir bin
RUN mkdir data
COPY --from=builder /var/dcapal/target/release-with-debug/dcapal-backend /var/dcapal/dcapal-backend/bin
COPY --from=builder /var/dcapal/target/release-with-debug/migration /var/dcapal/dcapal-backend/bin
COPY --from=builder /usr/local/cargo/bin/sqlx /usr/local/bin/sqlx
COPY --from=builder /usr/local/cargo/bin/sea-orm-cli /usr/local/bin/sea-orm-cli
COPY ./dcapal-backend/migration ./migration
COPY ./dcapal-backend/migration/Cargo.toml ./migration/Cargo.toml
COPY ./dcapal-backend/migration/src ./migration/src
# Create start script
COPY ./dcapal-backend/scripts/start.sh ./start.sh
RUN chmod +x ./start.sh
CMD ["./start.sh"]