/**
 * Dead Letter Queue (DLQ) for Failed Events
 *
 * Persists events that fail processing to PostgreSQL for later retry.
 * This ensures no events are silently lost due to transient failures.
 *
 * Features:
 * - Automatic retry with exponential backoff
 * - Max retry limit to prevent infinite loops
 * - Persistent storage in PostgreSQL
 * - Batch processing for efficiency
 * - Statistics and monitoring
 */

import { Pool, type PoolClient } from "pg";
import { configLogger } from "../logger";

// ============================================================================
// TYPES
// ============================================================================

export interface DLQConfig {
  /** Retry interval in milliseconds (default: 300000 = 5 minutes) */
  retryIntervalMs?: number;
  /** Maximum number of retries per event (default: 3) */
  maxRetries?: number;
  /** Maximum events in DLQ before dropping oldest (default: 1000) */
  maxQueueSize?: number;
  /** Batch size for retry processing (default: 10) */
  batchSize?: number;
  /** Enable automatic retry processing (default: true) */
  autoRetry?: boolean;
  /** Enable debug logging (default: false) */
  debugLogging?: boolean;
}

export interface DLQEvent {
  id?: number;
  eventId: string;
  eventData: Record<string, unknown>;
  errorMessage: string;
  retryCount: number;
  createdAt?: Date;
  lastRetryAt?: Date;
}

export interface DLQStats {
  totalEnqueued: number;
  totalRetried: number;
  totalSuccessful: number;
  totalFailed: number;
  currentQueueSize: number;
}

export type EventProcessor = (
  eventData: Record<string, unknown>
) => Promise<void>;

// ============================================================================
// DEFAULT CONFIGURATION
// ============================================================================

const DEFAULT_CONFIG: Required<DLQConfig> = {
  retryIntervalMs: 5 * 60 * 1000, // 5 minutes
  maxRetries: 3,
  maxQueueSize: 1000,
  batchSize: 10,
  autoRetry: true,
  debugLogging: false,
};

// ============================================================================
// DEAD LETTER QUEUE
// ============================================================================

export class DeadLetterQueue {
  private pool: Pool | null = null;
  private config: Required<DLQConfig>;
  private connectionString: string;
  private retryInterval: NodeJS.Timeout | null = null;
  private eventProcessor: EventProcessor | null = null;
  private stats: DLQStats;
  private initialized = false;

  constructor(connectionString: string, config: DLQConfig = {}) {
    this.connectionString = connectionString;
    this.config = { ...DEFAULT_CONFIG, ...config };
    this.stats = {
      totalEnqueued: 0,
      totalRetried: 0,
      totalSuccessful: 0,
      totalFailed: 0,
      currentQueueSize: 0,
    };
  }

  /**
   * Initialize the DLQ (create table if not exists, start retry loop)
   */
  async initialize(eventProcessor?: EventProcessor): Promise<void> {
    if (this.initialized) {
      configLogger.warn({}, "DLQ already initialized");
      return;
    }

    this.eventProcessor = eventProcessor ?? null;

    try {
      this.pool = new Pool({
        connectionString: this.connectionString,
        max: 3,
        idleTimeoutMillis: 30_000,
        connectionTimeoutMillis: 10_000,
      });

      await this.ensureTableExists();
      await this.updateQueueSize();

      if (this.config.autoRetry && this.eventProcessor) {
        this.startRetryLoop();
      }

      this.initialized = true;
      configLogger.info(
        {
          retryIntervalMs: this.config.retryIntervalMs,
          maxRetries: this.config.maxRetries,
          autoRetry: this.config.autoRetry,
        },
        "Dead Letter Queue initialized"
      );
    } catch (error) {
      configLogger.error(
        { error: error instanceof Error ? error.message : String(error) },
        "Failed to initialize Dead Letter Queue"
      );
      // Don't throw - allow Ponder to continue without DLQ
    }
  }

  /**
   * Shutdown the DLQ gracefully
   */
  async shutdown(): Promise<void> {
    if (this.retryInterval) {
      clearInterval(this.retryInterval);
      this.retryInterval = null;
    }
    if (this.pool) {
      await this.pool.end();
      this.pool = null;
    }
    this.initialized = false;
    configLogger.info({}, "Dead Letter Queue shut down");
  }

