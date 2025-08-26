# ---------- Build Stage ----------
FROM rust:1.80 as builder

# Set workdir as per your server structure
WORKDIR /WorkingTano/occam-API

# Copy manifest files first for caching
COPY Cargo.toml Cargo.lock ./

# Copy entire project
COPY . .

# Build the binary (release mode)
RUN cargo build --release --bin evm


# ---------- Runtime Stage ----------
FROM debian:bullseye-slim AS runtime

# Install only required runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Match working directory with your setup
WORKDIR /WorkingTano/occam-API

# Copy compiled binary from builder
COPY --from=builder /WorkingTano/occam-API/target/release/evm ./evm

# Set environment variables (can override at runtime)
ENV SP1_PROVER=network

# Expose the API port
EXPOSE 4000

# Run the server
CMD ["./evm"]
