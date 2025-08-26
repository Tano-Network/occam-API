# ---------- Build Stage ----------
FROM rust:1.80 as builder

# Install succinct toolchain for SP1 SDK
RUN rustup install succinct && \
    rustup component add rust-src --toolchain succinct && \
    rustup default succinct

WORKDIR /WorkingTano/occam-API

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy entire project
COPY . .

# Build binary using succinct toolchain
RUN RUSTUP_TOOLCHAIN=succinct cargo build --release --bin evm


# ---------- Runtime Stage ----------
FROM debian:bullseye-slim AS runtime

# Install only required runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /WorkingTano/occam-API

# Copy binary from builder
COPY --from=builder /WorkingTano/occam-API/target/release/evm ./evm

# Default env (can override at runtime)
ENV SP1_PROVER=network

# Expose API port
EXPOSE 4000

# Start API
CMD ["./evm"]
