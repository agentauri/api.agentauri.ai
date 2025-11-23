#!/bin/bash

# ============================================================================
# Ponder Indexers Setup Script
# ============================================================================
# This script sets up the Ponder indexers project
# ============================================================================

set -e

echo "========================================"
echo "Ponder Indexers Setup"
echo "========================================"

# Check if pnpm is installed
if ! command -v pnpm &> /dev/null; then
    echo "pnpm is not installed. Installing pnpm..."
    npm install -g pnpm@8.15.0
fi

echo "pnpm version: $(pnpm --version)"

# Check if .env exists
if [ ! -f .env ]; then
    echo "Creating .env file from .env.example..."
    cp .env.example .env
    echo "Please edit .env and add your RPC API keys and database URL"
else
    echo ".env file already exists"
fi

# Install dependencies
echo "Installing dependencies..."
pnpm install

# Run type check
echo "Running type check..."
pnpm typecheck || echo "Warning: Type check failed. You may need to add contract addresses."

echo ""
echo "========================================"
echo "Setup Complete!"
echo "========================================"
echo ""
echo "Next steps:"
echo "1. Edit .env and add your RPC API keys"
echo "2. Update contract addresses in ponder.config.ts"
echo "3. Ensure PostgreSQL is running (from root: docker-compose up -d)"
echo "4. Run 'pnpm dev' to start the indexer"
echo ""
echo "For more information, see README.md"
