# =============================================================================
# AgentAuri Backend - Multi-Stage Docker Build
# =============================================================================
# This Dockerfile builds all three Rust services:
# - api-gateway (main REST API server)
# - event-processor (blockchain event processing)
# - action-workers (trigger action execution)
#
# Usage:
#   docker build -t agentauri-backend .
#   docker run -p 8080:8080 agentauri-backend api-gateway
#   docker run agentauri-backend event-processor
#   docker run agentauri-backend action-workers
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build Environment
# -----------------------------------------------------------------------------
FROM rust:1.88-slim-bookworm AS builder

# Install build dependencies
# Note: curl is needed for utoipa-swagger-ui to download Swagger UI assets at build time
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace configuration first (for better layer caching)
COPY Cargo.toml Cargo.lock ./

# Copy all crates
COPY crates/ crates/

# Copy SQLx offline query data (required for compile-time SQL verification)
COPY .sqlx/ .sqlx/

# Configure Cargo for memory-efficient release builds
# - Uses "thin" LTO instead of "fat" LTO (much less memory intensive)
# - Increases codegen-units to reduce peak memory during compilation
# - Still produces optimized binaries suitable for production
RUN mkdir -p .cargo && echo '[profile.release]\nlto = "thin"\ncodegen-units = 4' > .cargo/config.toml

# Enable SQLx offline mode (uses cached query metadata)
ENV SQLX_OFFLINE=true

# Build all binaries in release mode with AWS Secrets Manager support
# This produces optimized binaries for production use
RUN cargo build --release --features aws-secrets

# -----------------------------------------------------------------------------
# Stage 2: Runtime Environment
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
