/**
 * Ponder Telemetry Dashboard - Standalone Viewer
 *
 * Shows indexing progress grouped by chain by reading from PostgreSQL database.
 * Run this in a separate terminal alongside Ponder to see real-time stats.
 *
 * Usage:
 *   tsx telemetry-dashboard.ts
 */

import { createRequire } from "module";
const require = createRequire(import.meta.url);
const pg = require("pg");

const { Pool } = pg;

// Database connection (from DATABASE_URL env var)
const DATABASE_URL = process.env.DATABASE_URL;

if (!DATABASE_URL) {
  console.error("❌ ERROR: DATABASE_URL environment variable not set");
  console.error("Set it in your .env file or export it before running this script.");
  process.exit(1);
}

const pool = new Pool({
  connectionString: DATABASE_URL,
});

// Chain metadata
const CHAINS: Record<number, string> = {
  11155111: "ethereumSepolia",
  84532: "baseSepolia",
  59141: "lineaSepolia",
  80002: "polygonAmoy",
  1: "ethereumMainnet",
  8453: "baseMainnet",
  59144: "lineaMainnet",
};

interface ChainStats {
  name: string;
  chainId: number;
  currentBlock: number;
  totalEvents: number;
  events: Record<string, { count: number }>;
}

/**
 * Fetch current state from database
 */
async function fetchStats(): Promise<Map<number, ChainStats>> {
  const stats = new Map<number, ChainStats>();

  // Initialize chains
  for (const [chainId, name] of Object.entries(CHAINS)) {
    stats.set(Number(chainId), {
      name,
      chainId: Number(chainId),
      currentBlock: 0,
      totalEvents: 0,
      events: {},
    });
  }

  try {
    // Get current block per chain from checkpoints
    const checkpointResult = await pool.query(`
      SELECT "chainId", "lastBlockNumber"
      FROM "Checkpoint"
    `);

    for (const row of checkpointResult.rows) {
      const chain = stats.get(Number(row.chainId));
      if (chain) {
        chain.currentBlock = Number(row.lastBlockNumber);
      }
    }

    // Get event counts grouped by chain and event type
    const eventsResult = await pool.query(`
      SELECT
        "chainId",
        registry,
        "eventType",
        COUNT(*) as count
      FROM "Event"
      GROUP BY "chainId", registry, "eventType"
      ORDER BY "chainId", registry, "eventType"
    `);

    for (const row of eventsResult.rows) {
      const chain = stats.get(Number(row.chainId));
      if (chain) {
        const eventName = `${row.registry}:${row.eventType}`;
        chain.events[eventName] = { count: Number(row.count) };
        chain.totalEvents += Number(row.count);
      }
    }
  } catch (error) {
    console.error("Error fetching stats:", error);
  }

  return stats;
}

/**
 * Display formatted dashboard
 */
function displayDashboard(stats: Map<number, ChainStats>): void {
  // Clear screen and move cursor to top
  console.clear();

  console.log("\n┌─────────────────────────────────────────────────────────────┐");
  console.log("│              Ponder Indexer Telemetry Dashboard             │");
  console.log("└─────────────────────────────────────────────────────────────┘\n");

  // Sync Status Table
  console.log("Sync Status\n");
  console.log(
    `| ${"Network".padEnd(20)} | ${"Current Block".padEnd(15)} | ${"Total Events".padEnd(12)} |`
  );
  console.log(
    `|-${"-".repeat(20)}-|-${"-".repeat(15)}-|-${"-".repeat(12)}-|`
  );

  for (const chain of Array.from(stats.values())) {
    if (chain.currentBlock === 0 && chain.totalEvents === 0) continue;

    console.log(
      `| ${chain.name.padEnd(20)} | ${chain.currentBlock.toString().padEnd(15)} | ${chain.totalEvents.toString().padEnd(12)} |`
    );
  }

  console.log("\n");

  // Events grouped by chain
  console.log("Events by Chain\n");

  for (const chain of Array.from(stats.values())) {
    if (Object.keys(chain.events).length === 0) continue;

    console.log(`\n${chain.name} (Chain ${chain.chainId}):`);
    console.log(
      `| ${"Event".padEnd(40)} | ${"Count".padEnd(10)} |`
    );
    console.log(
      `|-${"-".repeat(40)}-|-${"-".repeat(10)}-|`
    );

    // Sort events by count (descending)
    const sortedEvents = Object.entries(chain.events).sort(
      ([, a], [, b]) => b.count - a.count
    );

    for (const [eventName, eventStats] of sortedEvents) {
      console.log(
        `| ${eventName.padEnd(40)} | ${eventStats.count.toString().padEnd(10)} |`
      );
    }
  }

  console.log("\n");

  // Footer
  const totalEvents = Array.from(stats.values()).reduce(
    (sum, chain) => sum + chain.totalEvents,
    0
  );

  console.log(`Total Events Indexed: ${totalEvents}`);
  console.log(`Refresh Rate: 2s`);
  console.log(`Database: ${DATABASE_URL.split("@")[1]?.split("?")[0] || "connected"}`);
  console.log("\nPress Ctrl+C to stop\n");
}

/**
 * Main loop
 */
async function main(): Promise<void> {
  console.log("🚀 Starting Ponder Telemetry Dashboard...\n");
  console.log(`Connecting to database: ${DATABASE_URL.split("@")[1]?.split("?")[0]}\n`);

  // Test connection
  try {
    await pool.query("SELECT 1");
    console.log("✅ Database connection successful\n");
  } catch (error) {
    console.error("❌ Database connection failed:", error);
    process.exit(1);
  }

  // Refresh every 2 seconds
  setInterval(async () => {
    const stats = await fetchStats();
    displayDashboard(stats);
  }, 2000);

  // Initial display
  const stats = await fetchStats();
  displayDashboard(stats);
}

// Graceful shutdown
process.on("SIGINT", async () => {
  console.log("\n\n👋 Shutting down telemetry dashboard...");
  await pool.end();
  process.exit(0);
});

process.on("SIGTERM", async () => {
  await pool.end();
  process.exit(0);
});

// Run
main().catch((error) => {
  console.error("Fatal error:", error);
  process.exit(1);
});
