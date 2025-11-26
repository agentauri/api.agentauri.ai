/* eslint-disable @typescript-eslint/no-unsafe-assignment */
/* eslint-disable @typescript-eslint/no-unsafe-call */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */
/* eslint-disable @typescript-eslint/no-unsafe-return */
/**
 * Ponder API Routes
 *
 * Note: ESLint unsafe rules are disabled because Ponder's dynamic API
 * doesn't provide complete TypeScript type definitions for route handlers.
 * The handlers are strongly typed at runtime by Ponder.
 */
import { ponder } from "@ponder/core";

// ============================================================================
// GRAPHQL API
// ============================================================================
// Ponder automatically generates a GraphQL API based on the schema.
// This file exports custom resolvers and extends the default API.

/**
 * Custom GraphQL API extensions
 *
 * The default Ponder API provides:
 * - Query events by all indexed fields
 * - Filter by chainId, registry, agentId, etc.
 * - Pagination with cursor-based navigation
 * - Sorting by any field
 *
 * Example queries:
 *
 * 1. Get all events for a specific agent:
 *    query {
 *      events(where: { agentId: "42" }) {
 *        items {
 *          id
 *          chainId
 *          registry
 *          eventType
 *          timestamp
 *        }
 *      }
 *    }
 *
 * 2. Get reputation events on Base Sepolia:
 *    query {
 *      events(where: { chainId: "84532", registry: "reputation" }) {
 *        items {
 *          agentId
 *          eventType
 *          score
 *          timestamp
 *        }
 *      }
 *    }
 *
 * 3. Get latest events across all chains:
 *    query {
 *      events(orderBy: "timestamp", orderDirection: "desc", limit: 100) {
 *        items {
 *          chainId
 *          registry
 *          eventType
 *          agentId
 *          timestamp
 *        }
 *      }
 *    }
 *
 * 4. Get checkpoints for all chains:
 *    query {
 *      checkpoints {
 *        items {
 *          chainId
 *          lastBlockNumber
 *        }
 *      }
 *    }
 */

// You can add custom GraphQL resolvers here if needed
// For now, the auto-generated API is sufficient

ponder.use("/graphql", (c) => {
  // Custom middleware can be added here
  return c;
});

// Health check endpoint
ponder.use("/health", (c) => {
  return c.text("OK");
});

// Status endpoint with indexing stats
ponder.use("/status", async (c) => {
  const { Event, Checkpoint } = c.tables;

  try {
    // Get total event count
    const events = await Event.findMany({ limit: 1 });

    // Get all checkpoints
    const checkpoints = await Checkpoint.findMany();

    return c.json({
      status: "healthy",
      indexer: {
        totalEvents: events.length,
        chains: checkpoints.items.map((cp) => ({
          chainId: cp.chainId.toString(),
          lastBlockNumber: cp.lastBlockNumber.toString(),
          lastBlockHash: cp.lastBlockHash,
        })),
      },
      timestamp: new Date().toISOString(),
    });
  } catch (error) {
    return c.json(
      {
        status: "error",
        error: error instanceof Error ? error.message : "Unknown error",
        timestamp: new Date().toISOString(),
      },
      500
    );
  }
});

export default ponder;
