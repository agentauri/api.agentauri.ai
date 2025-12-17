# =============================================================================
# Ponder Indexers Dockerfile
# =============================================================================
# Multi-stage build for production-ready Ponder blockchain indexer
#
# Build from project ROOT (monorepo structure):
#   docker build -t ponder-indexers -f ponder-indexers/Dockerfile .
#
# Run:
#   docker run -e DATABASE_URL=... -e PONDER_RPC_URL_1=... ponder-indexers

# -----------------------------------------------------------------------------
# Stage 1: Dependencies
# -----------------------------------------------------------------------------
FROM node:20-alpine AS deps

# Install pnpm
RUN corepack enable && corepack prepare pnpm@8.15.0 --activate

WORKDIR /app

# Copy workspace and package files from monorepo root
# Root package.json contains pnpm overrides that must match lockfile
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY ponder-indexers/package.json ./ponder-indexers/

# Install dependencies for ponder-indexers only
RUN pnpm install --frozen-lockfile --filter ponder-indexers

# -----------------------------------------------------------------------------
# Stage 2: Builder
# -----------------------------------------------------------------------------
FROM node:20-alpine AS builder

# Install pnpm
RUN corepack enable && corepack prepare pnpm@8.15.0 --activate

WORKDIR /app

# Copy workspace config
COPY pnpm-lock.yaml pnpm-workspace.yaml ./

# Copy dependencies from deps stage
COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/ponder-indexers/node_modules ./ponder-indexers/node_modules

# Copy ponder-indexers source files
COPY ponder-indexers ./ponder-indexers

# Generate Ponder types (Ponder compiles on start, no separate build step needed)
WORKDIR /app/ponder-indexers
RUN pnpm codegen

# -----------------------------------------------------------------------------
# Stage 3: Production
# -----------------------------------------------------------------------------
FROM node:20-alpine AS production

# Install pnpm for running start command
RUN corepack enable && corepack prepare pnpm@8.15.0 --activate

# Add non-root user for security
RUN addgroup -g 1001 -S ponder && \
    adduser -S ponder -u 1001 -G ponder

# Add AWS RDS CA certificate bundle for proper TLS validation
# This allows Node.js to verify RDS PostgreSQL SSL certificates
COPY ponder-indexers/rds-ca-bundle.pem /etc/ssl/certs/rds-ca-bundle.pem
ENV NODE_EXTRA_CA_CERTS=/etc/ssl/certs/rds-ca-bundle.pem

WORKDIR /app

# Copy workspace structure for pnpm to work correctly
COPY --from=builder /app/ponder-indexers/package.json ./ponder-indexers/
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./

# Copy node_modules (both root and workspace)
COPY --from=deps /app/node_modules ./node_modules
COPY --from=deps /app/ponder-indexers/node_modules ./ponder-indexers/node_modules

# Copy application files (Ponder compiles TypeScript on start)
COPY --from=builder /app/ponder-indexers/ponder.config.ts ./ponder-indexers/
COPY --from=builder /app/ponder-indexers/ponder.schema.ts ./ponder-indexers/
COPY --from=builder /app/ponder-indexers/src ./ponder-indexers/src
COPY --from=builder /app/ponder-indexers/abis ./ponder-indexers/abis

# Change ownership to non-root user
RUN chown -R ponder:ponder /app

USER ponder

# Environment variables (to be provided at runtime)
ENV NODE_ENV=production
ENV PONDER_LOG_LEVEL=info

# Health check - Ponder exposes metrics on port 42069 by default
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:42069/health || exit 1

# Expose Ponder's default port
EXPOSE 42069

# Work from ponder-indexers directory
WORKDIR /app/ponder-indexers

# Start Ponder via pnpm filter to use workspace properly
CMD ["pnpm", "start"]
