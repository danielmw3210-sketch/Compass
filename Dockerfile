# Build Stage
FROM rust:latest AS builder
WORKDIR /usr/src/app
COPY . .
# Install clang/llvm for rocksdb if needed (usually standard image has build-essential)
RUN apt-get update && apt-get install -y clang cmake
RUN cargo install --path .

# Runtime Stage (Python + Rust Binary)
FROM python:3.11-slim
WORKDIR /usr/local/bin

# Install runtime deps
RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    pkg-config \
    sysstat \
    && rm -rf /var/lib/apt/lists/*

# Copy Binary
COPY --from=builder /usr/local/cargo/bin/rust_compass .
# Copy Python Runner
COPY --from=builder /usr/src/app/ai_runner.py .

# Expose ports (if needed, but worker is client-outbound mainly)
EXPOSE 9000 

# Default Command: Worker Mode
# Expects NODE_URL env var or args
ENTRYPOINT ["rust_compass", "worker"]
CMD ["--node-url", "http://host.docker.internal:9000", "--model", "gpt-4o-mini"]
