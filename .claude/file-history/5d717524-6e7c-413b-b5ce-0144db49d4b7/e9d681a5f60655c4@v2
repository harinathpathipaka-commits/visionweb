# ── Builder Stage ─────────────────────────────────────────────────────
FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependencies by copying manifests first
COPY Cargo.toml Cargo.lock* ./
COPY crates/ans-proto/Cargo.toml crates/ans-proto/
COPY crates/ans-core/Cargo.toml crates/ans-core/
COPY crates/ans-cdp/Cargo.toml crates/ans-cdp/
COPY crates/ans-daemon/Cargo.toml crates/ans-daemon/
COPY crates/ans-distill/Cargo.toml crates/ans-distill/
COPY crates/ans-diff/Cargo.toml crates/ans-diff/
COPY crates/ans-immune/Cargo.toml crates/ans-immune/
COPY crates/ans-goal/Cargo.toml crates/ans-goal/
COPY crates/ans-signal/Cargo.toml crates/ans-signal/
COPY crates/ans-ipc/Cargo.toml crates/ans-ipc/
COPY crates/ans-storage/Cargo.toml crates/ans-storage/
COPY crates/ans-budget/Cargo.toml crates/ans-budget/
COPY crates/ans-gateway/Cargo.toml crates/ans-gateway/

# Dummy source for dependency resolution
RUN mkdir -p crates/ans-proto/src crates/ans-core/src crates/ans-cdp/src \
    crates/ans-daemon/src crates/ans-distill/src crates/ans-diff/src \
    crates/ans-immune/src crates/ans-goal/src crates/ans-signal/src \
    crates/ans-ipc/src crates/ans-storage/src crates/ans-budget/src \
    crates/ans-gateway/src \
    && for d in crates/*/src; do echo 'fn main() {}' > "$d/lib.rs"; done \
    && echo 'fn main() {}' > crates/ans-daemon/src/main.rs \
    && echo 'fn main() {}' > crates/ans-proto/build.rs

RUN cargo build --release 2>/dev/null || true

# Copy real source
COPY crates/ crates/

# Build with touch to invalidate cached dummy
RUN touch crates/*/src/*.rs && cargo build --release

# ── Runtime Stage ──────────────────────────────────────────────────────
FROM debian:bookworm-slim

# Install Chromium for CDP browser control
RUN apt-get update && apt-get install -y --no-install-recommends \
    chromium \
    chromium-sandbox \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash ans && \
    mkdir -p /data && chown ans:ans /data

# Chrome environment
ENV CHROME_BIN=/usr/bin/chromium \
    CHROME_FLAGS="--headless --no-sandbox --disable-gpu --disable-dev-shm-usage" \
    ANS_GRPC_PORT=50051 \
    ANS_GATEWAY_PORT=50052

COPY --from=builder /build/target/release/ans-daemon /usr/local/bin/ans-daemon

USER ans
WORKDIR /data

EXPOSE 50051 50052

HEALTHCHECK --interval=15s --timeout=3s --retries=3 \
    CMD curl -sf http://localhost:${ANS_GATEWAY_PORT}/api/v1/health || exit 1

ENTRYPOINT ["ans-daemon"]
CMD ["--grpc-port", "50051", "--gateway-port", "50052"]
