-- Subscription billing: product-based recurring charges with automatic invoicing
CREATE TABLE subscription_plans (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    description     TEXT,
    price           BIGINT NOT NULL CHECK (price >= 0),
    currency        TEXT NOT NULL DEFAULT 'USD',
    billing_cycle   TEXT NOT NULL DEFAULT 'monthly'
                    CHECK (billing_cycle IN ('weekly','monthly','quarterly','annually')),
    trial_days      INT NOT NULL DEFAULT 0,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (organization_id, name)
);

CREATE TABLE subscriptions (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id     UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    plan_id             UUID NOT NULL REFERENCES subscription_plans(id) ON DELETE RESTRICT,
    contact_id          UUID NOT NULL REFERENCES contacts(id) ON DELETE RESTRICT,
    status              TEXT NOT NULL DEFAULT 'trialing'
                        CHECK (status IN ('trialing','active','past_due','cancelled','expired')),
    quantity            INT NOT NULL DEFAULT 1 CHECK (quantity > 0),
    current_period_start DATE NOT NULL,
    current_period_end   DATE NOT NULL,
    trial_end            DATE,
    cancel_at_period_end BOOLEAN NOT NULL DEFAULT FALSE,
    cancelled_at         TIMESTAMPTZ,
    notes               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Invoices generated from this subscription
ALTER TABLE invoices ADD COLUMN subscription_id UUID REFERENCES subscriptions(id) ON DELETE SET NULL;

CREATE INDEX idx_sub_plans_org ON subscription_plans(organization_id);
CREATE INDEX idx_subs_org      ON subscriptions(organization_id);
CREATE INDEX idx_subs_contact  ON subscriptions(contact_id);
CREATE INDEX idx_subs_plan     ON subscriptions(plan_id);
CREATE INDEX idx_invoices_sub  ON invoices(subscription_id) WHERE subscription_id IS NOT NULL;
