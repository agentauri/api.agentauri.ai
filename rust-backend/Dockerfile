# syntax=docker/dockerfile:1.4
# =============================================================================
# AgentAuri Backend - Multi-Stage Docker Build with Cargo Chef
# =============================================================================
# This Dockerfile builds all three Rust services with optimized caching:
# - api-gateway (main REST API server)
# - event-processor (blockchain event processing)
# - action-workers (trigger action execution)
#
# Uses cargo-chef for dependency caching - dependencies are only recompiled
# when Cargo.toml or Cargo.lock changes, not on every source code change.
#
# Build optimizations:
# - cargo-chef: Separates dependency compilation from source changes
# - thin LTO + codegen-units=4: Faster builds, lower memory usage
# - BuildKit cache: Persistent cargo registry cache between builds
#
# Usage:
#   docker build -t agentauri-backend .
#   docker run -p 8080:8080 agentauri-backend api-gateway
#   docker run agentauri-backend event-processor
#   docker run agentauri-backend action-workers
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Chef - Install cargo-chef tool
# -----------------------------------------------------------------------------
FROM rust:1.88-slim-bookworm AS chef

RUN cargo install cargo-chef
WORKDIR /app

# -----------------------------------------------------------------------------
# Stage 2: Planner - Generate dependency recipe
# -----------------------------------------------------------------------------
FROM chef AS planner

# Copy only what's needed to generate the recipe
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Generate recipe.json - a hash of all dependencies
RUN cargo chef prepare --recipe-path recipe.json

# -----------------------------------------------------------------------------
# Stage 3: Builder - Compile dependencies then source
# -----------------------------------------------------------------------------
FROM chef AS builder

# Install build dependencies
# Note: curl is needed for utoipa-swagger-ui to download Swagger UI assets at build time
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Configure Cargo for optimized CI/CD builds
# - Uses "thin" LTO instead of "fat" LTO (much less memory intensive)
# - codegen-units = 4: reduces peak memory, enables parallel compilation
# - jobs = 4: parallel Cargo jobs for faster builds
# - Still produces optimized binaries suitable for production
RUN mkdir -p .cargo && \
    printf '[profile.release]\nlto = "thin"\ncodegen-units = 4\n\n[build]\njobs = 4\n' > .cargo/config.toml

# Enable SQLx offline mode (uses cached query metadata)
ENV SQLX_OFFLINE=true

# Copy recipe from planner and build ONLY dependencies
# This layer is cached as long as Cargo.toml/Cargo.lock don't change
# BuildKit cache mounts persist cargo registry between builds
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --features aws-secrets --recipe-path recipe.json

# Now copy actual source code
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY .sqlx/ .sqlx/

# Build workspace binaries (dependencies already compiled above)
# Cache mounts provide incremental compilation benefits
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --features aws-secrets

# -----------------------------------------------------------------------------
# Stage 4: Runtime Environment
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies only
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN groupadd -r agentauri && useradd -r -g agentauri agentauri

# Copy compiled binaries from builder stage
COPY --from=builder /app/target/release/api-gateway /usr/local/bin/
COPY --from=builder /app/target/release/event-processor /usr/local/bin/
COPY --from=builder /app/target/release/action-workers /usr/local/bin/

# Set ownership
RUN chown -R agentauri:agentauri /usr/local/bin/api-gateway \
    /usr/local/bin/event-processor \
    /usr/local/bin/action-workers

# Switch to non-root user
USER agentauri

# Default port for API Gateway
EXPOSE 8080

# Health check for API Gateway
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/api/v1/health || exit 1

# Default command (can be overridden)
# Usage: docker run <image> [api-gateway|event-processor|action-workers]
ENTRYPOINT []
CMD ["api-gateway"]
