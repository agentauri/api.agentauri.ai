/**
 * Database Module for Ponder
 *
 * Provides additional database resilience features on top of Ponder's
 * built-in database handling:
 *
 * - Health monitoring with latency tracking
 * - Dead Letter Queue for failed events
 *
 * These features use a separate connection pool from Ponder's internal
 * connection to avoid interference.
 */

export {
  DatabaseHealthMonitor,
  createDatabaseHealthMonitor,
  getDatabaseHealthMonitor,
  stopDatabaseHealthMonitor,
  type DatabaseHealthConfig,
  type HealthCheckResult,
  type DatabaseHealthStats,
} from "./health-monitor";

export {
  DeadLetterQueue,
  createDeadLetterQueue,
  getDeadLetterQueue,
  shutdownDeadLetterQueue,
  enqueueFailedEvent,
  type DLQConfig,
  type DLQEvent,
  type DLQStats,
  type EventProcessor,
} from "./dead-letter-queue";
