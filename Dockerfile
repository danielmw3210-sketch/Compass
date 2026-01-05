# =============================================================================
# Compass Blockchain - Production Dockerfile
# =============================================================================
# Supports both NODE and WORKER modes via build args / runtime args

# Build Stage
FROM rust:1.75-bookworm AS builder
WORKDIR /usr/src/app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    clang \
    cmake \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies by building with empty main first
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

# Build actual application
COPY . .
RUN cargo build --release

# =============================================================================
# Runtime Stage
# =============================================================================
FROM debian:bookworm-slim

# Labels
LABEL org.opencontainers.image.title="Compass Node"
LABEL org.opencontainers.image.description="Compass Blockchain Node"
LABEL org.opencontainers.image.version="1.0.0"

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false compass

WORKDIR /opt/compass

# Copy binary
COPY --from=builder /usr/src/app/target/release/rust_compass /usr/local/bin/
COPY --from=builder /usr/src/app/ai_runner.py /opt/compass/

# Create data directories
RUN mkdir -p /var/lib/compass /var/log/compass \
    && chown -R compass:compass /opt/compass /var/lib/compass /var/log/compass

# Ports
EXPOSE 19000 9000

# Healthcheck
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9000/health || exit 1

# Default: Run as node (override with docker run args for worker mode)
USER compass
ENTRYPOINT ["rust_compass"]
CMD ["node", "start", "--p2p-port", "19000", "--rpc-port", "9000", "--db-path", "/var/lib/compass/mainnet.db"]
