-- Configurable approval workflow rules
CREATE TABLE approval_rules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    entity_type     TEXT NOT NULL CHECK (entity_type IN ('expense', 'bill', 'purchase_order', 'purchase_requisition')),
    name            TEXT NOT NULL,
    -- Threshold amount in minor units; NULL means applies to all amounts
    min_amount      BIGINT,
    max_amount      BIGINT,
    -- Required role to approve: 'accountant' | 'admin' | 'owner'
    required_role   TEXT NOT NULL DEFAULT 'accountant'
                    CHECK (required_role IN ('accountant', 'admin', 'owner')),
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order      INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_approval_rules_org  ON approval_rules(organization_id, entity_type);
