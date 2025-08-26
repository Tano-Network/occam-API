# ---------- Build Stage ----------
FROM rust:1.80 as builder

# Install required build dependencies for SP1 + actix-web + reqwest
RUN apt-get update && apt-get install -y \
    build-essential \
    clang \
    lld \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /WorkingTano/occam-API

# Copy manifests first
COPY Cargo.toml Cargo.lock ./

# Copy the rest of the project
COPY . .

# Build binary in release mode
RUN cargo build --release --bin evm


# ---------- Runtime Stage ----------
FROM debian:bullseye-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /WorkingTano/occam-API

# Copy binary from builder
COPY --from=builder /WorkingTano/occam-API/target/release/evm ./evm

# Set default env (can override at runtime)
ENV SP1_PROVER=network

# Expose API port
EXPOSE 4000

# Start API
CMD ["./evm"]
