#!/bin/bash
# Database Setup Script
# Automates the creation and migration of the api.agentauri.ai PostgreSQL database

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
DB_NAME="${DB_NAME:-agentauri_backend}"
DB_USER="${DB_USER:-postgres}"
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_PASSWORD="${DB_PASSWORD:-}"

# Parse command line arguments
SKIP_DB_CREATE=false
LOAD_SEEDS=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-create)
            SKIP_DB_CREATE=true
            shift
            ;;
        --with-seeds)
            LOAD_SEEDS=true
            shift
            ;;
        --help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --skip-create    Skip database creation (database already exists)"
            echo "  --with-seeds     Load test data after migrations"
            echo "  --help           Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  DB_NAME          Database name (default: agentauri_backend)"
            echo "  DB_USER          Database user (default: postgres)"
            echo "  DB_HOST          Database host (default: localhost)"
            echo "  DB_PORT          Database port (default: 5432)"
            echo "  DB_PASSWORD      Database password (optional)"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo -e "${GREEN}api.agentauri.ai Database Setup${NC}"
echo "================================"
echo ""

# Check if psql is installed
if ! command -v psql &> /dev/null; then
    echo -e "${RED}Error: psql not found. Please install PostgreSQL.${NC}"
    exit 1
fi

# Check if sqlx is installed
if ! command -v sqlx &> /dev/null; then
    echo -e "${YELLOW}Warning: sqlx CLI not found.${NC}"
    echo "Install with: cargo install sqlx-cli --no-default-features --features postgres"
    exit 1
fi

# Build connection string
if [ -n "$DB_PASSWORD" ]; then
    DATABASE_URL="postgresql://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}"
    PSQL_CONN="postgresql://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/postgres"
else
    DATABASE_URL="postgresql://${DB_USER}@${DB_HOST}:${DB_PORT}/${DB_NAME}"
    PSQL_CONN="postgresql://${DB_USER}@${DB_HOST}:${DB_PORT}/postgres"
fi

# Create database if needed
if [ "$SKIP_DB_CREATE" = false ]; then
    echo -e "${YELLOW}Creating database: ${DB_NAME}${NC}"

    # Check if database exists
    DB_EXISTS=$(psql "$PSQL_CONN" -tAc "SELECT 1 FROM pg_database WHERE datname='${DB_NAME}'")

    if [ "$DB_EXISTS" = "1" ]; then
        echo -e "${YELLOW}Database ${DB_NAME} already exists. Skipping creation.${NC}"
    else
        psql "$PSQL_CONN" -c "CREATE DATABASE ${DB_NAME};"
        echo -e "${GREEN}Database created successfully.${NC}"
    fi
else
    echo -e "${YELLOW}Skipping database creation (--skip-create flag set)${NC}"
fi

echo ""

# Check if TimescaleDB extension is available
echo -e "${YELLOW}Checking for TimescaleDB extension...${NC}"
TIMESCALE_EXISTS=$(psql "$DATABASE_URL" -tAc "SELECT 1 FROM pg_available_extensions WHERE name='timescaledb';" 2>/dev/null || echo "0")

if [ "$TIMESCALE_EXISTS" != "1" ]; then
    echo -e "${RED}Warning: TimescaleDB extension not found.${NC}"
    echo "Install TimescaleDB: https://docs.timescale.com/install/latest/"
    echo "Continuing anyway (migration will fail if TimescaleDB is required)..."
else
    echo -e "${GREEN}TimescaleDB extension found.${NC}"
fi

echo ""

# Run migrations
echo -e "${YELLOW}Running migrations...${NC}"
export DATABASE_URL
cd "$(dirname "$0")/.."
sqlx migrate run --source database/migrations

if [ $? -eq 0 ]; then
    echo -e "${GREEN}Migrations completed successfully.${NC}"
else
    echo -e "${RED}Migration failed!${NC}"
    exit 1
fi

echo ""

# Load seed data if requested
if [ "$LOAD_SEEDS" = true ]; then
    echo -e "${YELLOW}Loading test data...${NC}"
    psql "$DATABASE_URL" -f database/seeds/test_data.sql

    if [ $? -eq 0 ]; then
        echo -e "${GREEN}Test data loaded successfully.${NC}"
        echo ""
        echo "Test users created:"
        echo "  - alice@example.com (password: password123)"
        echo "  - bob@example.com (password: password123)"
        echo "  - charlie@example.com (password: password123)"
    else
        echo -e "${RED}Failed to load test data!${NC}"
        exit 1
    fi
fi

echo ""
echo -e "${GREEN}Database setup complete!${NC}"
echo ""
echo "Database URL: ${DATABASE_URL}"
echo ""
echo "To verify, run:"
echo "  psql \"${DATABASE_URL}\" -c \"\\dt\""
