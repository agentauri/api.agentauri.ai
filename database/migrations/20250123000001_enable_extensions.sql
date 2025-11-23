-- Migration 1: Enable Required PostgreSQL Extensions
-- Description: Enables TimescaleDB for time-series optimization and pgcrypto for UUID generation
-- Created: 2025-01-23

-- Enable TimescaleDB extension for time-series data management
-- This must be done before creating any hypertables
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Enable pgcrypto extension for UUID generation
-- Used for generating random UUIDs in id columns
CREATE EXTENSION IF NOT EXISTS pgcrypto;
