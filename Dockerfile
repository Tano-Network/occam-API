FROM rust:1.80

# Install required system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    clang \
    lld \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /WorkingTano/occam-API

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy full source code
COPY . .

# Expose port
EXPOSE 4000

# Hardcoded env + command
CMD SP1_PROVER=network NETWORK_PRIVATE_KEY=9d2fe65604d872ea2b45f7dd48c49d4a83984f11e2f179275a65b98fe84d4899 cargo run --release --bin evm
