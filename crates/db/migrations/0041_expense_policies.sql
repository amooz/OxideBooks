CREATE TABLE expense_policies (
    id                    UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id       UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    category              TEXT        NOT NULL,
    max_amount            BIGINT      NOT NULL,
    requires_receipt_above BIGINT     NOT NULL DEFAULT 0,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(organization_id, category)
);