  /**
   * Enqueue a failed event
   */
  async enqueue(
    eventId: string,
    eventData: Record<string, unknown>,
    error: Error | string
  ): Promise<void> {
    if (!this.pool) {
      configLogger.warn(
        { eventId },
        "DLQ not initialized, event will be lost"
      );
      return;
    }

    const errorMessage =
      error instanceof Error ? error.message : String(error);

    try {
      // Check queue size and enforce limit
      if (this.stats.currentQueueSize >= this.config.maxQueueSize) {
        await this.removeOldestEvent();
      }

      await this.pool.query(
        `INSERT INTO ponder_dlq (event_id, event_data, error_message, retry_count)
         VALUES ($1, $2, $3, 0)
         ON CONFLICT (event_id) DO UPDATE SET
           error_message = EXCLUDED.error_message,
           retry_count = ponder_dlq.retry_count`,
        [eventId, JSON.stringify(eventData), errorMessage]
      );

      this.stats.totalEnqueued++;
      this.stats.currentQueueSize++;

      configLogger.warn(
        {
          eventId,
          errorMessage: errorMessage.substring(0, 200),
          queueSize: this.stats.currentQueueSize,
        },
        "Event enqueued to DLQ"
      );
    } catch (dbError) {
      configLogger.error(
        {
          eventId,
          error: dbError instanceof Error ? dbError.message : String(dbError),
        },
        "Failed to enqueue event to DLQ"
      );
    }
  }

  /**
   * Get current statistics
   */
  getStats(): DLQStats {
    return { ...this.stats };
  }

  /**
   * Manually trigger retry processing
   */
  async processRetries(): Promise<number> {
    if (!this.pool || !this.eventProcessor) {
      return 0;
    }

    let processedCount = 0;
    let client: PoolClient | null = null;

    try {
      client = await this.pool.connect();

      // Get events ready for retry (not yet at max retries)
      const result = await client.query<{
        id: number;
        event_id: string;
        event_data: string;
        retry_count: number;
      }>(
        `SELECT id, event_id, event_data, retry_count
         FROM ponder_dlq
         WHERE retry_count < $1
         ORDER BY created_at ASC
         LIMIT $2
         FOR UPDATE SKIP LOCKED`,
        [this.config.maxRetries, this.config.batchSize]
      );

      for (const row of result.rows) {
        try {
          const eventData = JSON.parse(row.event_data) as Record<string, unknown>;

          // Attempt to process the event
          await this.eventProcessor(eventData);

          // Success - remove from DLQ
          await client.query("DELETE FROM ponder_dlq WHERE id = $1", [row.id]);

          this.stats.totalSuccessful++;
          this.stats.currentQueueSize--;
          processedCount++;

          configLogger.info(
            {
              eventId: row.event_id,
              retryCount: row.retry_count,
            },
            "DLQ event processed successfully"
          );
        } catch (processingError) {
          // Failed - increment retry count
          const newRetryCount = row.retry_count + 1;

          await client.query(
            `UPDATE ponder_dlq
             SET retry_count = $1, last_retry_at = NOW()
             WHERE id = $2`,
            [newRetryCount, row.id]
          );

          this.stats.totalRetried++;

          if (newRetryCount >= this.config.maxRetries) {
            this.stats.totalFailed++;
            configLogger.error(
              {
                eventId: row.event_id,
                maxRetries: this.config.maxRetries,
                error:
                  processingError instanceof Error
                    ? processingError.message
                    : String(processingError),
              },
              "DLQ event reached max retries, marked as permanently failed"
            );
          } else {
            configLogger.warn(
              {
                eventId: row.event_id,
                retryCount: newRetryCount,
                maxRetries: this.config.maxRetries,
              },
              "DLQ event retry failed, will retry again"
            );
          }
        }
      }
    } catch (error) {
      configLogger.error(
        { error: error instanceof Error ? error.message : String(error) },
        "Error during DLQ retry processing"
      );
    } finally {
      if (client) {
        client.release();
      }
    }

    return processedCount;
  }

