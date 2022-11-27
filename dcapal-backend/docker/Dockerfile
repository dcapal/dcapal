FROM lukemathwalker/cargo-chef:latest-rust-1.65.0 AS chef
WORKDIR /var/dcapal-backend

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /var/dcapal-backend/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin dcapal-backend

FROM debian:bullseye-slim AS runtime
# Install runtime dependencies
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update
RUN apt-get install -qq \
    ca-certificates \
    curl
# Prepare binary
WORKDIR /var/dcapal-backend
RUN mkdir bin
COPY --from=builder /var/dcapal-backend/target/release/dcapal-backend /var/dcapal-backend/bin
ENTRYPOINT ["/var/dcapal-backend/bin/dcapal-backend"]