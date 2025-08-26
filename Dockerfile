FROM rust:1.80

RUN apt-get update && apt-get install -y \
    build-essential clang lld pkg-config libssl-dev \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /WorkingTano/occam-API

# Cache deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs
RUN cargo build --release --bin evm || true

# App source
COPY . .
RUN cargo build --release --bin evm

EXPOSE 4000

# Safe defaults; override with -e/--env-file
ENV SP1_PROVER=network
ENV NETWORK_PRIVATE_KEY=unset

CMD ["cargo", "run", "--release", "--bin", "evm"]