-- Migration: Create Subscriptions Table
-- Description: Stripe subscription management for organizations
-- Created: 2025-11-27

-- Table: subscriptions
-- Purpose: Track Stripe subscription state for organizations
CREATE TABLE subscriptions (
    id TEXT PRIMARY KEY DEFAULT gen_random_uuid()::TEXT,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    stripe_subscription_id TEXT UNIQUE,
    stripe_customer_id TEXT,
    plan TEXT NOT NULL DEFAULT 'free' CHECK (plan IN (
        'free',        -- No subscription, limited access
        'starter',     -- Basic paid tier
        'pro',         -- Professional tier
        'enterprise'   -- Enterprise tier with custom limits
    )),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN (
        'active',      -- Subscription is active
        'canceled',    -- User canceled, still valid until period end
        'past_due',    -- Payment failed
        'trialing',    -- In trial period
        'incomplete',  -- Awaiting initial payment
        'paused'       -- Temporarily paused
    )),
    current_period_start TIMESTAMPTZ,
    current_period_end TIMESTAMPTZ,
    canceled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    CONSTRAINT subscriptions_organization_unique UNIQUE (organization_id)
);

-- Indexes
CREATE INDEX idx_subscriptions_organization_id ON subscriptions(organization_id);
CREATE INDEX idx_subscriptions_status_active ON subscriptions(status) WHERE status = 'active';
CREATE INDEX idx_subscriptions_stripe_customer ON subscriptions(stripe_customer_id) WHERE stripe_customer_id IS NOT NULL;

-- Trigger for automatic updated_at maintenance
CREATE TRIGGER update_subscriptions_updated_at
    BEFORE UPDATE ON subscriptions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Comment on table
COMMENT ON TABLE subscriptions IS 'Organization subscription state for Stripe billing';
