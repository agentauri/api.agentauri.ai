-- Redis Rate Limiter - Sliding Window with Minute Granularity
--
-- This Lua script implements a sliding window rate limiter with 1-minute buckets.
-- It performs atomic check-and-increment operations to prevent race conditions.
--
-- Algorithm:
-- 1. Calculate current minute bucket (timestamp rounded to nearest minute)
-- 2. Sum all requests in the last 60 minutes (sliding window)
-- 3. Check if (current_usage + cost) <= limit
-- 4. If allowed: Increment current bucket, set TTL, return success
-- 5. If rejected: Return error with retry_after
--
-- Arguments:
--   KEYS[1]: Base key prefix (e.g., "rl:org:org_123")
--   ARGV[1]: Limit (max requests per window)
--   ARGV[2]: Window size in seconds (default: 3600)
--   ARGV[3]: Cost multiplier (1, 2, 5, or 10 for tiers 0-3)
--   ARGV[4]: Current timestamp (Unix epoch in seconds)
--
-- Returns:
--   [allowed, current_usage, limit, reset_at]
--   - allowed: 1 if request allowed, 0 if rejected
--   - current_usage: Total requests in current window
--   - limit: The configured limit
--   - reset_at: Unix timestamp when oldest bucket expires

local base_key = KEYS[1]
local limit = tonumber(ARGV[1])
local window_seconds = tonumber(ARGV[2])
local cost = tonumber(ARGV[3])
local current_time = tonumber(ARGV[4])

-- Calculate minute boundaries
local MINUTE_SECONDS = 60
local BUCKETS_PER_HOUR = window_seconds / MINUTE_SECONDS  -- 60 buckets

-- Current minute bucket (rounded down to nearest minute)
local current_minute = math.floor(current_time / MINUTE_SECONDS) * MINUTE_SECONDS

-- Calculate how many minutes back we need to check (sliding window)
local minutes_to_check = BUCKETS_PER_HOUR

-- Sum requests across all buckets in the window
local current_usage = 0
for i = 0, (minutes_to_check - 1) do
    local bucket_time = current_minute - (i * MINUTE_SECONDS)
    local bucket_key = base_key .. ":" .. bucket_time
    local bucket_count = redis.call('GET', bucket_key)

    if bucket_count then
        current_usage = current_usage + tonumber(bucket_count)
    end
end

-- Check if adding the cost would exceed the limit
if (current_usage + cost) > limit then
    -- REJECTED: Rate limit exceeded
    -- Calculate reset time (when the oldest bucket expires)
    local oldest_bucket_time = current_minute - ((minutes_to_check - 1) * MINUTE_SECONDS)
    local reset_at = oldest_bucket_time + window_seconds

    return {0, current_usage, limit, reset_at}
end

-- ALLOWED: Increment the current bucket
local current_bucket_key = base_key .. ":" .. current_minute

-- Increment by cost (default is 1 for Tier 0 queries)
local new_count = redis.call('INCRBY', current_bucket_key, cost)

-- Set TTL on the bucket (window + 1 minute buffer to ensure coverage)
-- Only set TTL if this is a new key (INCRBY returns the cost value)
if new_count == cost then
    redis.call('EXPIRE', current_bucket_key, window_seconds + MINUTE_SECONDS)
end

-- Return success
-- New current_usage includes the cost we just added
local new_usage = current_usage + cost
local reset_at = current_minute + window_seconds

return {1, new_usage, limit, reset_at}
