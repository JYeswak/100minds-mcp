# 100minds - Adversarial Decision Intelligence
# Multi-stage build for minimal image size

FROM rust:1.75-slim-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/

# Build release binary
RUN cargo build --release --bin 100minds --bin import

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries
COPY --from=builder /app/target/release/100minds /usr/local/bin/
COPY --from=builder /app/target/release/import /usr/local/bin/100minds-import

# Copy thinker data
COPY data/thinkers/ /app/data/thinkers/

# Initialize database on first run
ENV MINDS_DB_PATH=/app/data/wisdom.db

# Import thinkers and start server
ENTRYPOINT ["sh", "-c", "100minds-import /app/data/thinkers && exec 100minds \"$@\"", "--"]
CMD ["--serve", "--port=3100"]

EXPOSE 3100
