/**
 * Custom Telemetry for Ponder Indexers
 *
 * Provides enhanced visualization of indexing progress with:
 * - Current block per chain
 * - Events grouped by chain
 */

interface ChainStats {
  name: string;
  chainId: number;
  currentBlock: number;
  targetBlock: number;
  isRealtime: boolean;
  rpcRate: number;
  events: Record<string, EventStats>;
}

interface EventStats {
  count: number;
  durationMs: number;
}

class PonderTelemetry {
  private chains: Map<number, ChainStats> = new Map();
  private displayInterval: NodeJS.Timeout | null = null;
  private readonly DISPLAY_REFRESH_MS = 2000; // Refresh every 2 seconds

  constructor() {
    // Chain metadata (matches ponder.config.ts)
    const chainMetadata: Record<number, string> = {
      11155111: "ethereumSepolia",
      84532: "baseSepolia",
      59141: "lineaSepolia",
      80002: "polygonAmoy",
      1: "ethereumMainnet",
      8453: "baseMainnet",
      59144: "lineaMainnet",
    };

    // Initialize chain tracking
    for (const [chainId, name] of Object.entries(chainMetadata)) {
      this.chains.set(Number(chainId), {
        name,
        chainId: Number(chainId),
        currentBlock: 0,
        targetBlock: 0,
        isRealtime: false,
        rpcRate: 0,
        events: {},
      });
    }
  }

  /**
   * Update chain sync status
   */
  updateChainSync(
    chainId: number,
    currentBlock: number,
    targetBlock: number,
    isRealtime: boolean,
    rpcRate: number
  ): void {
    const chain = this.chains.get(chainId);
    if (chain) {
      chain.currentBlock = currentBlock;
      chain.targetBlock = targetBlock;
      chain.isRealtime = isRealtime;
      chain.rpcRate = rpcRate;
    }
  }

  /**
   * Record event processing
   */
  recordEvent(chainId: number, eventName: string, durationMs: number): void {
    const chain = this.chains.get(chainId);
    if (chain) {
      if (!chain.events[eventName]) {
        chain.events[eventName] = { count: 0, durationMs: 0 };
      }
      chain.events[eventName]!.count++;
      chain.events[eventName]!.durationMs += durationMs;
    }
  }

  /**
   * Start displaying telemetry
   */
  start(): void {
    if (this.displayInterval) return;

    this.displayInterval = setInterval(() => {
      this.display();
    }, this.DISPLAY_REFRESH_MS);
  }

  /**
   * Stop displaying telemetry
   */
  stop(): void {
    if (this.displayInterval) {
      clearInterval(this.displayInterval);
      this.displayInterval = null;
    }
  }

  /**
   * Display formatted telemetry
   */
  private display(): void {
    // Clear console and move cursor to top
    console.clear();

    console.log("\n┌─────────────────────────────────────────────────────────────┐");
    console.log("│                      Ponder Sync Status                     │");
    console.log("└─────────────────────────────────────────────────────────────┘\n");

    // Sync Status Table
    console.log("Sync\n");
    console.log(
      `| ${"Network".padEnd(20)} | ${"Status".padEnd(10)} | ${"Block".padEnd(15)} | ${"Progress".padEnd(12)} | ${"RPC (req/s)".padEnd(10)} |`
    );
    console.log(
      `|-${"-".repeat(20)}-|-${"-".repeat(10)}-|-${"-".repeat(15)}-|-${"-".repeat(12)}-|-${"-".repeat(10)}-|`
    );

    for (const chain of Array.from(this.chains.values())) {
      // Skip chains with no activity
      if (chain.currentBlock === 0 && Object.keys(chain.events).length === 0) {
        continue;
      }

      const status = chain.isRealtime ? "realtime" : "syncing";
      const blockInfo = chain.isRealtime
        ? chain.currentBlock.toString()
        : `${chain.currentBlock}/${chain.targetBlock}`;
      const progress = chain.isRealtime
        ? "100%"
        : chain.targetBlock > 0
          ? `${((chain.currentBlock / chain.targetBlock) * 100).toFixed(1)}%`
          : "0%";

      console.log(
        `| ${chain.name.padEnd(20)} | ${status.padEnd(10)} | ${blockInfo.padEnd(15)} | ${progress.padEnd(12)} | ${chain.rpcRate.toFixed(1).padEnd(10)} |`
      );
    }

    console.log("\n");

    // Indexing Status (grouped by chain)
    console.log("Indexing (by Chain)\n");

    for (const chain of Array.from(this.chains.values())) {
      // Skip chains with no events
      if (Object.keys(chain.events).length === 0) continue;

      console.log(`\n${chain.name} (Chain ${chain.chainId}):`);
      console.log(
        `| ${"Event".padEnd(40)} | ${"Count".padEnd(10)} | ${"Avg Duration (ms)".padEnd(18)} |`
      );
      console.log(
        `|-${"-".repeat(40)}-|-${"-".repeat(10)}-|-${"-".repeat(18)}-|`
      );

      // Sort events by count (descending)
      const sortedEvents = Object.entries(chain.events).sort(
        ([, a], [, b]) => (b as EventStats).count - (a as EventStats).count
      );

      for (const [eventName, stats] of sortedEvents) {
        const eventStats = stats as EventStats;
        const avgDuration =
          eventStats.count > 0
            ? (eventStats.durationMs / eventStats.count).toFixed(3)
            : "0.000";
        console.log(
          `| ${eventName.padEnd(40)} | ${eventStats.count.toString().padEnd(10)} | ${avgDuration.padEnd(18)} |`
        );
      }
    }

    console.log("\n");

    // Footer
    const totalEvents = Array.from(this.chains.values()).reduce(
      (sum, chain) =>
        sum +
        Object.values(chain.events).reduce((s, e) => s + e.count, 0),
      0
    );

    console.log(`Total Events Indexed: ${totalEvents}`);
    console.log(`Refresh Rate: ${this.DISPLAY_REFRESH_MS / 1000}s`);
    console.log("\nPress Ctrl+C to stop\n");
  }

  /**
   * Get chain statistics (for debugging)
   */
  getChainStats(chainId: number): ChainStats | undefined {
    return this.chains.get(chainId);
  }

  /**
   * Get all statistics (for debugging)
   */
  getAllStats(): ChainStats[] {
    return Array.from(this.chains.values());
  }
}

// Singleton instance
export const telemetry = new PonderTelemetry();

// Auto-start telemetry if enabled
if (process.env["PONDER_TELEMETRY_ENABLED"] === "true") {
  telemetry.start();

  // Graceful shutdown
  process.on("SIGINT", () => {
    telemetry.stop();
    process.exit(0);
  });

  process.on("SIGTERM", () => {
    telemetry.stop();
    process.exit(0);
  });
}
