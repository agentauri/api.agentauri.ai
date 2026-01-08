#!/bin/bash
# =============================================================================
# ERC-8004 v1.0 Migration - Ponder Data Reset Script
# =============================================================================
# This script resets Ponder indexer data for the v0.4 -> v1.0 migration.
# Run this BEFORE starting Ponder with the new v1.0 ABIs.
#
# What gets reset:
# 1. Ponder cache directory (.ponder/)
# 2. Database tables: Event, Checkpoint
# 3. Ponder internal schemas (ponder_*)
#
# Usage:
#   ./scripts/reset-ponder-v1.sh
#
# Prerequisites:
#   - docker-compose up -d (or local PostgreSQL running)
#   - DATABASE_URL set in .env.local
# =============================================================================

set -e

echo "========================================"
echo "ERC-8004 v1.0 - Ponder Data Reset"
echo "========================================"
echo ""

# Load environment variables
if [ -f .env.local ]; then
    export $(grep -E '^DATABASE_URL=' .env.local | xargs)
fi

if [ -z "$DATABASE_URL" ]; then
    echo "ERROR: DATABASE_URL not set. Please ensure .env.local exists with DATABASE_URL."
    exit 1
fi

echo "Step 1: Removing local Ponder cache..."
if [ -d ".ponder" ]; then
    rm -rf .ponder
    echo "  ✓ Removed .ponder directory"
else
    echo "  - No .ponder directory found (OK)"
fi

echo ""
echo "Step 2: Resetting database tables..."

# Extract database connection details
# DATABASE_URL format: postgresql://user:password@host:port/database
DB_HOST=$(echo $DATABASE_URL | sed -E 's/.*@([^:]+):.*/\1/')
DB_PORT=$(echo $DATABASE_URL | sed -E 's/.*:([0-9]+)\/.*/\1/')
DB_NAME=$(echo $DATABASE_URL | sed -E 's/.*\/([^?]+).*/\1/')
DB_USER=$(echo $DATABASE_URL | sed -E 's/.*\/\/([^:]+):.*/\1/')
DB_PASS=$(echo $DATABASE_URL | sed -E 's/.*:([^@]+)@.*/\1/')

# SQL to reset Ponder data
SQL_RESET=$(cat << 'EOF'
-- ERC-8004 v1.0 Migration: Reset Ponder indexed data
-- This removes v0.4 data which is incompatible with v1.0

BEGIN;

-- Drop Ponder event and checkpoint tables if they exist
DROP TABLE IF EXISTS "Event" CASCADE;
DROP TABLE IF EXISTS "Checkpoint" CASCADE;

-- Drop any Ponder internal schemas (created by Ponder 0.7.x)
DO $$
DECLARE
    schema_name TEXT;
BEGIN
    FOR schema_name IN
        SELECT nspname FROM pg_namespace
        WHERE nspname LIKE 'ponder%'
    LOOP
        EXECUTE 'DROP SCHEMA IF EXISTS ' || quote_ident(schema_name) || ' CASCADE';
        RAISE NOTICE 'Dropped schema: %', schema_name;
    END LOOP;
END $$;

-- Also clean up any replication slots Ponder might have created
-- (only if they exist and are not in use)
DO $$
DECLARE
    slot_name TEXT;
BEGIN
    FOR slot_name IN
        SELECT slot_name FROM pg_replication_slots
        WHERE slot_name LIKE 'ponder%' AND active = false
    LOOP
        EXECUTE 'SELECT pg_drop_replication_slot(' || quote_literal(slot_name) || ')';
        RAISE NOTICE 'Dropped replication slot: %', slot_name;
    END LOOP;
EXCEPTION WHEN OTHERS THEN
    -- Ignore errors if no slots exist or insufficient permissions
    NULL;
END $$;

COMMIT;

-- Verify cleanup
SELECT 'Tables remaining:' AS info, count(*) AS count
FROM information_schema.tables
WHERE table_name IN ('Event', 'Checkpoint');

SELECT 'Ponder schemas remaining:' AS info, count(*) AS count
FROM pg_namespace
WHERE nspname LIKE 'ponder%';
EOF
)

echo "$SQL_RESET" | PGPASSWORD="$DB_PASS" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -v ON_ERROR_STOP=1

if [ $? -eq 0 ]; then
    echo "  ✓ Database tables reset successfully"
else
    echo "  ✗ Database reset failed. Check connection and permissions."
    exit 1
fi

echo ""
echo "========================================"
echo "Reset complete!"
echo "========================================"
echo ""
echo "Next steps:"
echo "  1. Start Ponder: pnpm dev"
echo "  2. Ponder will re-index from block 9989393 (v1.0 deployment)"
echo "  3. Monitor logs for any errors"
echo ""
echo "New contract addresses (Ethereum Sepolia):"
echo "  Identity:   0x8004A818BFB912233c491871b3d84c89A494BD9e"
echo "  Reputation: 0x8004B663056A597Dffe9eCcC1965A193B7388713"
echo "  Validation: Not yet deployed"
echo ""
