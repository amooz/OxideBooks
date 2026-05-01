# ─── Build stage ──────────────────────────────────────────────────────────────
FROM rust:1.82-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Cache dependency compilation separately from source compilation.
COPY Cargo.toml Cargo.lock ./
COPY crates/core/Cargo.toml   crates/core/Cargo.toml
COPY crates/db/Cargo.toml     crates/db/Cargo.toml
COPY crates/api/Cargo.toml    crates/api/Cargo.toml

# Stub src files so Cargo can resolve the workspace graph without full source.
RUN mkdir -p crates/core/src crates/db/src crates/api/src && \
    echo "pub fn stub() {}" > crates/core/src/lib.rs && \
    echo "pub fn stub() {}" > crates/db/src/lib.rs && \
    echo "fn main() {}"    > crates/api/src/main.rs

RUN cargo build --release --package oxidebooks-api 2>/dev/null || true

# Now copy real source and rebuild (only changed crates recompile).
COPY crates/ crates/

# Touch to bust the stub cache.
RUN touch crates/core/src/lib.rs crates/db/src/lib.rs crates/api/src/main.rs

RUN cargo build --release --package oxidebooks-api

# ─── Runtime stage ────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

RUN groupadd -r oxidebooks && useradd -r -g oxidebooks oxidebooks

COPY --from=builder /build/target/release/oxidebooks /usr/local/bin/oxidebooks

USER oxidebooks
WORKDIR /app

EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/oxidebooks"]
