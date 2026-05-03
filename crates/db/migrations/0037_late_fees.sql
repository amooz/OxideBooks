CREATE TABLE late_fee_rules (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    grace_days      INT         NOT NULL DEFAULT 0,
    fee_type        TEXT        NOT NULL DEFAULT 'percent', -- 'flat' | 'percent'
    -- flat: minor units; percent: integer hundredths of a percent (150 = 1.5%)
    amount          BIGINT      NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(organization_id)
);

CREATE TABLE late_fees (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    invoice_id      UUID        NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    organization_id UUID        NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    amount          BIGINT      NOT NULL,
    applied_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX ON late_fees(invoice_id);
