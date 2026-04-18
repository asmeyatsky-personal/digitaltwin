# syntax=docker/dockerfile:1.7
# Multi-stage build for identity-service. Final image is distroless, no shell,
# runs as non-root (§4 supply-chain + least-privilege).

FROM rust:1.93-slim AS chef
RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked --version 0.1.67
WORKDIR /app

FROM chef AS planner
COPY backend ./backend
COPY contracts ./contracts
WORKDIR /app/backend
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/backend/recipe.json ./backend/recipe.json
WORKDIR /app/backend
RUN cargo chef cook --release --recipe-path recipe.json --bin identity-service
COPY backend ./
COPY contracts /app/contracts
RUN cargo build --release --bin identity-service

FROM gcr.io/distroless/cc-debian12:nonroot
WORKDIR /app
COPY --from=builder /app/backend/target/release/identity-service /usr/local/bin/identity-service
USER nonroot:nonroot
ENV PORT=8080 METRICS_BIND=0.0.0.0:9100
EXPOSE 8080 8081 9100
ENTRYPOINT ["/usr/local/bin/identity-service"]
