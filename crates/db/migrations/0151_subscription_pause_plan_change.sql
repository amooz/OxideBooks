-- Sprint 90: subscription pause/resume and plan-change support

-- Extend the status check to allow 'paused'
ALTER TABLE subscriptions DROP CONSTRAINT IF EXISTS subscriptions_status_check;
ALTER TABLE subscriptions ADD CONSTRAINT subscriptions_status_check
    CHECK (status IN ('trialing','active','past_due','cancelled','expired','paused'));

-- Track when a subscription was paused
ALTER TABLE subscriptions ADD COLUMN IF NOT EXISTS paused_at TIMESTAMPTZ;

-- Log every plan change for audit and proration calculations
CREATE TABLE IF NOT EXISTS subscription_plan_changes (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id  UUID NOT NULL REFERENCES subscriptions(id) ON DELETE CASCADE,
    organization_id  UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    old_plan_id      UUID NOT NULL REFERENCES subscription_plans(id),
    new_plan_id      UUID NOT NULL REFERENCES subscription_plans(id),
    old_price        BIGINT NOT NULL,
    new_price        BIGINT NOT NULL,
    proration_credit BIGINT NOT NULL DEFAULT 0,
    changed_by       UUID REFERENCES users(id) ON DELETE SET NULL,
    changed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sub_plan_changes_sub
    ON subscription_plan_changes(subscription_id, changed_at DESC);
CREATE INDEX IF NOT EXISTS idx_sub_plan_changes_org
    ON subscription_plan_changes(organization_id, changed_at DESC);
