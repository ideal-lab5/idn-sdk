# =============================================================================
# IDN-SDK Multi-Platform Dockerfile
# =============================================================================
# Builds idn-node or idn-consumer-node for linux/amd64 and linux/arm64.
#
# Build args:
#   NODE_PACKAGE: Cargo package to build (idn-node or idn-consumer-node)
#
# Example:
#   docker buildx build --platform linux/amd64,linux/arm64 \
#     --build-arg NODE_PACKAGE=idn-node -t ideallabs/idn-node:latest .
#
# Recommended ports:
#   30333 - Parachain P2P
#   30343 - Relay chain P2P (embedded)
#   9944  - WebSocket RPC
#   9615  - Prometheus metrics
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Chef - Base with cargo-chef and build dependencies
# -----------------------------------------------------------------------------
FROM rust:1.90-bookworm AS chef

RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    clang \
    libclang-dev \
    libssl-dev \
    pkg-config \
    cmake \
    make \
    g++ \
    git \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef --locked

WORKDIR /build

# -----------------------------------------------------------------------------
# Stage 2: Planner - Generate dependency recipe
# -----------------------------------------------------------------------------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# -----------------------------------------------------------------------------
# Stage 3: Builder - Build dependencies (cached) then build binary
# -----------------------------------------------------------------------------
FROM chef AS builder

ARG NODE_PACKAGE=idn-node
ARG GIT_COMMIT=unknown

ENV CARGO_INCREMENTAL=0
ENV SUBSTRATE_CLI_GIT_COMMIT_HASH=${GIT_COMMIT}

RUN rustup target add wasm32v1-none && rustup component add rust-src

COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json -p ${NODE_PACKAGE}

COPY . .
RUN cargo build --release -p ${NODE_PACKAGE}

# -----------------------------------------------------------------------------
# Stage 4: Runtime - Minimal production image
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

ARG NODE_PACKAGE=idn-node

LABEL org.opencontainers.image.title="${NODE_PACKAGE}" \
      org.opencontainers.image.description="Ideal Network blockchain node" \
      org.opencontainers.image.vendor="Ideal Labs" \
      org.opencontainers.image.source="https://github.com/aspect-labs/idn-sdk" \
      org.opencontainers.image.licenses="Apache-2.0"

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 -U -s /bin/sh -d /node node && \
    mkdir -p /node/data && chown -R node:node /node

WORKDIR /node

COPY --from=builder /build/target/release/${NODE_PACKAGE} /usr/local/bin/node
RUN chmod +x /usr/local/bin/node

USER node

VOLUME ["/node/data"]

# Health check requires RPC to be exposed (--rpc-port 9944)
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -sf http://localhost:9944/health || exit 1

ENTRYPOINT ["/usr/local/bin/node"]
CMD ["--help"]