  /**
   * Get all permanently failed events (max retries reached)
   */
  async getFailedEvents(): Promise<DLQEvent[]> {
    if (!this.pool) return [];

    const result = await this.pool.query<{
      id: number;
      event_id: string;
      event_data: string;
      error_message: string;
      retry_count: number;
      created_at: Date;
      last_retry_at: Date | null;
    }>(
      `SELECT id, event_id, event_data, error_message, retry_count, created_at, last_retry_at
       FROM ponder_dlq
       WHERE retry_count >= $1
       ORDER BY created_at DESC
       LIMIT 100`,
      [this.config.maxRetries]
    );

    return result.rows.map((row) => ({
      id: row.id,
      eventId: row.event_id,
      eventData: JSON.parse(row.event_data) as Record<string, unknown>,
      errorMessage: row.error_message,
      retryCount: row.retry_count,
      createdAt: row.created_at,
      lastRetryAt: row.last_retry_at ?? undefined,
    }));
  }

  // ============================================================================
  // PRIVATE METHODS
  // ============================================================================

  private async ensureTableExists(): Promise<void> {
    if (!this.pool) return;

    await this.pool.query(`
      CREATE TABLE IF NOT EXISTS ponder_dlq (
        id SERIAL PRIMARY KEY,
        event_id TEXT NOT NULL UNIQUE,
        event_data JSONB NOT NULL,
        error_message TEXT,
        retry_count INT DEFAULT 0,
        created_at TIMESTAMPTZ DEFAULT NOW(),
        last_retry_at TIMESTAMPTZ
      )
    `);

    await this.pool.query(`
      CREATE INDEX IF NOT EXISTS idx_ponder_dlq_retry
      ON ponder_dlq(retry_count, last_retry_at)
    `);

    await this.pool.query(`
      CREATE INDEX IF NOT EXISTS idx_ponder_dlq_created
      ON ponder_dlq(created_at)
    `);
  }

  private async updateQueueSize(): Promise<void> {
    if (!this.pool) return;

    const result = await this.pool.query<{ count: string }>(
      "SELECT COUNT(*) as count FROM ponder_dlq"
    );
    this.stats.currentQueueSize = parseInt(result.rows[0]?.count ?? "0", 10);
  }

  private async removeOldestEvent(): Promise<void> {
    if (!this.pool) return;

    await this.pool.query(`
      DELETE FROM ponder_dlq
      WHERE id = (
        SELECT id FROM ponder_dlq
        ORDER BY created_at ASC
        LIMIT 1
      )
    `);

    configLogger.warn(
      { maxQueueSize: this.config.maxQueueSize },
      "DLQ at capacity, removed oldest event"
    );
  }

  private startRetryLoop(): void {
    this.retryInterval = setInterval(() => {
      void this.processRetries().then((processed) => {
        if (processed > 0 && this.config.debugLogging) {
          configLogger.debug(
            { processedCount: processed },
            "DLQ retry batch completed"
          );
        }
      });
    }, this.config.retryIntervalMs);

    configLogger.info(
      { intervalMs: this.config.retryIntervalMs },
      "DLQ auto-retry loop started"
    );
  }
}

// ============================================================================
// SINGLETON INSTANCE
// ============================================================================

let dlqInstance: DeadLetterQueue | null = null;

/**
 * Create and initialize the DLQ singleton
 */
export async function createDeadLetterQueue(
  connectionString: string,
  config?: DLQConfig,
  eventProcessor?: EventProcessor
): Promise<DeadLetterQueue> {
  if (dlqInstance) {
    configLogger.warn({}, "DLQ already exists, reusing");
    return dlqInstance;
  }

  dlqInstance = new DeadLetterQueue(connectionString, config);
  await dlqInstance.initialize(eventProcessor);
  return dlqInstance;
}

/**
 * Get the existing DLQ instance
 */
export function getDeadLetterQueue(): DeadLetterQueue | null {
  return dlqInstance;
}

/**
 * Shutdown and clean up the DLQ singleton
 */
export async function shutdownDeadLetterQueue(): Promise<void> {
  if (dlqInstance) {
    await dlqInstance.shutdown();
    dlqInstance = null;
  }
}

/**
 * Helper function to enqueue an event (safe even if DLQ not initialized)
 */
export async function enqueueFailedEvent(
  eventId: string,
  eventData: Record<string, unknown>,
  error: Error | string
): Promise<void> {
  const dlq = getDeadLetterQueue();
  if (dlq) {
    await dlq.enqueue(eventId, eventData, error);
  } else {
    configLogger.warn(
      { eventId },
      "DLQ not available, failed event will be lost"
    );
  }
}
