CREATE TABLE prepayments (
    id               UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id  UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    contact_id       UUID        NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    amount           BIGINT      NOT NULL CHECK (amount > 0),
    reference        TEXT,
    date             DATE        NOT NULL,
    applied_amount   BIGINT      NOT NULL DEFAULT 0 CHECK (applied_amount >= 0),
    status           TEXT        NOT NULL DEFAULT 'available'
                                 CHECK (status IN ('available', 'partially_applied', 'fully_applied', 'voided')),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_prepayments_org     ON prepayments(organization_id);
CREATE INDEX idx_prepayments_contact ON prepayments(organization_id, contact_id);
