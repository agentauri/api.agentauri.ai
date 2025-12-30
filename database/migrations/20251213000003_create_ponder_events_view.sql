-- ============================================================================
-- Migration: Create View for Ponder Events (Column Mapping)
-- ============================================================================
-- This view provides a snake_case column interface to the Ponder Event table,
-- making it compatible with the existing Rust models and event-processor code.
--
-- Ponder creates tables with camelCase columns:
--   ponder."Event" with columns like chainId, blockNumber, eventType
--
-- The backend expects snake_case columns:
--   events with columns like chain_id, block_number, event_type
--
-- This view bridges the gap without modifying either side.
--
-- Created: 2025-12-13
-- Updated: 2025-12-25 (made conditional for local dev without Ponder)
-- ============================================================================

-- Conditional view creation - only if ponder."Event" exists
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables
        WHERE table_schema = 'ponder' AND table_name = 'Event'
    ) THEN
        -- Create the view that maps Ponder columns to snake_case
        CREATE OR REPLACE VIEW ponder_events AS
        SELECT
            id,
            "chainId" AS chain_id,
            "blockNumber" AS block_number,
            "blockHash" AS block_hash,
            "transactionHash" AS transaction_hash,
            "logIndex" AS log_index,
            registry,
            "eventType" AS event_type,
            "agentId" AS agent_id,
            timestamp,
            -- Identity Registry fields
            owner,
            "tokenUri" AS token_uri,
            "metadataKey" AS metadata_key,
            "metadataValue" AS metadata_value,
            -- Reputation Registry fields
            "clientAddress" AS client_address,
            "feedbackIndex" AS feedback_index,
            score,
            tag1,
            tag2,
            "fileUri" AS file_uri,
            "fileHash" AS file_hash,
            -- Validation Registry fields
            "validatorAddress" AS validator_address,
            "requestHash" AS request_hash,
            response,
            "responseUri" AS response_uri,
            "responseHash" AS response_hash,
            tag,
            -- Note: created_at doesn't exist in Ponder schema, use timestamp
            to_timestamp(timestamp) AS created_at
        FROM ponder."Event";

        COMMENT ON VIEW ponder_events IS
        'View mapping Ponder camelCase columns to snake_case for backend compatibility.
        Use this view instead of directly querying ponder."Event".';

        -- Update the unprocessed_events view to use ponder_events
        CREATE OR REPLACE VIEW unprocessed_events AS
        SELECT
            e.id,
            e.chain_id,
            e.block_number,
            e.registry,
            e.event_type,
            e.created_at,
            EXTRACT(EPOCH FROM (NOW() - e.created_at)) AS age_seconds
        FROM ponder_events e
        WHERE NOT EXISTS (
            SELECT 1 FROM processed_events pe WHERE pe.event_id = e.id
        )
        ORDER BY e.created_at ASC, e.id ASC;

        RAISE NOTICE 'Ponder views created successfully';
    ELSE
        RAISE NOTICE 'Skipping Ponder views creation - ponder."Event" table does not exist yet';
    END IF;
END $$;
