# =============================================================================
# Development stage - with cargo-watch for hot reloading
# =============================================================================
FROM rust:1.91-slim-trixie AS dev

WORKDIR /app

# Install dependencies for building and development
RUN apt-get update && apt-get install -y \
  pkg-config \
  libssl-dev \
  curl \
  && rm -rf /var/lib/apt/lists/*

# Install cargo-watch for hot reloading
RUN cargo install cargo-watch

# Copy manifests first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy src to pre-build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only (this layer will be cached)
RUN cargo build && rm -rf src target/debug/inferxgate target/debug/deps/inferxgate-*

# Copy source code
COPY src ./src

# Expose port
EXPOSE 3000

# Run with cargo-watch for hot reloading
CMD ["cargo", "watch", "-x", "run"]

# =============================================================================
# Build stage - compile release binary
# =============================================================================
FROM rust:1.91-slim-trixie AS builder

WORKDIR /app

# Install dependencies for building
RUN apt-get update && apt-get install -y \
  pkg-config \
  libssl-dev \
  && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build for release
RUN cargo build --release

# =============================================================================
# Production stage - minimal runtime image
# =============================================================================
FROM debian:trixie-slim AS prod

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
  ca-certificates \
  libssl3 \
  curl \
  && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/inferxgate /app/inferxgate

# Create non-root user
RUN useradd -m -u 1000 appuser && chown -R appuser:appuser /app
USER appuser

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

# Run the binary
CMD ["./inferxgate"]
